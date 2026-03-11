use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};

use super::super::{ProviderKind, TrackMeta};

static NEXT_ID: AtomicU64 = AtomicU64::new(3_000_000);
const CODE_RADIO_NAME: &str = "freeCodeCamp Code Radio";
const CODE_RADIO_URL: &str =
    "https://coderadio-admin-v2.freecodecamp.org/listen/coderadio/radio.mp3";

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
            if let Some((_dur, title)) = rest.split_once(',') {
                pending_title = Some(title.trim().to_string());
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let title = pending_title
            .take()
            .unwrap_or_else(|| extract_name_from_url(line));
        tracks.push(new_station_track(title, line.to_string()));
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
        tracks.push(new_station_track(title, url.clone()));
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

pub fn load_radio_stations() -> Vec<TrackMeta> {
    read_station_entries()
        .into_iter()
        .map(|entry| new_station_track(entry.name, entry.url))
        .collect()
}

pub fn load_radio_stations_with_default() -> Vec<TrackMeta> {
    merged_station_entries(read_station_entries())
        .into_iter()
        .map(|entry| new_station_track(entry.name, entry.url))
        .collect()
}

pub fn list_station_names() -> Vec<(String, String)> {
    read_station_entries()
        .into_iter()
        .map(|e| (e.name, e.url))
        .collect()
}

pub fn save_station(name: &str, url: &str) -> Result<()> {
    let path = super::super::config::radios_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config dir: {}", parent.display()))?;
    }

    let mut entries = read_station_entries();

    if let Some(existing) = entries.iter_mut().find(|e| e.name == name || e.url == url) {
        existing.name = name.to_string();
        existing.url = url.to_string();
    } else {
        entries.push(RadioEntry {
            name: name.to_string(),
            url: url.to_string(),
        });
    }

    write_station_entries(&path, &entries)
}

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

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
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
    parse_station_entries(&content)
}

fn parse_station_entries(content: &str) -> Vec<RadioEntry> {
    let file: RadioFile = toml::from_str(content).unwrap_or(RadioFile {
        station: Vec::new(),
    });
    file.station
}

fn write_station_entries(path: &Path, entries: &[RadioEntry]) -> Result<()> {
    let file = RadioFile {
        station: entries.to_vec(),
    };
    let content = toml::to_string_pretty(&file).context("failed to serialize radios.toml")?;
    std::fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn merged_station_entries(user_entries: Vec<RadioEntry>) -> Vec<RadioEntry> {
    let mut merged = default_station_entries();

    for user in user_entries {
        if let Some(existing) = merged.iter_mut().find(|entry| entry.url == user.url) {
            *existing = user;
            continue;
        }
        if let Some(existing) = merged.iter_mut().find(|entry| entry.name == user.name) {
            *existing = user;
            continue;
        }
        merged.push(user);
    }

    merged
}

fn default_station_entries() -> Vec<RadioEntry> {
    vec![RadioEntry {
        name: CODE_RADIO_NAME.to_string(),
        url: CODE_RADIO_URL.to_string(),
    }]
}

fn new_station_track(title: String, url: String) -> TrackMeta {
    TrackMeta {
        id: next_id(),
        title,
        artist: "Radio".to_string(),
        duration: None,
        is_live: true,
        provider: ProviderKind::Radio,
        path_or_url: url,
    }
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
        assert!(tracks[0].is_live);
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

    #[test]
    fn explicit_radio_load_includes_default_code_radio() {
        let stations = merged_station_entries(Vec::new());
        assert_eq!(stations.len(), 1);
        assert_eq!(stations[0].name, CODE_RADIO_NAME);
        assert_eq!(stations[0].url, CODE_RADIO_URL);
    }

    #[test]
    fn load_radio_stations_appends_user_entries() {
        let merged = merged_station_entries(vec![RadioEntry {
            name: "Jazz FM".into(),
            url: "https://jazz.example.com/live".into(),
        }]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].name, CODE_RADIO_NAME);
        assert_eq!(merged[1].name, "Jazz FM");
    }

    #[test]
    fn user_entry_replaces_default_on_same_url() {
        let merged = merged_station_entries(vec![RadioEntry {
            name: "My Code Radio".into(),
            url: CODE_RADIO_URL.into(),
        }]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "My Code Radio");
    }

    #[test]
    fn saved_station_list_excludes_default_code_radio() {
        let listed = parse_station_entries("")
            .into_iter()
            .map(|entry| (entry.name, entry.url))
            .collect::<Vec<_>>();
        assert!(listed.is_empty());
    }
}
