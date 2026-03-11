use std::{
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use walkdir::WalkDir;

use super::provider::{podcast, radio, ytdlp};
use super::{InputRef, ProviderKind, TrackMeta};

static NEXT_TRACK_ID: AtomicU64 = AtomicU64::new(1);

/// Classify raw CLI inputs into typed `InputRef` variants.
pub fn parse_inputs(raw_inputs: &[String]) -> Vec<InputRef> {
    raw_inputs
        .iter()
        .map(|input| classify_input(input))
        .collect()
}

fn classify_input(input: &str) -> InputRef {
    if !input.starts_with("http://") && !input.starts_with("https://") {
        // Local path — could also be a local .m3u/.pls
        let lower = input.to_ascii_lowercase();
        if lower.ends_with(".m3u") || lower.ends_with(".pls") {
            return InputRef::Radio(input.to_string());
        }
        return InputRef::Path(input.to_string());
    }

    let lower = input.to_ascii_lowercase();

    // Podcast RSS feeds
    if lower.ends_with(".xml")
        || lower.ends_with(".rss")
        || lower.ends_with("/feed")
        || lower.contains("/feed.")
        || lower.contains("rss")
        || lower.contains("feed.xml")
    {
        return InputRef::Podcast(input.to_string());
    }

    // Playlist files (radio)
    if lower.ends_with(".m3u") || lower.ends_with(".m3u8") || lower.ends_with(".pls") {
        return InputRef::Radio(input.to_string());
    }

    // yt-dlp compatible sites
    if lower.contains("youtube.com")
        || lower.contains("youtu.be")
        || lower.contains("soundcloud.com")
        || lower.contains("bandcamp.com")
        || lower.contains("vimeo.com")
        || lower.contains("dailymotion.com")
        || lower.contains("bilibili.com")
    {
        return InputRef::YtDlp(input.to_string());
    }

    // Default: treat as direct HTTP stream
    InputRef::Url(input.to_string())
}

/// Build `TrackMeta` entries for all inputs, dispatching to the correct provider.
pub fn build_tracks(inputs: &[InputRef]) -> Vec<TrackMeta> {
    let mut tracks = Vec::new();

    for input in inputs {
        match input {
            InputRef::Url(url) => tracks.push(new_url_track(url)),
            InputRef::Path(path) => {
                let p = Path::new(path);
                if p.is_file() {
                    if is_audio_file(p) {
                        tracks.push(new_file_track(p));
                    }
                    continue;
                }

                if p.is_dir() {
                    tracks.extend(
                        WalkDir::new(p)
                            .into_iter()
                            .filter_map(Result::ok)
                            .map(|entry| entry.into_path())
                            .filter(|path| path.is_file() && is_audio_file(path))
                            .map(|path| new_file_track(&path)),
                    );
                }
            }
            InputRef::Podcast(feed_url) => match podcast::fetch_podcast_tracks(feed_url) {
                Ok(episode_tracks) => tracks.extend(episode_tracks),
                Err(err) => {
                    // In case of error, add a placeholder track showing the error
                    tracks.push(TrackMeta {
                        id: next_track_id(),
                        title: format!("⚠ Podcast error: {err:#}"),
                        artist: "Error".to_string(),
                        duration: None,
                        is_live: false,
                        provider: ProviderKind::Podcast,
                        path_or_url: feed_url.clone(),
                    });
                }
            },
            InputRef::YtDlp(url) => match ytdlp::resolve_ytdlp_tracks(url) {
                Ok(resolved) => tracks.extend(resolved),
                Err(err) => {
                    tracks.push(TrackMeta {
                        id: next_track_id(),
                        title: format!("⚠ yt-dlp error: {err:#}"),
                        artist: "Error".to_string(),
                        duration: None,
                        is_live: false,
                        provider: ProviderKind::YtDlp,
                        path_or_url: url.clone(),
                    });
                }
            },
            InputRef::Radio(url_or_path) => match radio::parse_playlist(url_or_path) {
                Ok(station_tracks) => tracks.extend(station_tracks),
                Err(err) => {
                    tracks.push(TrackMeta {
                        id: next_track_id(),
                        title: format!("⚠ Radio error: {err:#}"),
                        artist: "Error".to_string(),
                        duration: None,
                        is_live: false,
                        provider: ProviderKind::Radio,
                        path_or_url: url_or_path.clone(),
                    });
                }
            },
        }
    }

    // Also load user-configured radio stations if no explicit radio input given
    let has_radio = inputs.iter().any(|i| matches!(i, InputRef::Radio(_)));
    if !has_radio {
        let stations = radio::load_radio_stations();
        if !stations.is_empty() {
            tracks.extend(stations);
        }
    }

    tracks
}

fn new_file_track(path: &Path) -> TrackMeta {
    let title = path
        .file_stem()
        .and_then(|it| it.to_str())
        .unwrap_or("Untitled")
        .to_string();

    TrackMeta {
        id: next_track_id(),
        title,
        artist: "Local".to_string(),
        duration: None,
        is_live: false,
        provider: ProviderKind::Local,
        path_or_url: path.to_string_lossy().to_string(),
    }
}

fn new_url_track(url: &str) -> TrackMeta {
    TrackMeta {
        id: next_track_id(),
        title: url.to_string(),
        artist: "Stream".to_string(),
        duration: None,
        is_live: false,
        provider: ProviderKind::HttpStream,
        path_or_url: url.to_string(),
    }
}

fn next_track_id() -> u64 {
    NEXT_TRACK_ID.fetch_add(1, Ordering::Relaxed)
}

fn is_audio_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|it| it.to_str())
        .map(|it| it.to_ascii_lowercase());

    matches!(
        ext.as_deref(),
        Some("mp3")
            | Some("flac")
            | Some("wav")
            | Some("ogg")
            | Some("m4a")
            | Some("aac")
            | Some("opus")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_local_path() {
        let input = classify_input("/home/user/music/song.mp3");
        assert!(matches!(input, InputRef::Path(_)));
    }

    #[test]
    fn classify_http_stream() {
        let input = classify_input("https://example.com/stream");
        assert!(matches!(input, InputRef::Url(_)));
    }

    #[test]
    fn classify_youtube_url() {
        let input = classify_input("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
        assert!(matches!(input, InputRef::YtDlp(_)));
    }

    #[test]
    fn classify_soundcloud_url() {
        let input = classify_input("https://soundcloud.com/artist/track");
        assert!(matches!(input, InputRef::YtDlp(_)));
    }

    #[test]
    fn classify_podcast_rss() {
        let input = classify_input("https://example.com/podcast/feed.xml");
        assert!(matches!(input, InputRef::Podcast(_)));
    }

    #[test]
    fn classify_m3u_playlist() {
        let input = classify_input("https://radio.example.com/stream.m3u");
        assert!(matches!(input, InputRef::Radio(_)));
    }

    #[test]
    fn classify_pls_playlist() {
        let input = classify_input("https://radio.example.com/stream.pls");
        assert!(matches!(input, InputRef::Radio(_)));
    }

    #[test]
    fn classify_local_playlist() {
        let input = classify_input("/home/user/stations.m3u");
        assert!(matches!(input, InputRef::Radio(_)));
    }

    #[test]
    fn explicit_radio_input_skips_default_station_injection() {
        let tracks = build_tracks(&[InputRef::Radio("/tmp/stations.m3u".into())]);
        assert_eq!(tracks.len(), 1);
        assert!(tracks[0].title.starts_with("⚠ Radio error:"));
    }
}
