// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{env::args_os, ffi::OsStr, path::Path, process::exit};

fn main() {
    let mut args = args_os().peekable();
    let bin_name = match args
        .next()
        .as_deref()
        .map(Path::new)
        .and_then(Path::file_stem)
        .and_then(OsStr::to_str)
    {
        Some("cargo-packager") => {
            if args.peek().and_then(|s| s.to_str()) == Some("packager") {
                // remove the extra cargo subcommand
                args.next();
                Some("cargo packager".into())
            } else {
                Some("cargo-packager".into())
            }
        }
        Some(stem) => Some(stem.to_string()),
        None => {
            eprintln!("cargo-packager wrapper unable to read first argument");
            exit(1);
        }
    };

    cargo_packager::cli::run(args, bin_name)
}
