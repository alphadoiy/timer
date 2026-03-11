use std::fs::File;

use anyhow::{Context, Result};

use super::ReadSeek;

pub fn open_local(path: &str) -> Result<Box<dyn ReadSeek>> {
    let file = File::open(path).with_context(|| format!("failed to open file: {path}"))?;
    Ok(Box::new(file))
}
