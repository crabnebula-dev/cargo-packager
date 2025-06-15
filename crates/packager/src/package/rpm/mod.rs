// Copyright 2016-2019 Cargo-Bundle developers <https://github.com/burtonageo/cargo-bundle>
// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
    collections::{BTreeSet, HashMap}, ffi::OsStr, fs::{self, File}, io::BufReader, path::{Path, PathBuf}
};

use handlebars::Handlebars;
use image::{codecs::png::PngDecoder, ImageDecoder};
use relative_path::PathExt;
use rpm::{PackageBuilder, FileOptions};
use serde::Serialize;

use super::Context;
use crate::{
    config::Config,
    util::{self},
    Error,
};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct RpmIcon {
    pub width: u32,
    pub height: u32,
    pub is_high_density: bool,
    pub path: PathBuf,
}

// Generate the icon files and store them under the `data_dir`.
#[tracing::instrument(level = "trace", skip(config))]
fn generate_icon_files(config: &Config, data_dir: &Path) -> crate::Result<BTreeSet<RpmIcon>> {
    let hicolor_dir = data_dir.join("usr/share/icons/hicolor");
    let main_binary_name = config.main_binary_name()?;
    let get_dest_path = |width: u32, height: u32, is_high_density: bool| {
        hicolor_dir.join(format!(
            "{}x{}{}/apps/{}.png",
            width,
            height,
            if is_high_density { "@2" } else { "" },
            main_binary_name
        ))
    };
    let mut icons_set = BTreeSet::new();
    if let Some(icons) = config.icons()? {
        for icon_path in icons {
            if icon_path.extension() != Some(OsStr::new("png")) {
                continue;
            }
            // Put file in scope so that it's closed when copying it
            let rpm_ico = {
                let file =
                    File::open(&icon_path).map_err(|e| Error::IoWithPath(icon_path.clone(), e))?;
                let file = BufReader::new(file);
                let decoder = PngDecoder::new(file)?;
                let width = decoder.dimensions().0;
                let height = decoder.dimensions().1;
                let is_high_density = util::is_retina(&icon_path);
                let dest_path = get_dest_path(width, height, is_high_density);
                RpmIcon {
                    width,
                    height,
                    is_high_density,
                    path: dest_path,
                }
            };
            if !icons_set.contains(&rpm_ico) {
                let parent = rpm_ico
                    .path
                    .parent()
                    .ok_or_else(|| crate::Error::ParentDirNotFound(rpm_ico.path.clone()))?;
                fs::create_dir_all(parent)
                    .map_err(|e| Error::IoWithPath(parent.to_path_buf(), e))?;
                fs::copy(&icon_path, &rpm_ico.path)
                    .map_err(|e| Error::CopyFile(icon_path.clone(), rpm_ico.path.clone(), e))?;
                icons_set.insert(rpm_ico);
            }
        }
    }
    Ok(icons_set)
}

/// Generate the application desktop file and store it under the `data_dir`.
#[tracing::instrument(level = "trace", skip(config))]
fn generate_desktop_file(config: &Config, data_dir: &Path) -> crate::Result<()> {
    let bin_name = config.main_binary_name()?;
    let desktop_file_name = format!("{}.desktop", bin_name);
    let desktop_file_path = data_dir
        .join("usr/share/applications")
        .join(desktop_file_name);

    // For more information about the format of this file, see:
    // <https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html>
    let file = &mut util::create_file(&desktop_file_path)?;

    let mut handlebars = Handlebars::new();
    handlebars.register_escape_fn(handlebars::no_escape);
    if let Some(template) = config.rpm().and_then(|d| d.desktop_template.as_ref()) {
        handlebars
            .register_template_string("main.desktop", fs::read_to_string(template)?)
            .map_err(Box::new)?;
    } else {
        handlebars
            .register_template_string("main.desktop", include_str!("./main.desktop"))
            .map_err(Box::new)?;
    }

    #[derive(Serialize)]
    struct DesktopTemplateParams<'a> {
        categories: &'a str,
        comment: Option<&'a str>,
        exec: &'a str,
        exec_arg: Option<&'a str>,
        icon: &'a str,
        name: &'a str,
        mime_type: Option<String>,
    }

    // Set the argument code at the end of the `Exec` key.
    // See the docs for `DebianConfig::desktop_template` for more details.
    let mut exec_arg = None;

    let mut mime_type: Vec<String> = Vec::new();

    if let Some(associations) = &config.file_associations {
        if !associations.is_empty() {
            exec_arg = Some("%F");
        }
        mime_type.extend(
            associations
                .iter()
                .filter_map(|association| association.mime_type.clone()),
        );
    }

    if let Some(protocols) = &config.deep_link_protocols {
        if !protocols.is_empty() {
            // Use "%U" even if file associations were already provided,
            // as it can also accommodate file names in addition to URLs.
            exec_arg = Some("%U");
        }
        mime_type.extend(
            protocols
                .iter()
                .flat_map(|protocol| &protocol.schemes)
                .map(|s| format!("x-scheme-handler/{s}")),
        );
    }

    let mime_type = (!mime_type.is_empty()).then(|| mime_type.join(";"));

    let bin_name_exec = if bin_name.contains(' ') {
        format!("\"{bin_name}\"")
    } else {
        bin_name.to_string()
    };

    handlebars.render_to_write(
        "main.desktop",
        &DesktopTemplateParams {
            categories: config
                .category
                .map(|category| category.gnome_desktop_categories())
                .unwrap_or(""),
            comment: config.description.as_deref(),
            exec: &bin_name_exec,
            exec_arg,
            icon: &bin_name,
            name: config.product_name.as_str(),
            mime_type,
        },
        file,
    )?;

    Ok(())
}

#[tracing::instrument(level = "trace", skip(config))]
pub fn generate_data(config: &Config, data_dir: &Path) -> crate::Result<BTreeSet<RpmIcon>> {
    let bin_dir = data_dir.join("usr/bin");

    tracing::debug!("Copying binaries");
    fs::create_dir_all(&bin_dir).map_err(|e| Error::IoWithPath(bin_dir.clone(), e))?;

    for bin in config.binaries.iter() {
        let bin_path = config.binary_path(bin);
        let bin_out_path = bin_dir.join(bin.path.file_name().unwrap());
        fs::copy(&bin_path, &bin_out_path)
            .map_err(|e| Error::CopyFile(bin_path.clone(), bin_out_path.clone(), e))?;
    }

    tracing::debug!("Copying resources");
    let resource_dir = data_dir.join("usr/lib").join(config.main_binary_name()?);
    config.copy_resources(&resource_dir)?;

    tracing::debug!("Copying external binaries");
    config.copy_external_binaries(&bin_dir)?;

    tracing::debug!("Generating icons");
    let icons = generate_icon_files(config, data_dir)?;

    let generate_desktop_entry = config
        .linux()
        .is_none_or(|linux| linux.generate_desktop_entry);

    if generate_desktop_entry {
        tracing::debug!("Generating desktop file");
        generate_desktop_file(config, data_dir)?;
    }

    Ok(icons)
}

/// Copies user-defined files to the rpm package.
#[tracing::instrument(level = "trace")]
pub fn copy_custom_files(files: &HashMap<String, String>, data_dir: &Path) -> crate::Result<()> {
    for (src, target) in files.iter() {
        let src = Path::new(src);
        let src = src
            .canonicalize()
            .map_err(|e| Error::IoWithPath(src.to_path_buf(), e))?;
        let target = Path::new(target);
        let target = if target.is_absolute() {
            target.strip_prefix("/").unwrap()
        } else {
            target
        };

        if src.is_file() {
            let dest = data_dir.join(target);
            let parent = dest
                .parent()
                .ok_or_else(|| crate::Error::ParentDirNotFound(dest.clone()))?;
            fs::create_dir_all(parent).map_err(|e| Error::IoWithPath(parent.to_path_buf(), e))?;
            fs::copy(&src, &dest).map_err(|e| Error::CopyFile(src, dest, e))?;
        } else if src.is_dir() {
            let dest_dir = data_dir.join(target);

            for entry in walkdir::WalkDir::new(&src) {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let relative = path.relative_to(&src)?.to_path("");
                    let dest = dest_dir.join(relative);
                    let parent = dest
                        .parent()
                        .ok_or_else(|| crate::Error::ParentDirNotFound(dest.clone()))?;
                    fs::create_dir_all(parent)
                        .map_err(|e| Error::IoWithPath(parent.to_path_buf(), e))?;
                    fs::copy(path, &dest)
                        .map_err(|e| Error::CopyFile(src.clone(), dest.clone(), e))?;
                }
            }
        }
    }

    Ok(())
}


fn add_custom_files(files: &HashMap<String, String>, rpm_pkg: &mut PackageBuilder) -> crate::Result<()> {
    for (src, target) in files.iter() {
        let src = Path::new(src);
        let src = src
            .canonicalize()
            .map_err(|e| Error::IoWithPath(src.to_path_buf(), e))?;
        let target = Path::new(target);
        let target = if target.is_absolute() {
            target.strip_prefix("/").unwrap()
        } else {
            target
        };

        if src.is_file() {
            let file_options = FileOptions::new(target.to_string_lossy());
            let pkg = std::mem::take(rpm_pkg);
            *rpm_pkg = pkg
                .with_file(&src, file_options)
                .map_err(|err| Error::RpmError(err.to_string()))?;
        } else if src.is_dir() {
            for entry in walkdir::WalkDir::new(&src) {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let relative = path.relative_to(&src)?.to_path("");
                    let file_options = FileOptions::new(relative.to_string_lossy());
                    let pkg = std::mem::take(rpm_pkg);
                    *rpm_pkg = pkg
                        .with_file(path, file_options)
                        .map_err(|err| Error::RpmError(err.to_string()))?;
                }
            }
        }
    }

    Ok(())
}



#[tracing::instrument(level = "trace", skip(ctx))]
pub fn package(ctx: &Context) -> crate::Result<Vec<PathBuf>> {
    let Context {
        config,
        intermediates_path,
        ..
    } = ctx;

    let arch = match config.target_arch()? {
        "x86" => "i386",
        "x86_64" => "amd64",
        "arm" => "armhf",
        "aarch64" => "arm64",
        other => other,
    };

    let intermediates_path = intermediates_path.join("deb");
    util::create_clean_dir(&intermediates_path)?;

    let rpm_base_name = format!("{}_{}_{}", config.main_binary_name()?, config.version, arch);
    let rpm_name = format!("{rpm_base_name}.rpm");

    let _rpm_dir = intermediates_path.join(&rpm_base_name);
    let rpm_path = config.out_dir().join(&rpm_name);

    tracing::info!("Packaging {} ({})", rpm_name, rpm_path.display());

    tracing::debug!("Generating RPM package");
    let mut rpm_pkg = rpm::PackageBuilder::new(
        config.name.as_deref().unwrap_or("test"),
        &config.version,
        "MIT",
        arch,
        config.description.as_deref().unwrap_or(""),
    )
    .compression(rpm::CompressionType::Zstd);

    tracing::debug!("Copying files specified in `rpm.files`");
    // Iterate over all the files in the data directory and add them to the RPM package.
    if let Some(files) = config.rpm().and_then(|d| d.files.as_ref()) {
        add_custom_files(files, &mut rpm_pkg)
            .map_err(|err| Error::RpmError(err.to_string()))?;
    };

    tracing::debug!("Building RPM package");
    let built_rpm_pkg = rpm_pkg.build()
        .map_err(|err| Error::RpmError(err.to_string()))?;


    tracing::debug!("Writing RPM package to {}", rpm_path.display());
    let mut f = fs::File::create(&rpm_path)
        .map_err(|err| Error::IoWithPath(rpm_path.to_path_buf(), err))?;
    built_rpm_pkg.write(&mut f)
        .map_err(|err| Error::RpmError(err.to_string()))?;

    Ok(vec![])
}

