use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use super::super::{ProviderKind, TrackMeta};

static NEXT_ID: AtomicU64 = AtomicU64::new(2_000_000);

fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Deserialize)]
struct YtDlpEntry {
    title: Option<String>,
    uploader: Option<String>,
    channel: Option<String>,
    duration: Option<f64>,
    url: Option<String>,
    // For playlists, entries contains sub-entries
    entries: Option<Vec<YtDlpEntry>>,
}

/// Check whether `yt-dlp` is available on PATH.
pub fn is_available() -> bool {
    Command::new("yt-dlp")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success())
}

/// Resolve a URL through yt-dlp.  Returns one or more tracks (playlists
/// expand to many).  Falls back gracefully if yt-dlp is not installed.
pub fn resolve_ytdlp_tracks(url: &str) -> Result<Vec<TrackMeta>> {
    if !is_available() {
        bail!(
            "yt-dlp is not installed. Install with: brew install yt-dlp\n\
             Then retry with the same URL."
        );
    }

    let output = Command::new("yt-dlp")
        .args([
            "--dump-json",
            "--flat-playlist",
            "--no-warnings",
            "-f",
            "bestaudio/best",
            url,
        ])
        .output()
        .context("failed to execute yt-dlp")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("yt-dlp failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_ytdlp_output(&stdout)
}

fn parse_ytdlp_output(json_lines: &str) -> Result<Vec<TrackMeta>> {
    let mut tracks = Vec::new();

    for line in json_lines.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let entry: YtDlpEntry = serde_json::from_str(line).ok().unwrap_or(YtDlpEntry {
            title: None,
            uploader: None,
            channel: None,
            duration: None,
            url: None,
            entries: None,
        });

        if let Some(ref entries) = entry.entries {
            for sub in entries {
                if let Some(track) = entry_to_track(sub) {
                    tracks.push(track);
                }
            }
        } else if let Some(track) = entry_to_track(&entry) {
            tracks.push(track);
        }
    }

    Ok(tracks)
}

fn entry_to_track(entry: &YtDlpEntry) -> Option<TrackMeta> {
    let url = entry.url.as_deref()?;
    let title = entry
        .title
        .clone()
        .unwrap_or_else(|| "Untitled".to_string());
    let artist = entry
        .uploader
        .clone()
        .or_else(|| entry.channel.clone())
        .unwrap_or_else(|| "Unknown".to_string());
    let duration = entry
        .duration
        .filter(|d| *d > 0.0)
        .map(|d| std::time::Duration::from_secs_f64(d));

    Some(TrackMeta {
        id: next_id(),
        title,
        artist,
        duration,
        provider: ProviderKind::YtDlp,
        path_or_url: url.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_entry_json() {
        let json = r#"{"title":"Test Song","uploader":"Artist","duration":180.5,"url":"https://example.com/audio.m4a"}"#;
        let tracks = parse_ytdlp_output(json).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].title, "Test Song");
        assert_eq!(tracks[0].artist, "Artist");
        assert_eq!(
            tracks[0].path_or_url,
            "https://example.com/audio.m4a"
        );
        assert!(matches!(tracks[0].provider, ProviderKind::YtDlp));
        assert!(tracks[0].duration.is_some());
        assert_eq!(tracks[0].duration.unwrap().as_secs(), 180);
    }

    #[test]
    fn parses_multiple_json_lines() {
        let json = r#"{"title":"A","uploader":"X","duration":60,"url":"https://a.mp3"}
{"title":"B","channel":"Y","duration":120,"url":"https://b.mp3"}"#;
        let tracks = parse_ytdlp_output(json).unwrap();
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].title, "A");
        assert_eq!(tracks[1].artist, "Y");
    }

    #[test]
    fn skips_entries_without_url() {
        let json = r#"{"title":"No URL","uploader":"X","duration":60}"#;
        let tracks = parse_ytdlp_output(json).unwrap();
        assert!(tracks.is_empty());
    }
}
