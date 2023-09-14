use std::{
    io::Write,
    path::{Path, PathBuf},
};

use cargo_packager::{
    config::{Binary, Config},
    package, util, Result,
};
use cargo_packager_config::{LogLevel, PackageFormat};
use clap::{ArgAction, CommandFactory, FromArgMatches, Parser};
use env_logger::fmt::Color;
use log::{log_enabled, Level};

fn parse_config_file<P: AsRef<Path>>(path: P) -> crate::Result<Vec<(Option<PathBuf>, Config)>> {
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

fn find_config_files() -> Vec<PathBuf> {
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

fn load_configs_from_cargo_workspace(
    release: bool,
    profile: Option<String>,
    manifest_path: Option<PathBuf>,
    packages: Option<Vec<String>>,
) -> Result<Vec<(Option<PathBuf>, Config)>> {
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

#[derive(Parser)]
#[clap(
    author,
    version,
    about,
    bin_name("cargo-packager"),
    propagate_version(true),
    no_binary_name(true)
)]
pub(crate) struct Cli {
    /// Enables verbose logging
    #[clap(short, long, global = true, action = ArgAction::Count)]
    verbose: u8,
    /// Specify the package fromats to build.
    #[clap(long, value_enum)]
    formats: Option<Vec<PackageFormat>>,
    /// Specify a configuration to read, which could be a JSON file,
    /// TOML file, or a raw JSON string. By default, cargo-pacakger
    /// looks for `{p,P}ackager.{toml,json}` and
    /// `[package.metadata.packager]` in `Cargo.toml` files.
    #[clap(short, long)]
    config: Option<String>,

    /// Package the release version of your app.
    /// Ignored when `--config` is used.
    #[clap(short, long, group = "cargo-profile")]
    release: bool,
    /// Specify the cargo profile to use for packaging your app.
    /// Ignored when `--config` is used.
    #[clap(long, group = "cargo-profile")]
    profile: Option<String>,
    /// Specify the cargo packages to use from the current workspace.
    /// Ignored when `--config` is used.
    #[clap(short, long)]
    packages: Option<Vec<String>>,
    /// Specify the manifest path to use for reading the configuration.
    /// Ignored when `--config` is used.
    #[clap(long)]
    manifest_path: Option<PathBuf>,
}

fn try_run(cli: Cli) -> Result<()> {
    use std::fmt::Write;

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
        _ => [
            find_config_files()
                .into_iter()
                .filter_map(|c| parse_config_file(c).ok())
                .collect::<Vec<_>>()
                .concat(),
            load_configs_from_cargo_workspace(
                cli.release,
                cli.profile,
                cli.manifest_path,
                cli.packages,
            )?,
        ]
        .concat(),
    };

    if configs.is_empty() {
        log::warn!("Couldn't detect a valid configuration file! Nothing to do here.")
    }

    for (_, config) in &mut configs {
        if let Some(formats) = &cli.formats {
            config.formats.replace(formats.clone());
        }

        if config.log_level.is_none() {
            config.log_level.replace(match cli.verbose {
                0 => LogLevel::Info,
                1 => LogLevel::Debug,
                2.. => LogLevel::Trace,
            });
        }
    }

    let mut outputs = Vec::new();
    for (config_dir, config) in configs {
        if let Some(path) = config_dir {
            // change the directory to the manifest being built
            // so paths are read relative to it
            std::env::set_current_dir(
                path.parent()
                    .ok_or(cargo_packager::Error::ParentDirNotFound)?,
            )?;
        }

        // create the packages
        outputs.extend(package(&config)?);
    }

    // print information when finished
    let len = outputs.len();
    if len >= 1 {
        let pluralised = if len == 1 { "package" } else { "packages" };
        let mut printable_paths = String::new();
        for p in outputs {
            for path in &p.paths {
                writeln!(printable_paths, "        {}", util::display_path(path)).unwrap();
            }
        }
        log::info!(action = "Finished"; "{} {} at:\n{}", len, pluralised, printable_paths);
    }

    Ok(())
}

fn main() {
    // prepare cli args
    let args = std::env::args_os().skip(1);
    let cli = Cli::command();
    let matches = cli.get_matches_from(args);
    let res = Cli::from_arg_matches(&matches).map_err(|e| e.format(&mut Cli::command()));
    let cli = match res {
        Ok(s) => s,
        Err(e) => e.exit(),
    };

    // setup logger
    let filter_level = match cli.verbose {
        0 => Level::Info,
        1 => Level::Debug,
        2.. => Level::Trace,
    }
    .to_level_filter();
    let mut builder = env_logger::Builder::from_default_env();
    let logger_init_res = builder
        .format_indent(Some(12))
        .filter(None, filter_level)
        .format(|f, record| {
            let mut is_command_output = false;
            if let Some(action) = record.key_values().get("action".into()) {
                let action = action.to_str().unwrap();
                is_command_output = action == "stdout" || action == "stderr";
                if !is_command_output {
                    let mut action_style = f.style();
                    action_style.set_color(Color::Green).set_bold(true);
                    write!(f, "{:>12} ", action_style.value(action))?;
                }
            } else {
                let mut level_style = f.default_level_style(record.level());
                level_style.set_bold(true);
                let level = match record.level() {
                    Level::Error => "Error",
                    Level::Warn => "Warn",
                    Level::Info => "Info",
                    Level::Debug => "Debug",
                    Level::Trace => "Trace",
                };
                write!(f, "{:>12} ", level_style.value(level))?;
            }

            if !is_command_output && log_enabled!(Level::Debug) {
                let mut target_style = f.style();
                target_style.set_color(Color::Black);
                write!(f, "[{}] ", target_style.value(record.target()))?;
            }

            writeln!(f, "{}", record.args())
        })
        .try_init();

    if let Err(err) = logger_init_res {
        eprintln!("Failed to attach logger: {err}");
    }

    if let Err(e) = try_run(cli) {
        log::error!("{}", e);
        std::process::exit(1);
    }
}
