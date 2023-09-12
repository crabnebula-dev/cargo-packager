use std::path::PathBuf;

use crate::config::Config;

pub fn package(_config: &Config) -> crate::Result<Vec<PathBuf>> {
    log::warn!("`appimage` format is not implemented yet! skipping...");
    Ok(vec![])
}
