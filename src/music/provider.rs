use std::{
    fs::File,
    io::{Cursor, Read, Seek},
};

use anyhow::{Context, Result, bail};

use super::{SourceKind, TrackMeta};

pub trait ReadSeek: Read + Seek + Send + Sync {}
impl<T: Read + Seek + Send + Sync> ReadSeek for T {}

pub fn open_reader(track: &TrackMeta) -> Result<Box<dyn ReadSeek>> {
    match track.source_kind {
        SourceKind::LocalFile => {
            let file = File::open(&track.path_or_url)
                .with_context(|| format!("failed to open file: {}", track.path_or_url))?;
            Ok(Box::new(file))
        }
        SourceKind::HttpStream => {
            let resp = reqwest::blocking::get(&track.path_or_url)
                .with_context(|| format!("failed to GET {}", track.path_or_url))?;
            if !resp.status().is_success() {
                bail!("request failed with status {}", resp.status());
            }
            let bytes = resp.bytes().context("failed to read response body")?;
            Ok(Box::new(Cursor::new(bytes.to_vec())))
        }
    }
}
