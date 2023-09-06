use std::path::PathBuf;

use crate::config::Config;

pub fn package(_config: &Config) -> crate::Result<Vec<PathBuf>> {
    // let mut wix_path = dirs::cache_dir().unwrap();
    // wix_path.push("tauri/WixTools");

    // if !wix_path.exists() {
    //     get_and_extract_wix(&wix_path)?;
    // } else if WIX_REQUIRED_FILES
    //     .iter()
    //     .any(|p| !wix_path.join(p).exists())
    // {
    //     warn!("WixTools directory is missing some files. Recreating it.");
    //     std::fs::remove_dir_all(&wix_path)?;
    //     get_and_extract_wix(&wix_path)?;
    // }

    // build_wix_app_installer(settings, &wix_path, updater)
    Ok(vec![])
}
