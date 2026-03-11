use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};

use super::super::{ProviderKind, TrackMeta};

static NEXT_ID: AtomicU64 = AtomicU64::new(3_000_000);

fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Parse a remote or local M3U / PLS playlist into a list of stream tracks.
pub fn parse_playlist(url_or_path: &str) -> Result<Vec<TrackMeta>> {
    let content = if url_or_path.starts_with("http://") || url_or_path.starts_with("https://") {
        reqwest::blocking::get(url_or_path)
            .with_context(|| format!("failed to fetch playlist: {url_or_path}"))?
            .text()
            .context("failed to read playlist body")?
    } else {
        std::fs::read_to_string(url_or_path)
            .with_context(|| format!("failed to read playlist file: {url_or_path}"))?
    };

    let lower = content.to_ascii_lowercase();
    if lower.contains("[playlist]") {
        Ok(parse_pls(&content))
    } else {
        Ok(parse_m3u(&content))
    }
}

fn parse_m3u(content: &str) -> Vec<TrackMeta> {
    let mut tracks = Vec::new();
    let mut pending_title: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line == "#EXTM3U" {
            continue;
        }
        if let Some(rest) = line.strip_prefix("#EXTINF:") {
            // Format: #EXTINF:<duration>,<title>
            if let Some((_dur, title)) = rest.split_once(',') {
                pending_title = Some(title.trim().to_string());
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        // This is a URL or path line
        let title = pending_title
            .take()
            .unwrap_or_else(|| extract_name_from_url(line));
        tracks.push(TrackMeta {
            id: next_id(),
            title,
            artist: "Radio".to_string(),
            duration: None,
            provider: ProviderKind::Radio,
            path_or_url: line.to_string(),
        });
    }

    tracks
}

fn parse_pls(content: &str) -> Vec<TrackMeta> {
    let mut urls: Vec<(usize, String)> = Vec::new();
    let mut titles: Vec<(usize, String)> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("File") {
            if let Some((num, url)) = rest.split_once('=') {
                if let Ok(n) = num.trim().parse::<usize>() {
                    urls.push((n, url.trim().to_string()));
                }
            }
        } else if let Some(rest) = line.strip_prefix("Title") {
            if let Some((num, title)) = rest.split_once('=') {
                if let Ok(n) = num.trim().parse::<usize>() {
                    titles.push((n, title.trim().to_string()));
                }
            }
        }
    }

    urls.sort_by_key(|(n, _)| *n);
    let mut tracks = Vec::new();
    for (num, url) in &urls {
        let title = titles
            .iter()
            .find(|(n, _)| n == num)
            .map(|(_, t)| t.clone())
            .unwrap_or_else(|| extract_name_from_url(url));
        tracks.push(TrackMeta {
            id: next_id(),
            title,
            artist: "Radio".to_string(),
            duration: None,
            provider: ProviderKind::Radio,
            path_or_url: url.clone(),
        });
    }
    tracks
}

fn extract_name_from_url(url: &str) -> String {
    url.rsplit('/')
        .next()
        .unwrap_or(url)
        .split('?')
        .next()
        .unwrap_or(url)
        .to_string()
}

/// Load custom radio stations from `~/.config/timer/radios.toml`.
pub fn load_radio_stations() -> Vec<TrackMeta> {
    let entries = read_station_entries();
    entries
        .into_iter()
        .map(|entry| TrackMeta {
            id: next_id(),
            title: entry.name,
            artist: "Radio".to_string(),
            duration: None,
            provider: ProviderKind::Radio,
            path_or_url: entry.url,
        })
        .collect()
}

/// List saved station names and URLs (for `:station list`).
pub fn list_station_names() -> Vec<(String, String)> {
    read_station_entries()
        .into_iter()
        .map(|e| (e.name, e.url))
        .collect()
}

/// Add a station to `radios.toml`. Creates the file/directory if needed.
pub fn save_station(name: &str, url: &str) -> Result<()> {
    let path = super::super::config::radios_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config dir: {}", parent.display()))?;
    }

    let mut entries = read_station_entries();

    // Overwrite if name already exists
    if let Some(existing) = entries.iter_mut().find(|e| e.name == name) {
        existing.url = url.to_string();
    } else {
        entries.push(RadioEntry {
            name: name.to_string(),
            url: url.to_string(),
        });
    }

    write_station_entries(&path, &entries)
}

/// Remove a station from `radios.toml` by name. Returns true if found.
pub fn remove_station(name: &str) -> Result<bool> {
    let path = super::super::config::radios_path();
    let mut entries = read_station_entries();
    let before = entries.len();
    entries.retain(|e| e.name != name);
    if entries.len() == before {
        return Ok(false);
    }
    write_station_entries(&path, &entries)?;
    Ok(true)
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct RadioEntry {
    name: String,
    url: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RadioFile {
    #[serde(default)]
    station: Vec<RadioEntry>,
}

fn read_station_entries() -> Vec<RadioEntry> {
    let path = super::super::config::radios_path();
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let file: RadioFile = toml::from_str(&content).unwrap_or(RadioFile {
        station: Vec::new(),
    });
    file.station
}

fn write_station_entries(path: &std::path::Path, entries: &[RadioEntry]) -> Result<()> {
    let file = RadioFile {
        station: entries.to_vec(),
    };
    let content = toml::to_string_pretty(&file).context("failed to serialize radios.toml")?;
    std::fs::write(path, content)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_m3u_basic() {
        let m3u = "#EXTM3U\n#EXTINF:-1,Jazz FM\nhttps://jazz.example.com/stream\n#EXTINF:-1,Lo-fi Beats\nhttps://lofi.example.com/stream\n";
        let tracks = parse_m3u(m3u);
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].title, "Jazz FM");
        assert_eq!(tracks[0].path_or_url, "https://jazz.example.com/stream");
        assert_eq!(tracks[1].title, "Lo-fi Beats");
        assert!(matches!(tracks[0].provider, ProviderKind::Radio));
    }

    #[test]
    fn parse_pls_basic() {
        let pls = "[playlist]\nFile1=https://stream1.example.com\nTitle1=Station One\nFile2=https://stream2.example.com\nTitle2=Station Two\nNumberOfEntries=2\n";
        let tracks = parse_pls(pls);
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].title, "Station One");
        assert_eq!(tracks[1].path_or_url, "https://stream2.example.com");
    }

    #[test]
    fn parse_m3u_without_extinf() {
        let m3u = "https://example.com/stream1\nhttps://example.com/stream2\n";
        let tracks = parse_m3u(m3u);
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].title, "stream1");
    }
}
