// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{fs, path::PathBuf};

use clap::Parser;

use crate::cli::{Error, Result};

#[derive(Debug, Clone, Parser)]
#[clap(about = "Sign a file")]
pub struct Options {
    /// Load the private key from a file or a string.
    #[clap(short = 'k', long, env = "CARGO_PACKAGER_SIGN_PRIVATE_KEY")]
    private_key: Option<String>,
    /// The password for the private key.
    #[clap(long, env = "CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD")]
    password: Option<String>,
    /// The file to be signed.
    file: PathBuf,
}

pub fn command(options: Options) -> Result<()> {
    let private_key = match options.private_key {
        Some(path) if PathBuf::from(&path).exists() => {
            fs::read_to_string(&path).map_err(|e| Error::IoWithPath(PathBuf::from(&path), e))?
        }
        Some(key) => key,
        None => {
            tracing::error!("--private-key was not specified, aborting signign.");
            std::process::exit(1);
        }
    };

    let config = crate::sign::SigningConfig {
        private_key,
        password: Some(options.password.unwrap_or_default()),
    };
    let signature_path = crate::sign::sign_file(&config, options.file)?;

    tracing::info!(
        "Signed the file successfully! find the signature at: {}",
        signature_path.0.display()
    );

    Ok(())
}
