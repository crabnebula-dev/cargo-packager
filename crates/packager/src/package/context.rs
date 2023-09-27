use std::path::PathBuf;

use crate::{config::ConfigExt, util, Config};

/// The packaging context info
#[derive(Debug)]
pub struct Context {
    /// The config for the app we are packaging
    pub config: Config,
    /// The intermediates path, which is `<out-dir>/.cargo-packager`
    pub intermediates_path: PathBuf,
    /// The global path which we store tools used by cargo-packager and usually is
    /// `<cache-dir>/.cargo-packager`
    pub tools_path: PathBuf,
}

impl Context {
    pub fn new(config: &Config) -> crate::Result<Self> {
        let tools_path = dirs::cache_dir()
            .unwrap_or_else(|| config.out_dir())
            .join(".cargo-packager");
        if !tools_path.exists() {
            std::fs::create_dir_all(&tools_path)?;
        }

        let intermediates_path = config.out_dir().join(".cargo-packager");
        util::create_clean_dir(&intermediates_path)?;

        Ok(Self {
            config: config.clone(),
            tools_path,
            intermediates_path,
        })
    }
}
