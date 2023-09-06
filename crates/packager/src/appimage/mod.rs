use std::path::PathBuf;

use crate::config::Config;

pub fn package(_config: &Config) -> crate::Result<Vec<PathBuf>> {
    log::error!("`appimage` format is not implemented yet!");
    std::process::exit(1);
}
