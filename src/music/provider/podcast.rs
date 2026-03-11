use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};

use super::super::{ProviderKind, TrackMeta};

static NEXT_ID: AtomicU64 = AtomicU64::new(1_000_000);

fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Parse an RSS/Atom podcast feed and return one `TrackMeta` per episode that
/// has an `<enclosure>` element with an audio URL.
pub fn fetch_podcast_tracks(feed_url: &str) -> Result<Vec<TrackMeta>> {
    let body = reqwest::blocking::get(feed_url)
        .with_context(|| format!("failed to fetch podcast feed: {feed_url}"))?
        .text()
        .context("failed to read podcast response body")?;

    parse_rss(&body)
}

fn parse_rss(xml: &str) -> Result<Vec<TrackMeta>> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut tracks: Vec<TrackMeta> = Vec::new();
    let mut channel_title = String::new();

    // Parsing state
    let mut in_channel = false;
    let mut in_item = false;
    let mut current_tag = String::new();
    let mut item_title = String::new();
    let mut item_url = String::new();

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name_bytes = e.name().0.to_vec();
                let local = local_name(&name_bytes);
                match local {
                    "channel" | "feed" => in_channel = true,
                    "item" | "entry" => {
                        in_item = true;
                        item_title.clear();
                        item_url.clear();
                    }
                    "enclosure" | "link" if in_item => {
                        for attr in e.attributes().flatten() {
                            let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or_default();
                            if (local == "enclosure" && key == "url")
                                || (local == "link" && key == "href")
                            {
                                let val = attr.unescape_value().unwrap_or_default();
                                if looks_like_audio(&val) && item_url.is_empty() {
                                    item_url = val.to_string();
                                }
                            }
                        }
                    }
                    _ => {}
                }
                current_tag = local.to_string();
            }
            Ok(Event::Empty(ref e)) => {
                let name_bytes = e.name().0.to_vec();
                let local = local_name(&name_bytes);
                if (local == "enclosure" || local == "link") && in_item {
                    for attr in e.attributes().flatten() {
                        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or_default();
                        if (local == "enclosure" && key == "url")
                            || (local == "link" && key == "href")
                        {
                            let val = attr.unescape_value().unwrap_or_default();
                            if looks_like_audio(&val) && item_url.is_empty() {
                                item_url = val.to_string();
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if in_item && current_tag == "title" && item_title.is_empty() {
                    item_title = text;
                } else if in_channel
                    && !in_item
                    && current_tag == "title"
                    && channel_title.is_empty()
                {
                    channel_title = text;
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name().0.to_vec();
                let local = local_name(&name_bytes);
                if local == "item" || local == "entry" {
                    if !item_url.is_empty() {
                        tracks.push(TrackMeta {
                            id: next_id(),
                            title: if item_title.is_empty() {
                                "Untitled Episode".to_string()
                            } else {
                                item_title.clone()
                            },
                            artist: if channel_title.is_empty() {
                                "Podcast".to_string()
                            } else {
                                channel_title.clone()
                            },
                            duration: None,
                            is_live: false,
                            provider: ProviderKind::Podcast,
                            path_or_url: item_url.clone(),
                        });
                    }
                    in_item = false;
                }
                if local == "channel" || local == "feed" {
                    in_channel = false;
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(tracks)
}

/// Strip XML namespace prefix to get the local element name.
fn local_name(full: &[u8]) -> &str {
    let s = std::str::from_utf8(full).unwrap_or_default();
    s.rsplit_once(':').map_or(s, |(_, local)| local)
}

fn looks_like_audio(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains(".mp3")
        || lower.contains(".m4a")
        || lower.contains(".ogg")
        || lower.contains(".opus")
        || lower.contains(".wav")
        || lower.contains(".flac")
        || lower.contains(".aac")
        || lower.contains("audio")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>My Tech Podcast</title>
    <item>
      <title>Episode 1: Hello World</title>
      <enclosure url="https://example.com/ep1.mp3" length="12345" type="audio/mpeg"/>
    </item>
    <item>
      <title>Episode 2: Deep Dive</title>
      <enclosure url="https://example.com/ep2.mp3" length="67890" type="audio/mpeg"/>
    </item>
    <item>
      <title>Episode 3 (video only)</title>
      <enclosure url="https://example.com/ep3.mp4" length="99999" type="video/mp4"/>
    </item>
  </channel>
</rss>"#;

    #[test]
    fn parses_rss_feed_episodes() {
        let tracks = parse_rss(SAMPLE_RSS).unwrap();
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].title, "Episode 1: Hello World");
        assert_eq!(tracks[0].artist, "My Tech Podcast");
        assert_eq!(tracks[0].path_or_url, "https://example.com/ep1.mp3");
        assert!(matches!(tracks[0].provider, ProviderKind::Podcast));
        assert_eq!(tracks[1].title, "Episode 2: Deep Dive");
    }

    #[test]
    fn empty_feed_returns_empty_vec() {
        let tracks = parse_rss("<rss><channel></channel></rss>").unwrap();
        assert!(tracks.is_empty());
    }
}
