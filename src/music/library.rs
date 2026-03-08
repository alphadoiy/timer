use std::{
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use walkdir::WalkDir;

use super::{InputRef, SourceKind, TrackMeta};

static NEXT_TRACK_ID: AtomicU64 = AtomicU64::new(1);

pub fn parse_inputs(raw_inputs: &[String]) -> Vec<InputRef> {
    raw_inputs
        .iter()
        .map(|input| {
            if input.starts_with("http://") || input.starts_with("https://") {
                InputRef::Url(input.clone())
            } else {
                InputRef::Path(input.clone())
            }
        })
        .collect()
}

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
        source_kind: SourceKind::LocalFile,
        path_or_url: path.to_string_lossy().to_string(),
    }
}

fn new_url_track(url: &str) -> TrackMeta {
    TrackMeta {
        id: next_track_id(),
        title: url.to_string(),
        artist: "Stream".to_string(),
        duration: None,
        source_kind: SourceKind::HttpStream,
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
