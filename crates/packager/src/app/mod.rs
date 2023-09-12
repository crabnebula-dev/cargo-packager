use std::path::PathBuf;

use crate::config::Config;

pub fn package(_config: &Config) -> crate::Result<Vec<PathBuf>> {
    log::warn!("`app` format is not implemented yet! skipping...");
    Ok(vec![])
}
