//! The cli entry point

#![cfg(feature = "cli")]

use std::{fmt::Write, path::PathBuf};

use clap::{ArgAction, CommandFactory, FromArgMatches, Parser, Subcommand};

use self::config::{find_config_files, load_configs_from_cargo_workspace, parse_config_file};
use crate::{
    config::{Config, LogLevel, PackageFormat},
    package, sign_outputs, util, Result, SigningConfig,
};

mod config;
mod signer;

#[derive(Debug, Clone, Subcommand)]
enum Commands {
    Signer(signer::Options),
}

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about,
    bin_name("cargo-packager"),
    propagate_version(true),
    no_binary_name(true)
)]
pub(crate) struct Cli {
    /// Enables verbose logging.
    #[clap(short, long, global = true, action = ArgAction::Count)]
    verbose: u8,
    /// Disables logging
    #[clap(short, long)]
    quite: bool,

    /// Specify the package fromats to build.
    #[clap(short, long, value_enum, value_delimiter = ',')]
    formats: Option<Vec<PackageFormat>>,
    /// Specify a configuration to read, which could be a JSON file,
    /// TOML file, or a raw JSON string.
    ///
    /// By default, cargo-pacakger looks for `{p,P}ackager.{toml,json}` and
    /// `[package.metadata.packager]` in `Cargo.toml` files.
    #[clap(short, long)]
    config: Option<String>,
    /// Load a private key from a file or a string to sign the generated ouptuts.
    #[clap(short = 'k', long, env = "CARGO_PACKAGER_SIGN_PRIVATE_KEY")]
    private_key: Option<String>,
    /// The password for the signing private key.
    #[clap(long, env = "CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD")]
    password: Option<String>,
    /// Specify which packages to use from the current workspace.
    #[clap(short, long, value_delimiter = ',')]
    packages: Option<Vec<String>>,
    /// Specify The directory where the `binaries` exist and where the packages will be placed.
    out_dir: Option<PathBuf>,

    /// Package the release version of your app.
    /// Ignored when `--config` is used.
    #[clap(short, long, group = "cargo-profile")]
    release: bool,
    /// Specify the cargo profile to use for packaging your app.
    /// Ignored when `--config` is used.
    #[clap(long, group = "cargo-profile")]
    profile: Option<String>,
    /// Specify the manifest path to use for reading the configuration.
    /// Ignored when `--config` is used.
    #[clap(long)]
    manifest_path: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
fn try_run(cli: Cli) -> Result<()> {
    // run subcommand and exit if one was specified,
    // otherwise run the default packaging command
    if let Some(command) = cli.command {
        match command {
            Commands::Signer(opts) => signer::command(opts)?,
        }
        return Ok(());
    }

    let mut configs = match cli.config {
        // if a raw json object
        Some(c) if c.starts_with('{') => vec![(None, serde_json::from_str::<Config>(&c)?)],
        // if a raw json array
        Some(c) if c.starts_with('[') => serde_json::from_str::<Vec<Config>>(&c)?
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
            let cargo_configs =
                load_configs_from_cargo_workspace(cli.release, cli.profile, cli.manifest_path)?;
            [config_files, cargo_configs]
                .concat()
                .into_iter()
                .filter(|(_, c)| {
                    // skip if this package was not specified in the explicit packages to build
                    // otherwise we should package it if `cli_packages` was `None`
                    cli.packages
                        .as_ref()
                        .map(|p| {
                            c.name
                                .as_ref()
                                .map(|name| p.contains(name))
                                .unwrap_or(false)
                        })
                        .unwrap_or(true)
                })
                .collect()
        }
    };

    if configs.is_empty() {
        tracing::warn!("Couldn't detect a valid configuration file! Nothing to do here.");
        return Ok(());
    }

    let cli_out_dir = cli.out_dir.as_ref().map(dunce::canonicalize).transpose()?;

    for (_, config) in &mut configs {
        if let Some(dir) = &cli_out_dir {
            config.out_dir = dir.clone()
        }

        if let Some(formats) = &cli.formats {
            config.formats.replace(formats.clone());
        }

        if config.log_level.is_none() && !cli.quite {
            let level = match parse_log_level(cli.verbose) {
                tracing::Level::ERROR => LogLevel::Error,
                tracing::Level::WARN => LogLevel::Warn,
                tracing::Level::INFO => LogLevel::Info,
                tracing::Level::DEBUG => LogLevel::Debug,
                tracing::Level::TRACE => LogLevel::Trace,
            };
            config.log_level.replace(level);
        }
    }

    let private_key = match cli.private_key {
        Some(path) if PathBuf::from(&path).exists() => std::fs::read_to_string(path).ok(),
        k => k,
    };
    let signing_config = private_key.map(|k| SigningConfig {
        private_key: k,
        password: cli.password,
    });

    let mut outputs = Vec::new();
    let mut signatures = Vec::new();
    for (config_dir, config) in configs {
        if let Some(path) = config_dir {
            // change the directory to the config being built
            // so paths will be read relative to it
            std::env::set_current_dir(
                path.parent()
                    .ok_or_else(|| crate::Error::ParentDirNotFound(path.clone()))?,
            )?;
        }

        // create the packages
        let mut packages = package(&config)?;

        // sign the packages
        if let Some(signing_config) = &signing_config {
            let s = sign_outputs(signing_config, &mut packages)?;
            signatures.extend(s);
        }

        outputs.extend(packages);
    }

    // flatten paths
    let outputs = outputs
        .into_iter()
        .flat_map(|o| o.paths)
        .collect::<Vec<_>>();

    // print information when finished
    let len = outputs.len();
    if len >= 1 {
        let pluralised = if len == 1 { "package" } else { "packages" };
        let mut printable_paths = String::new();
        for path in outputs {
            let _ = writeln!(printable_paths, "        {}", util::display_path(path));
        }
        tracing::info!(
            "Finished packaging {} {} at:\n{}",
            len,
            pluralised,
            printable_paths
        );
    }

    let len = signatures.len();
    if len >= 1 {
        let pluralised = if len == 1 { "signature" } else { "signatures" };
        let mut printable_paths = String::new();
        for path in signatures {
            let _ = writeln!(printable_paths, "        {}", util::display_path(path));
        }
        tracing::info!(
            "Finished signing packages, {} {} at:\n{}",
            len,
            pluralised,
            printable_paths
        );
    }

    Ok(())
}

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

/// Run the packager CLI
pub fn run() {
    // prepare cli args
    let args = std::env::args_os().skip(1);
    let cli = Cli::command();
    let matches = cli.get_matches_from(args);
    let res = Cli::from_arg_matches(&matches).map_err(|e| e.format(&mut Cli::command()));
    let cli = match res {
        Ok(s) => s,
        Err(e) => e.exit(),
    };

    if !cli.quite {
        let level = parse_log_level(cli.verbose);

        let debug = level == tracing::Level::DEBUG;
        let tracing = level == tracing::Level::TRACE;

        tracing_subscriber::fmt()
            .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
            .without_time()
            .with_target(debug)
            .with_line_number(tracing)
            .with_file(tracing)
            .with_max_level(level)
            .init();
    }

    if let Err(e) = try_run(cli) {
        tracing::error!("{}", e);
        std::process::exit(1);
    }
}
