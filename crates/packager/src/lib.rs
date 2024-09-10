// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! [![cargo-packager splash](https://github.com/crabnebula-dev/cargo-packager/raw/main/.github/splash.png)](https://github.com/crabnebula-dev/cargo-packager)
//!
//! Executable packager, bundler and updater. A cli tool and library to generate installers or app bundles for your executables.
//! It also comes with useful addons:
//! - an [updater](https://docs.rs/cargo-packager-updater)
//! - a [resource resolver](https://docs.rs/cargo-packager-resource-resolver)
//!
//! ### Supported packages
//!
//! - macOS
//!   - DMG (.dmg)
//!   - Bundle (.app)
//! - Linux
//!   - Debian package (.deb)
//!   - AppImage (.AppImage)
//!   - Pacman (.tar.gz and PKGBUILD)
//! - Windows
//!   - NSIS (.exe)
//!   - MSI using WiX Toolset (.msi)
//!
//! ## CLI
//!
//! This crate is a cargo subcommand so you can install using:
//!
//! ```sh
//! cargo install cargo-packager --locked
//! ```
//! You then need to configure your app so the cli can recognize it. Configuration can be done in `Packager.toml` or `packager.json` in your project or modify Cargo.toml and include this snippet:
//!
//! ```toml
//! [package.metadata.packager]
//! before-packaging-command = "cargo build --release"
//! ```
//!
//! Once, you are done configuring your app, run:
//!
//! ```sh
//! cargo packager --release
//! ```
//!
//! ### Configuration
//!
//! By default, the packager reads its configuration from `Packager.toml` or `packager.json` if it exists, and from `package.metadata.packager` table in `Cargo.toml`.
//! You can also specify a custom configuration using the `-c/--config` cli argument.
//!
//! For a full list of configuration options, see [Config].
//!
//! You could also use the [schema](./schema.json) file from GitHub to validate your configuration or have auto completions in your IDE.
//!
//! ### Building your application before packaging
//!
//! By default, the packager doesn't build your application, so if your app requires a compilation step, the packager has an option to specify a shell command to be executed before packaing your app, `beforePackagingCommand`.
//!
//! ### Cargo profiles
//!
//! By default, the packager looks for binaries built using the `debug` profile, if your `beforePackagingCommand` builds your app using `cargo build --release`, you will also need to
//! run the packager in release mode `cargo packager --release`, otherwise, if you have a custom cargo profile, you will need to specify it using `--profile` cli arg `cargo packager --profile custom-release-profile`.
//!
//! ### Library
//!
//! This crate is also published to crates.io as a library that you can integrate into your tooling, just make sure to disable the default-feature flags.
//!
//! ```sh
//! cargo add cargo-packager --no-default-features
//! ```
//!
//! #### Feature flags
//!
//! - **`cli`**: Enables the cli specifc features and dependencies. Enabled by default.
//! - **`tracing`**: Enables `tracing` crate integration.

#![cfg_attr(doc_cfg, feature(doc_cfg))]
#![deny(missing_docs)]

use std::{io::Write, path::PathBuf};

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
use flate2::{write::GzEncoder, Compression};
pub use sign::SigningConfig;

pub use package::{package, PackageOutput};
use util::PathExt;

#[cfg(feature = "cli")]
fn parse_log_level(verbose: u8) -> tracing::Level {
    match verbose {
        0 => tracing_subscriber::EnvFilter::builder()
            .from_env_lossy()
            .max_level_hint()
            .and_then(|l| l.into_level())
            .unwrap_or(tracing::Level::INFO),
        1 => tracing::Level::DEBUG,
        2.. => tracing::Level::TRACE,
    }
}

/// Inits the tracing subscriber.
#[cfg(feature = "cli")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "cli")))]
pub fn init_tracing_subscriber(verbosity: u8) {
    let level = parse_log_level(verbosity);

    let debug = level == tracing::Level::DEBUG;
    let tracing = level == tracing::Level::TRACE;

    let subscriber = tracing_subscriber::fmt()
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .with_target(debug)
        .with_line_number(tracing)
        .with_file(tracing)
        .with_max_level(level);

    let formatter = tracing_subscriber::fmt::format()
        .compact()
        .with_target(debug)
        .with_line_number(tracing)
        .with_file(tracing);

    if tracing {
        subscriber
            .event_format(TracingFormatter::WithTime(formatter))
            .init();
    } else {
        subscriber
            .without_time()
            .event_format(TracingFormatter::WithoutTime(formatter.without_time()))
            .init();
    }
}

#[cfg(feature = "cli")]
enum TracingFormatter {
    WithoutTime(
        tracing_subscriber::fmt::format::Format<tracing_subscriber::fmt::format::Compact, ()>,
    ),
    WithTime(tracing_subscriber::fmt::format::Format<tracing_subscriber::fmt::format::Compact>),
}

#[cfg(feature = "cli")]
struct ShellFieldVisitor {
    message: String,
}

#[cfg(feature = "cli")]
impl tracing::field::Visit for ShellFieldVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }
}

#[cfg(feature = "cli")]
impl<S, N> tracing_subscriber::fmt::FormatEvent<S, N> for TracingFormatter
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &tracing_subscriber::fmt::FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        if event.fields().any(|f| f.name() == "shell") {
            let mut visitor = ShellFieldVisitor { message: "".into() };
            event.record(&mut visitor);
            writeln!(writer, "{}", visitor.message)
        } else {
            match self {
                TracingFormatter::WithoutTime(formatter) => {
                    formatter.format_event(ctx, writer, event)
                }
                TracingFormatter::WithTime(formatter) => formatter.format_event(ctx, writer, event),
            }
        }
    }
}

/// Sign the specified packages and return the signatures paths.
///
/// If `packages` contain a directory in the case of [`PackageFormat::App`]
/// it will zip the directory before signing and appends it to `packages`.
#[tracing::instrument(level = "trace")]
pub fn sign_outputs(
    config: &SigningConfig,
    packages: &mut Vec<PackageOutput>,
) -> crate::Result<Vec<PathBuf>> {
    let mut signatures = Vec::new();
    for package in packages {
        for path in &package.paths.clone() {
            let path = if path.is_dir() {
                let zip = path.with_additional_extension("tar.gz");
                let dest_file = util::create_file(&zip)?;
                let gzip_encoder = GzEncoder::new(dest_file, Compression::default());
                let writer = util::create_tar_from_dir(path, gzip_encoder)?;
                let mut dest_file = writer.finish()?;
                dest_file.flush()?;

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
) -> crate::Result<(Vec<PackageOutput>, Vec<PathBuf>)> {
    let mut packages = package(config)?;
    let signatures = sign_outputs(signing_config, &mut packages)?;
    Ok((packages, signatures))
}
