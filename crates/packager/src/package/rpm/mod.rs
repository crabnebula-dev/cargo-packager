// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{io, path::PathBuf};

use heck::AsKebabCase;

use super::{deb, Context};
use crate::{util, Error};

#[tracing::instrument(level = "trace", skip(ctx))]
pub(crate) fn package(ctx: &Context) -> crate::Result<Vec<PathBuf>> {
    let Context {
        config,
        intermediates_path,
        ..
    } = ctx;
    let arch = match config.target_arch()? {
        "x86" => "i686",
        "arm" => "armv7hl",
        other => other,
    };
    let name = config
        .rpm()
        .and_then(|rpm| rpm.package_name.clone())
        .unwrap_or_else(|| AsKebabCase(&config.product_name).to_string());
    let summary = config
        .description
        .as_deref()
        .unwrap_or(&config.product_name);
    let work = intermediates_path.join("rpm");
    util::create_clean_dir(&work)?;
    let payload = work.join("payload");

    tracing::debug!("Generating RPM payload");
    deb::generate_data(config, &payload)?;
    if let Some(files) = config.rpm().and_then(|rpm| rpm.files.as_ref()) {
        deb::copy_custom_files(files, &payload)?;
    }

    let mut builder = rpm::PackageBuilder::new(
        &name,
        &config.version,
        config
            .rpm()
            .and_then(|rpm| rpm.license.as_deref())
            .unwrap_or("Unspecified"),
        arch,
        summary,
    );
    builder.release("1");
    builder.description(config.long_description.as_deref().unwrap_or(summary));
    if let Some(homepage) = &config.homepage {
        builder.url(homepage);
    }
    if let Some(dependencies) = config.rpm().and_then(|rpm| rpm.depends.as_ref()) {
        for dependency in dependencies.to_list()? {
            builder.requires(rpm::Dependency::any(dependency));
        }
    }
    builder
        .with_dir(&payload, "/", |options| options)
        .map_err(|error| Error::Io(io::Error::other(error.to_string())))?;
    let package = builder
        .build()
        .map_err(|error| Error::Io(io::Error::other(error.to_string())))?;

    let out_dir = config.out_dir();
    let rpm_path = package
        .write_to(&out_dir)
        .map_err(|error| Error::Io(io::Error::other(error.to_string())))?;
    tracing::info!("Created RPM package {}", rpm_path.display());
    Ok(vec![rpm_path])
}
