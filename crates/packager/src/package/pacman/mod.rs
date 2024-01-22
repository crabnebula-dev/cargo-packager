// Copyright 2024-2024 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use super::{Context, deb::{copy_custom_files, generate_data, tar_and_gzip_dir}};
use heck::AsKebabCase;
use sha2::{Digest, Sha512};
use std::{fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};
use crate::{config::Config, util};

#[tracing::instrument(level = "trace")]
pub(crate) fn package(ctx: &Context) -> crate::Result<Vec<PathBuf>> {
    let Context {
        config,
        intermediates_path,
        ..
    } = ctx;

    let arch = match config.target_arch()? {
        "x86" => "i386",
        "arm" => "armhf",
        other => other,
    };

    let intermediates_path = intermediates_path.join("pacman");
    util::create_clean_dir(&intermediates_path)?;

    let package_base_name = format!("{}-{}-1-{}", config.main_binary_name()?, config.version, arch);
    let package_name = format!("{}.tar.gz", package_base_name);

    let pkg_dir = intermediates_path.join(&package_base_name);
    let pkg_path = config.out_dir().join(&package_name);

    tracing::info!("Packaging {} ({})", package_name, pkg_path.display());

    tracing::debug!("Generating data");
    let _ = generate_data(config, &pkg_dir)?;

    tracing::debug!("Copying files specified in `deb.files`");
    if let Some(files) = config.deb().and_then(|d| d.files.as_ref()) {
        copy_custom_files(files, &pkg_dir)?;
    }

    // Apply tar/gzip to create the final package file.
    tracing::debug!("Creating package archive using tar and gzip");
    let data_tar_gz_path = tar_and_gzip_dir(pkg_dir)?;
    std::fs::copy(data_tar_gz_path, &pkg_path)?;

    Ok(vec![pkg_path])
}
