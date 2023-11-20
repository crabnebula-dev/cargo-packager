// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[clap(about = "Generate a new signing key to sign files")]
pub struct Options {
    /// Set a password for the new signing key.
    #[clap(long, env = "CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD")]
    password: Option<String>,
    #[clap(long)]
    /// A path where the private key will be stored.
    path: Option<PathBuf>,
    /// Overwrite the private key even if it exists on the specified path.
    #[clap(short, long)]
    force: bool,
    /// Run in CI mode and skip prompting for values.
    #[clap(long)]
    ci: bool,
}

pub fn command(mut options: Options) -> crate::Result<()> {
    options.ci = options.ci || std::env::var("CI").is_ok();
    if options.ci && options.password.is_none() {
        tracing::warn!("Generating a new private key without a password, for security reasons, we recommend setting a password instead.");
        options.password.replace("".into());
    }

    tracing::info!("Generating a new signing key.");
    let keypair = crate::sign::generate_key(options.password)?;

    match options.path {
        Some(path) => {
            let keys = crate::sign::save_keypair(&keypair, path, options.force)?;
            tracing::info!(
                "Finished generating and saving the keys:\n        {}\n        {}",
                keys.0.display(),
                keys.1.display()
            );
        }
        None => {
            tracing::info!("Finished generating secret key:\n{}", keypair.sk);
            tracing::info!("Finished generating public key:\n{}", keypair.pk);
        }
    }

    Ok(())
}
