// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! [![cargo-packager splash](https://github.com/crabnebula-dev/cargo-packager/raw/main/.github/splash.png)](https://github.com/crabnebula-dev/cargo-packager)
//!
//! `cargo-packager` is a tool and library to generate installers or app bundles for your executables.
//! It also has a comptabile updater through [cargo-packager-updater](https://docs.rs/cargo-packager-updater).
//!
//! ## CLI
//!
//! ### Installation
//!
//! ```sh
//! cargo install cargo-packager --locked
//! ```
//!
//! ### Usage
//!
//! 1. Add `Packager.toml` or `packager.json` in your project or modify Cargo.toml and include
//!
//!    ```toml
//!    [package.metadata.packager]
//!    before-packaging-command = "cargo build --release"
//!    ```
//!
//! 2. Run the CLI
//!
//!    ```sh
//!    cargo packager --release
//!    ```
//!
//! ### Supported packages
//!
//! - macOS
//!   - DMG (.dmg)
//!   - Bundle (.app)
//! - Linux
//!   - Debian package (.deb)
//!   - AppImage (.AppImage)
//! - Windows
//!   - NSIS (.exe)
//!   - MSI using WiX Toolset (.msi)
//!
//! ### Configuration
//!
//! By default, `cargo-packager` reads configuration from `Packager.toml` or `packager.json` if exists, and from `package.metadata.packager` table in `Cargo.toml`.
//! You can also specify a custom configuration file using `-c/--config` cli argument.
//! All configuration options could be either a single config or array of configs.
//!
//! For full list of configuration options, see [config::Config]
//!
//! You could also use the schema from GitHub releases to validate your configuration or have auto completions in your IDE.
//!
//! ### Building your application before packaging
//!
//! By default, `cargo-packager` doesn't build your application, it only looks for it inside the directory specified in `config.out_dir` or `--out-dir` cli arg,
//! However, `cargo-packager` has an option to specify a shell command to be executed before packaing your app, `beforePackagingCommand`.
//!
//! ### Cargo profiles
//!
//! By default, `cargo-packager` looks for binaries built using the `debug` profile, if your `beforePackagingCommand` builds your app using `cargo build --release`, you will also need to
//! run `cargo-packager` in release mode `cargo packager --release`, otherwise, if you have a custom cargo profile, you will need to specify it using `--profile` cli arg `cargo packager --profile custom-release-profile`.
//!
//! For more information, checkout the available [configuration options](config::Config) and for a list of available CLI
//! commands and arguments, run `cargo packager --help`.
//!
//! ## Library
//!
//! This crate is also published to crates.io as a library that you can integrate into your tooling, just make sure to disable the default-feature flags.
//!
//! ```sh
//! cargo add cargo-packager --no-default-features
//! ```
//!
//! #### Feature flags
//!
//! - **`cli`**: Enables the CLI specifc features and dependencies. Enabled by default.
//! - **`tracing`**: Enables `tracing` crate integration.

#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![deny(missing_docs)]

use std::path::PathBuf;

mod codesign;
mod error;
mod package;
mod shell;
mod util;

#[cfg(feature = "cli")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "cli")))]
pub mod cli;
pub mod config;
pub mod sign;

pub use config::{Config, PackageFormat};
pub use error::{Error, Result};
pub use sign::SigningConfig;

pub use package::{package, PackageOuput};

/// Sign the specified packages and return the signatures paths.
///
/// If `packages` contain a directory in the case of [`PackageFormat::App`]
/// it will zip the directory before signing and appends it to `packages`.
#[tracing::instrument(level = "trace")]
pub fn sign_outputs(
    config: &SigningConfig,
    packages: &mut Vec<PackageOuput>,
) -> crate::Result<Vec<PathBuf>> {
    let mut signatures = Vec::new();
    for package in packages {
        for path in &package.paths.clone() {
            let path = if path.is_dir() {
                let extension = path.extension().unwrap_or_default().to_string_lossy();
                let extension = format!(
                    "{}{}tar.gz",
                    extension,
                    if extension.is_empty() { "" } else { "." }
                );
                let zip = path.with_extension(extension);
                let dest_file = util::create_file(&zip)?;
                let gzip_encoder = libflate::gzip::Encoder::new(dest_file)?;
                util::create_tar_from_dir(path, gzip_encoder)?;
                package.paths.push(zip);
                package.paths.last().unwrap()
            } else {
                path
            };
            signatures.push(sign::sign_file(config, path)?);
        }
    }

    Ok(signatures)
}

/// Package an app using the specified config.
/// Then signs the generated packages.
///
/// This is similar to calling `sign_outputs(signing_config, package(config)?)`
///
/// Returns a tuple of list of packages and list of signatures.
#[tracing::instrument(level = "trace")]
pub fn package_and_sign(
    config: &Config,
    signing_config: &SigningConfig,
) -> crate::Result<(Vec<PackageOuput>, Vec<PathBuf>)> {
    let mut packages = package(config)?;
    let signatures = sign_outputs(signing_config, &mut packages)?;
    Ok((packages, signatures))
}
