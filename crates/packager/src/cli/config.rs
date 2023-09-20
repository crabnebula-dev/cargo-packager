use std::path::{Path, PathBuf};

use crate::{config::Binary, Config};

pub fn parse_config_file<P: AsRef<Path>>(path: P) -> crate::Result<Vec<(Option<PathBuf>, Config)>> {
    let path = path.as_ref().to_path_buf().canonicalize()?;
    let content = std::fs::read_to_string(&path)?;
    let configs = match path.extension().and_then(|e| e.to_str()) {
        Some("toml") => {
            if let Ok(cs) = toml::from_str::<Vec<Config>>(&content) {
                cs.into_iter().map(|c| (Some(path.clone()), c)).collect()
            } else {
                vec![(Some(path), toml::from_str::<Config>(&content)?)]
            }
        }
        _ => {
            if let Ok(cs) = serde_json::from_str::<Vec<Config>>(&content) {
                cs.into_iter().map(|c| (Some(path.clone()), c)).collect()
            } else {
                vec![(Some(path), serde_json::from_str::<Config>(&content)?)]
            }
        }
    };

    Ok(configs)
}

pub fn find_config_files() -> Vec<PathBuf> {
    let opts = glob::MatchOptions {
        case_sensitive: false,
        ..Default::default()
    };

    [
        glob::glob_with("**/packager.toml", opts)
            .unwrap()
            .flatten()
            .collect::<Vec<_>>(),
        glob::glob_with("**/packager.json", opts)
            .unwrap()
            .flatten()
            .collect::<Vec<_>>(),
    ]
    .concat()
}

pub fn load_configs_from_cargo_workspace(
    release: bool,
    profile: Option<String>,
    manifest_path: Option<PathBuf>,
    packages: Option<Vec<String>>,
) -> crate::Result<Vec<(Option<PathBuf>, Config)>> {
    let profile = if release {
        "release"
    } else if let Some(profile) = &profile {
        profile.as_str()
    } else {
        "debug"
    };

    let mut metadata_cmd = cargo_metadata::MetadataCommand::new();
    if let Some(manifest_path) = &manifest_path {
        metadata_cmd.manifest_path(manifest_path);
    }
    let metadata = metadata_cmd.exec()?;

    let mut configs = Vec::new();
    for package in metadata.workspace_packages().iter() {
        // skip if this package was not specified in the explicit packages to build
        if packages
            .as_ref()
            .map(|packages| !packages.contains(&package.name))
            .unwrap_or(false)
        {
            continue;
        }

        if let Some(config) = package.metadata.get("packager") {
            let mut config: Config = serde_json::from_value(config.to_owned())?;
            if config.product_name.is_empty() {
                config.product_name = package.name.clone();
            }
            if config.version.is_empty() {
                config.version = package.version.to_string();
            }
            if config.out_dir.as_os_str().is_empty() {
                config.out_dir = metadata
                    .target_directory
                    .as_std_path()
                    .to_path_buf()
                    .join(profile);
            }
            if config.description.is_none() {
                config.description = package.description.clone();
            }
            if config.authors.is_empty() {
                config.authors = package.authors.clone();
            }
            if config.license_file.is_none() {
                config.license_file = package
                    .license_file
                    .as_ref()
                    .map(|p| p.as_std_path().to_owned());
            }
            let targets = package
                .targets
                .iter()
                .filter(|t| t.is_bin())
                .collect::<Vec<_>>();
            for target in &targets {
                config.binaries.push(Binary {
                    filename: target.name.clone(),
                    main: match targets.len() {
                        1 => true,
                        _ => target.name == package.name,
                    },
                })
            }
            configs.push((
                Some(package.manifest_path.as_std_path().to_path_buf()),
                config,
            ));
        }
    }

    Ok(configs)
}
