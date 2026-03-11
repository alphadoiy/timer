use std::io::{Read, Seek};

use anyhow::Result;

use super::{ProviderKind, TrackMeta};

pub mod http;
pub mod local;
pub mod podcast;
pub mod radio;
pub mod ytdlp;

pub trait ReadSeek: Read + Seek + Send + Sync {}
impl<T: Read + Seek + Send + Sync> ReadSeek for T {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReaderCapability {
    Seekable,
    Streaming,
}

pub struct AudioSource {
    capability: ReaderCapability,
    reader: Box<dyn ReadSeek>,
}

impl AudioSource {
    pub fn new(capability: ReaderCapability, reader: Box<dyn ReadSeek>) -> Self {
        Self { capability, reader }
    }

    pub fn capability(&self) -> ReaderCapability {
        self.capability
    }

    pub fn is_seekable(&self) -> bool {
        self.capability == ReaderCapability::Seekable
    }

    pub fn into_reader(self) -> Box<dyn ReadSeek> {
        self.reader
    }
}

/// Open a reader for the given track, dispatching on its provider kind.
pub fn open_reader(track: &TrackMeta) -> Result<AudioSource> {
    if track.is_live {
        let reader = http::open_http_stream(&track.path_or_url)?;
        return Ok(AudioSource::new(ReaderCapability::Streaming, reader));
    }

    let reader = match track.provider {
        ProviderKind::Local => local::open_local(&track.path_or_url)?,
        ProviderKind::HttpStream
        | ProviderKind::Podcast
        | ProviderKind::YtDlp
        | ProviderKind::Radio => http::open_http(&track.path_or_url)?,
    };

    Ok(AudioSource::new(ReaderCapability::Seekable, reader))
}
