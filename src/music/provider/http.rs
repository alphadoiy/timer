use std::io::Cursor;

use anyhow::{Context, Result, bail};

use super::ReadSeek;

pub fn open_http(url: &str) -> Result<Box<dyn ReadSeek>> {
    let resp =
        reqwest::blocking::get(url).with_context(|| format!("failed to GET {url}"))?;
    if !resp.status().is_success() {
        bail!("request failed with status {}", resp.status());
    }
    let bytes = resp.bytes().context("failed to read response body")?;
    Ok(Box::new(Cursor::new(bytes.to_vec())))
}
