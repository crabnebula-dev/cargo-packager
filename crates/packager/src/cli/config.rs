// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use crate::{config::Binary, Config};

fn find_nearset_pkg_name(path: &Path) -> crate::Result<Option<String>> {
    fn find_nearset_pkg_name_inner() -> crate::Result<Option<String>> {
        if let Ok(contents) = std::fs::read_to_string("Cargo.toml") {
            let toml = toml::from_str::<toml::Table>(&contents)?;
            if let Some(name) = toml.get("package").and_then(|p| p.get("name")) {
                return Ok(Some(name.to_string()));
            }
        }

        if let Ok(contents) = std::fs::read("package.json") {
            let json = serde_json::from_slice::<serde_json::Value>(&contents)?;
            if let Some(name) = json.get("name") {
                return Ok(Some(name.to_string()));
            }
        }

        Ok(None)
    }

    let cwd = std::env::current_dir()?;
    std::env::set_current_dir(path)?;
    let res = find_nearset_pkg_name_inner();
    std::env::set_current_dir(cwd)?;
    res
}

#[tracing::instrument(level = "trace")]
pub fn parse_config_file<P: AsRef<Path> + Debug>(
    path: P,
) -> crate::Result<Vec<(Option<PathBuf>, Config)>> {
    let path = path.as_ref().to_path_buf().canonicalize()?;
    let content = std::fs::read_to_string(&path)?;
    let mut configs = match path.extension().and_then(|e| e.to_str()) {
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

    for (path, config) in &mut configs {
        // fill config.name if unset
        if config.name.is_none() {
            // and config wasn't passed using `--config` cli arg
            if let Some(path) = &path {
                let name = find_nearset_pkg_name(path)?;
                config.name = name;
            }
        }
    }

    Ok(configs)
}

#[tracing::instrument(level = "trace")]
pub fn find_config_files() -> crate::Result<Vec<PathBuf>> {
    let opts = glob::MatchOptions {
        case_sensitive: false,
        ..Default::default()
    };

    Ok([
        glob::glob_with("**/packager.toml", opts)?
            .flatten()
            .collect::<Vec<_>>(),
        glob::glob_with("**/packager.json", opts)?
            .flatten()
            .collect::<Vec<_>>(),
    ]
    .concat())
}

#[tracing::instrument(level = "trace")]
pub fn load_configs_from_cargo_workspace(
    release: bool,
    profile: Option<String>,
    manifest_path: Option<PathBuf>,
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
    let Ok(metadata) = metadata_cmd.exec() else {
        return Ok(Vec::new());
    };

    let mut configs = Vec::new();
    for package in metadata.workspace_packages().iter() {
        if let Some(config) = package.metadata.get("packager") {
            let mut config: Config = serde_json::from_value(config.to_owned())?;
            if config.name.is_none() {
                config.name.replace(package.name.clone());
            }
            if config.product_name.is_empty() {
                config.product_name = package.name.clone();
            }
            if config.version.is_empty() {
                config.version = package.version.to_string();
            }
            if config.identifier.is_none() {
                let author = package
                    .authors
                    .first()
                    .map(|a| {
                        let a = a.replace(['_', ' ', '.'], "-").to_lowercase();
                        a.strip_suffix('_').map(ToString::to_string).unwrap_or(a)
                    })
                    .unwrap_or_else(|| format!("{}-author", package.name));
                config
                    .identifier
                    .replace(format!("com.{}.{}", author, package.name));
            }

            let cargo_out_dir = metadata
                .target_directory
                .as_std_path()
                .to_path_buf()
                .join(profile);
            if config.binaries_dir.is_none() {
                config.binaries_dir.replace(cargo_out_dir.clone());
            }
            if config.out_dir.as_os_str().is_empty() {
                config.out_dir = cargo_out_dir;
            }

            if config.description.is_none() {
                config.description = package.description.clone();
            }
            if config.authors.is_none() {
                config.authors = Some(package.authors.clone());
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
                    path: target.name.clone().into(),
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
