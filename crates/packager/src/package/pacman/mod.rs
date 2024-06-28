// Copyright 2024-2024 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use super::{
    deb::{copy_custom_files, generate_data, tar_and_gzip_dir},
    Context,
};
use crate::{config::Config, util};
use heck::AsKebabCase;
use sha2::{Digest, Sha512};
use std::{
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
};

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

    let package_base_name = format!("{}_{}_{}", config.main_binary_name()?, config.version, arch);
    let package_name = format!("{}.tar.gz", package_base_name);

    let pkg_dir = intermediates_path.join(&package_base_name);
    let pkg_path = config.out_dir().join(&package_name);
    let pkgbuild_path = pkg_path.with_file_name("PKGBUILD");

    tracing::info!("Packaging {} ({})", package_name, pkg_path.display());

    tracing::debug!("Generating data");
    let _ = generate_data(config, &pkg_dir)?;

    tracing::debug!("Copying files specified in `pacman.files`");
    if let Some(files) = config.pacman().and_then(|d| d.files.as_ref()) {
        copy_custom_files(files, &pkg_dir)?;
    }

    // Apply tar/gzip to create the final package file.
    tracing::debug!("Creating package archive using tar and gzip");
    let data_tar_gz_path = tar_and_gzip_dir(pkg_dir)?;
    std::fs::copy(data_tar_gz_path, &pkg_path)?;

    tracing::info!("Generating PKGBUILD: {}", pkgbuild_path.display());
    generate_pkgbuild_file(config, arch, pkgbuild_path.as_path(), pkg_path.as_path())?;

    Ok(vec![pkg_path])
}

/// Generates the pacman PKGBUILD file.
/// For more information about the format of this file, see
/// <https://wiki.archlinux.org/title/PKGBUILD>
fn generate_pkgbuild_file(
    config: &Config,
    arch: &str,
    dest_dir: &Path,
    package_path: &Path,
) -> crate::Result<()> {
    let pkgbuild_path = dest_dir.with_file_name("PKGBUILD");
    let mut file = util::create_file(&pkgbuild_path)?;

    if let Some(authors) = &config.authors {
        writeln!(file, "# Maintainer: {}", authors.join(", "))?;
    }
    writeln!(file, "pkgname={}", AsKebabCase(&config.product_name))?;
    writeln!(file, "pkgver={}", config.version)?;
    writeln!(file, "pkgrel=1")?;
    writeln!(file, "epoch=")?;
    writeln!(
        file,
        "pkgdesc=\"{}\"",
        config.description.as_deref().unwrap_or("")
    )?;
    writeln!(file, "arch=('{}')", arch)?;

    if let Some(homepage) = &config.homepage {
        writeln!(file, "url=\"{}\"", homepage)?;
    }

    let dependencies = config
        .pacman()
        .and_then(|d| d.depends.as_ref())
        .map_or_else(|| Ok(Vec::new()), |d| d.to_list())?;
    writeln!(file, "depends=({})", dependencies.join(" \n"))?;

    let provides = config
        .pacman()
        .and_then(|d| d.provides.clone())
        .unwrap_or_default();
    writeln!(file, "provides=({})", provides.join(" \n"))?;

    let conflicts = config
        .pacman()
        .and_then(|d| d.conflicts.clone())
        .unwrap_or_default();
    writeln!(file, "conflicts=({})", conflicts.join(" \n"))?;

    let replaces = config
        .pacman()
        .and_then(|d| d.replaces.clone())
        .unwrap_or_default();
    writeln!(file, "replaces=({})", replaces.join(" \n"))?;

    writeln!(file, "options=(!lto)")?;
    let source = config
        .pacman()
        .and_then(|d| d.source.clone())
        .unwrap_or_default();

    if source.is_empty() {
        writeln!(file, "source=({:?})", package_path.file_name().unwrap())?;
    } else {
        writeln!(file, "source=({})", source.join(" \n"))?;
    }

    // Generate SHA512 sum of the package
    let mut sha_file = File::open(package_path)?;
    let mut sha512 = Sha512::new();
    io::copy(&mut sha_file, &mut sha512)?;
    let sha_hash = sha512.finalize();

    writeln!(file, "sha512sums=(\"{:x}\")", sha_hash)?;
    writeln!(file, "package() {{\n\tcp -r ${{srcdir}}/* ${{pkgdir}}/\n}}")?;

    file.flush()?;
    Ok(())
}
