// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
};

use super::{Error, Result};
use crate::{config::Binary, Config};

impl Config {
    pub(crate) fn name(&self) -> &str {
        self.name.as_deref().unwrap_or_default()
    }

    /// Whether this config should be packaged or skipped
    pub(crate) fn should_pacakge(&self, cli: &super::Cli) -> bool {
        // Should be packaged when it is enabled and this package was in the explicit packages list specified on the CLI,
        // or the packages list specified on the CLI is empty which means build all
        self.enabled
            && cli
                .packages
                .as_ref()
                .map(|packages| packages.iter().any(|p| p == self.name()))
                .unwrap_or(true)
    }
}

fn find_nearset_pkg_name(path: &Path) -> Result<Option<String>> {
    // `path` is the config file, so the package manifests are searched next to
    // it, not in it: using the file as the working directory fails with
    // `ERROR_DIRECTORY` (os error 267) on Windows and `ENOTDIR` elsewhere.
    // A bare file name has an empty parent, which `join` resolves against the
    // current directory.
    let dir = path.parent().unwrap_or(Path::new("."));

    if let Ok(contents) = fs::read_to_string(dir.join("Cargo.toml")) {
        let toml = toml::from_str::<toml::Table>(&contents)
            .map_err(|e| Error::FailedToParseCargoToml(Box::new(e)))?;

        if let Some(name) = toml
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
        {
            return Ok(Some(name.to_string()));
        }
    }

    if let Ok(contents) = fs::read(dir.join("package.json")) {
        let json = serde_json::from_slice::<serde_json::Value>(&contents)
            .map_err(Error::FailedToParsePacakgeJson)?;

        if let Some(name) = json.get("name").and_then(|n| n.as_str()) {
            return Ok(Some(name.to_string()));
        }
    }

    Ok(None)
}

#[tracing::instrument(level = "trace")]
fn parse_config_file<P: AsRef<Path> + Debug>(path: P) -> Result<Vec<(Option<PathBuf>, Config)>> {
    let p = path.as_ref().to_path_buf();
    let path = p.canonicalize().map_err(|e| Error::IoWithPath(p, e))?;
    let content = fs::read_to_string(&path).map_err(|e| Error::IoWithPath(path.clone(), e))?;
    let mut configs = match path.extension().and_then(|e| e.to_str()) {
        Some("toml") => {
            if let Ok(configs) = toml::from_str::<Vec<Config>>(&content) {
                configs
                    .into_iter()
                    .map(|c| (Some(path.clone()), c))
                    .collect()
            } else {
                toml::from_str::<Config>(&content)
                    .map_err(|e| Error::FailedToParseTomlConfigFromPath(path.clone(), Box::new(e)))
                    .map(|config| vec![(Some(path), config)])?
            }
        }
        _ => {
            if let Ok(configs) = serde_json::from_str::<Vec<Config>>(&content) {
                configs
                    .into_iter()
                    .map(|c| (Some(path.clone()), c))
                    .collect()
            } else {
                serde_json::from_str::<Config>(&content)
                    .map_err(|e| Error::FailedToParseJsonConfigFromPath(path.clone(), e))
                    .map(|config| vec![(Some(path), config)])?
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
fn find_config_files() -> crate::Result<Vec<PathBuf>> {
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
fn load_configs_from_cargo_workspace(cli: &super::Cli) -> Result<Vec<(Option<PathBuf>, Config)>> {
    let profile = if cli.release {
        "release"
    } else if let Some(profile) = &cli.profile {
        profile.as_str()
    } else {
        "debug"
    };

    let mut metadata_cmd = cargo_metadata::MetadataCommand::new();
    if let Some(manifest_path) = &cli.manifest_path {
        metadata_cmd.manifest_path(manifest_path);
    }

    let metadata = match metadata_cmd.exec() {
        Ok(m) => m,
        Err(e) => {
            tracing::debug!("cargo metadata failed: {e}");
            return Ok(Vec::new());
        }
    };

    let mut configs = Vec::new();
    for package in metadata.workspace_packages().iter() {
        if let Some(config) = package.metadata.get("packager") {
            let mut config: Config = serde_json::from_value(config.to_owned())
                .map_err(Error::FailedToParseJsonConfigCargoToml)?;

            if config.name.is_none() {
                config.name.replace(package.name.to_string());
            }
            if config.product_name.is_empty() {
                config.product_name.clone_from(&package.name);
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

            let mut cargo_out_dir = metadata.target_directory.as_std_path().to_path_buf();
            if let Some(target_triple) = cli.target.as_ref().or(config.target_triple.as_ref()) {
                cargo_out_dir.push(target_triple);
            }
            cargo_out_dir.push(profile);

            if config.binaries_dir.is_none() {
                config.binaries_dir.replace(cargo_out_dir.clone());
            }
            if config.out_dir.as_os_str().is_empty() {
                config.out_dir = cargo_out_dir;
            }

            if config.description.is_none() {
                config.description.clone_from(&package.description);
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
            // Auto-detect binaries if none were explicitly configured
            if config.binaries.is_empty() {
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
                            _ => target.name == package.name.as_ref(),
                        },
                    })
                }
            }
            configs.push((
                Some(package.manifest_path.as_std_path().to_path_buf()),
                config,
            ));
        }
    }

    Ok(configs)
}

pub fn detect_configs(cli: &super::Cli) -> Result<Vec<(Option<PathBuf>, Config)>> {
    let configs = match &cli.config {
        // if a raw json object
        Some(c) if c.starts_with('{') => serde_json::from_str::<Config>(c)
            .map(|c| vec![(None, c)])
            .map_err(Error::FailedToParseJsonConfig)?,
        // if a raw json array
        Some(c) if c.starts_with('[') => serde_json::from_str::<Vec<Config>>(c)
            .map_err(Error::FailedToParseJsonConfig)?
            .into_iter()
            .map(|c| (None, c))
            .collect(),
        // if a path to config file
        Some(c) => parse_config_file(c)?,
        // fallback to config files and cargo workspaces configs
        _ => {
            let config_files = find_config_files()?
                .into_iter()
                .filter_map(|c| parse_config_file(c).ok())
                .collect::<Vec<_>>()
                .concat();

            let cargo_configs = load_configs_from_cargo_workspace(cli)?;

            [config_files, cargo_configs]
                .concat()
                .into_iter()
                .filter(|(_, c)| c.should_pacakge(cli))
                .collect()
        }
    };

    Ok(configs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_nearset_pkg_name_reads_the_manifest_next_to_the_config_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"my-app\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let config = dir.path().join("Packager.toml");
        fs::write(&config, "out_dir = \"dist\"\n").unwrap();

        let name = find_nearset_pkg_name(&config).unwrap();

        assert_eq!(name, Some("my-app".to_string()));
    }

    #[test]
    fn find_nearset_pkg_name_leaves_the_working_directory_alone() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("Packager.toml");
        fs::write(&config, "out_dir = \"dist\"\n").unwrap();
        let cwd = std::env::current_dir().unwrap();

        assert_eq!(find_nearset_pkg_name(&config).unwrap(), None);
        assert_eq!(std::env::current_dir().unwrap(), cwd);
    }

    #[test]
    fn find_nearset_pkg_name_reads_package_json_when_there_is_no_cargo_manifest() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("package.json"), r#"{"name":"my-node-app"}"#).unwrap();
        let config = dir.path().join("Packager.json");
        fs::write(&config, "{}").unwrap();

        let name = find_nearset_pkg_name(&config).unwrap();

        assert_eq!(name, Some("my-node-app".to_string()));
    }

    #[test]
    fn find_nearset_pkg_name_falls_back_to_package_json() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"app\"]\n",
        )
        .unwrap();
        fs::write(dir.path().join("package.json"), r#"{"name":"my-node-app"}"#).unwrap();
        let config = dir.path().join("Packager.toml");
        fs::write(&config, "{}").unwrap();

        let name = find_nearset_pkg_name(&config).unwrap();

        assert_eq!(name, Some("my-node-app".to_string()));
    }

    #[test]
    fn parse_config_file_fills_the_package_name_from_the_manifest() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"my-app\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let config_path = dir.path().join("Packager.toml");
        fs::write(&config_path, "product_name = \"My App\"\n").unwrap();

        let configs = parse_config_file(&config_path).unwrap();

        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].1.name(), "my-app");
    }
}
