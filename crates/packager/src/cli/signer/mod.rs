// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use clap::{Parser, Subcommand};

use super::Result;

mod generate;
mod sign;

#[derive(Debug, Clone, Subcommand)]
enum Commands {
    Sign(sign::Options),
    Generate(generate::Options),
}

#[derive(Debug, Clone, Parser)]
#[clap(about = "Sign a file or generate a new signing key to sign files")]
pub struct Options {
    #[command(subcommand)]
    command: Commands,
}

pub fn command(options: Options) -> Result<()> {
    match options.command {
        Commands::Sign(opts) => sign::command(opts).map_err(Into::into),
        Commands::Generate(opts) => generate::command(opts),
    }
}
