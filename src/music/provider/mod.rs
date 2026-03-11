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

/// Open a reader for the given track, dispatching on its provider kind.
pub fn open_reader(track: &TrackMeta) -> Result<Box<dyn ReadSeek>> {
    match track.provider {
        ProviderKind::Local => local::open_local(&track.path_or_url),
        ProviderKind::HttpStream
        | ProviderKind::Podcast
        | ProviderKind::YtDlp
        | ProviderKind::Radio => http::open_http(&track.path_or_url),
    }
}
