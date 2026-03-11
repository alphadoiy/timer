use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::sync::Mutex;

use anyhow::{Context, Result, bail};

use super::ReadSeek;

pub fn open_http(url: &str) -> Result<Box<dyn ReadSeek>> {
    let resp = reqwest::blocking::get(url).with_context(|| format!("failed to GET {url}"))?;
    if !resp.status().is_success() {
        bail!("request failed with status {}", resp.status());
    }
    let bytes = resp.bytes().context("failed to read response body")?;
    Ok(Box::new(Cursor::new(bytes.to_vec())))
}

pub fn open_http_stream(url: &str) -> Result<Box<dyn ReadSeek>> {
    let resp = reqwest::blocking::get(url)
        .with_context(|| format!("failed to open live stream: {url}"))?;
    if !resp.status().is_success() {
        bail!("request failed with status {}", resp.status());
    }

    Ok(Box::new(StreamingCacheReader::new(resp)))
}

struct StreamingCacheReader<R> {
    response: Mutex<R>,
    cache: Vec<u8>,
    cache_start: usize,
    pos: usize,
    eof: bool,
    max_cache_bytes: usize,
    keep_behind_bytes: usize,
}

impl<R: Read> StreamingCacheReader<R> {
    const DEFAULT_MAX_CACHE_BYTES: usize = 256 * 1024;
    const KEEP_BEHIND_BYTES: usize = 64 * 1024;

    fn new(response: R) -> Self {
        Self {
            response: Mutex::new(response),
            cache: Vec::new(),
            cache_start: 0,
            pos: 0,
            eof: false,
            max_cache_bytes: Self::DEFAULT_MAX_CACHE_BYTES,
            keep_behind_bytes: Self::KEEP_BEHIND_BYTES,
        }
    }

    #[cfg(test)]
    fn with_limit(response: R, max_cache_bytes: usize) -> Self {
        Self {
            response: Mutex::new(response),
            cache: Vec::new(),
            cache_start: 0,
            pos: 0,
            eof: false,
            max_cache_bytes: max_cache_bytes.max(1),
            keep_behind_bytes: (max_cache_bytes / 4).max(1),
        }
    }

    fn fill_until(&mut self, target_pos: usize) -> io::Result<()> {
        while !self.eof && self.buffered_end() < target_pos {
            let mut chunk = [0_u8; 8192];
            let read = self
                .response
                .lock()
                .map_err(|_| io::Error::other("live stream mutex poisoned"))?
                .read(&mut chunk)?;
            if read == 0 {
                self.eof = true;
                break;
            }
            self.cache.extend_from_slice(&chunk[..read]);
            self.evict_consumed_prefix();
        }
        Ok(())
    }

    fn current_end(&mut self) -> io::Result<usize> {
        self.fill_until(self.buffered_end().saturating_add(1))?;
        Ok(self.buffered_end())
    }

    fn buffered_end(&self) -> usize {
        self.cache_start.saturating_add(self.cache.len())
    }

    fn evict_consumed_prefix(&mut self) {
        if self.cache.len() <= self.max_cache_bytes {
            return;
        }

        let retain_from = self
            .pos
            .saturating_sub(self.keep_behind_bytes)
            .max(self.cache_start);
        let drop_len = retain_from
            .saturating_sub(self.cache_start)
            .min(self.cache.len());

        if drop_len > 0 {
            self.cache.drain(..drop_len);
            self.cache_start += drop_len;
        }
    }
}

impl<R: Read> Read for StreamingCacheReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        self.fill_until(self.pos.saturating_add(1))?;
        if self.pos < self.cache_start {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot read before buffered live stream window",
            ));
        }

        let start = self.pos - self.cache_start;
        let available = self.cache.len().saturating_sub(start);
        if available == 0 {
            return Ok(0);
        }

        let to_copy = available.min(buf.len());
        let end = start + to_copy;
        buf[..to_copy].copy_from_slice(&self.cache[start..end]);
        self.pos += to_copy;
        self.evict_consumed_prefix();
        Ok(to_copy)
    }
}

impl<R: Read + Send> Seek for StreamingCacheReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let target = match pos {
            SeekFrom::Start(offset) => {
                let offset = offset.try_into().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "seek target exceeds usize")
                })?;
                self.fill_until(offset)?;
                offset as i64
            }
            SeekFrom::Current(offset) => self.pos as i64 + offset,
            SeekFrom::End(offset) => self.current_end()? as i64 + offset,
        };

        if target < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot seek before start of stream",
            ));
        }

        let target = target as usize;
        self.fill_until(target)?;
        if target < self.cache_start {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot seek before buffered live stream window",
            ));
        }
        if target > self.buffered_end() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot seek beyond buffered live stream",
            ));
        }

        self.pos = target;
        Ok(self.pos as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::StreamingCacheReader;
    use std::io::{Cursor, Read, Seek, SeekFrom};

    #[test]
    fn streaming_cache_reader_evicts_consumed_bytes() {
        let payload = vec![b'x'; 256];
        let mut reader = StreamingCacheReader::with_limit(Cursor::new(payload), 128);
        let mut buf = [0_u8; 192];

        let read = reader.read(&mut buf).unwrap();
        assert_eq!(read, 192);
        assert!(reader.cache.len() <= 128);
        assert!(reader.cache_start > 0);
    }

    #[test]
    fn streaming_cache_reader_rejects_seek_before_window() {
        let payload = vec![b'x'; 256];
        let mut reader = StreamingCacheReader::with_limit(Cursor::new(payload), 128);
        let mut buf = [0_u8; 192];

        let _ = reader.read(&mut buf).unwrap();
        let err = reader.seek(SeekFrom::Start(0)).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }
}
