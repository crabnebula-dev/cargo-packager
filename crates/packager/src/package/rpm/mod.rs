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
use walkdir::WalkDir;

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

pub fn get_size<P: AsRef<Path>>(path: P) -> crate::Result<u64> {
    let mut result = 0;
    let path = path.as_ref();

    if path.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| Error::IoWithPath(path.to_path_buf(), e))? {
            let path = entry?.path();
            if path.is_file() {
                let metadata = path.metadata().map_err(|e| Error::IoWithPath(path, e))?;
                result += metadata.len();
            } else {
                result += get_size(path)?;
            }
        }
    } else {
        let metadata = path
            .metadata()
            .map_err(|e| Error::IoWithPath(path.to_path_buf(), e))?;
        result += metadata.len();
    }

    Ok(result)
}

/// Copies user-defined files to the deb package.
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

fn collect_package_files(build_root: &Path) -> crate::Result<Vec<(PathBuf, String)>> {
    let mut entries = Vec::new();
    for entry in WalkDir::new(build_root) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let rel_path = path.strip_prefix(build_root).unwrap();
            entries.push((path.to_path_buf(), format!("/{}", rel_path.display())));
        }
    }
    Ok(entries)
}

pub fn package(ctx: &Context) -> crate::Result<Vec<PathBuf>> {
    let config = &ctx.config;
    let build_root = ctx.intermediates_path.join("rpm");
    fs::create_dir_all(&build_root)?;

    generate_data(config, &build_root)?;
    if let Some(custom_files) = config.rpm().and_then(|rpm| rpm.files.as_ref()) {
        copy_custom_files(custom_files, &build_root)?;
    }

    let package_files = collect_package_files(&build_root)?;
    // Try to extract the license name from the license file, or use "MIT" as default
    let license = if let Some(license_file) = &config.license_file {
        match fs::read_to_string(license_file) {
            Ok(content) => {
                // Use the first non-empty line as the license name
                content
                    .lines()
                    .find(|line| !line.trim().is_empty())
                    .map(|line| line.trim().to_string())
                    .unwrap_or_else(|| "MIT".to_string())
            }
            Err(_) => "MIT".to_string(),
        }
    } else {
        "MIT".to_string()
    };

    let mut pkg_builder = PackageBuilder::new(
        &config.main_binary_name()?,
        &config.version,
        &license,
        "x86_64",
        config.description.as_deref().unwrap_or(""),
    );

    for (src, dest) in package_files {
        let mut opts = FileOptions::new(dest.clone());
        // Set executable bit for files in /usr/bin, otherwise 644
        if dest.starts_with("/usr/bin/") {
            opts = opts.mode(0o755);
        } else {
            opts = opts.mode(0o644);
        }
        pkg_builder = pkg_builder.with_file(src, opts).map_err(|e| Error::RpmError(e.to_string()))?;
    }

    let rpm = pkg_builder.build().map_err(|e| Error::RpmError(e.to_string()))?;
    let out_path = ctx.config.out_dir.as_path().join(format!("{}-{}.rpm", config.main_binary_name()?, config.version));
    let mut out_file = File::create(&out_path)?;
    rpm.write(&mut out_file).map_err(|e| Error::RpmError(e.to_string()))?;

    Ok(vec![out_path])
}

