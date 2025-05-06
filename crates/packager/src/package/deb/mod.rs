// Copyright 2016-2019 Cargo-Bundle developers <https://github.com/burtonageo/cargo-bundle>
// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
    collections::{BTreeSet, HashMap},
    ffi::OsStr,
    fs::{self, File},
    io::{BufReader, Write},
    path::{Path, PathBuf},
};

use flate2::{write::GzEncoder, Compression};
use handlebars::Handlebars;
use heck::AsKebabCase;
use image::{codecs::png::PngDecoder, ImageDecoder};
use relative_path::PathExt;
use serde::Serialize;
use tar::HeaderMode;
use walkdir::WalkDir;

use super::Context;
use crate::{
    config::Config,
    util::{self, PathExt as UtilPathExt},
    Error,
};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct DebIcon {
    pub width: u32,
    pub height: u32,
    pub is_high_density: bool,
    pub path: PathBuf,
}

/// Generate the icon files and store them under the `data_dir`.
#[tracing::instrument(level = "trace", skip(config))]
fn generate_icon_files(config: &Config, data_dir: &Path) -> crate::Result<BTreeSet<DebIcon>> {
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
            let deb_icon = {
                let file =
                    File::open(&icon_path).map_err(|e| Error::IoWithPath(icon_path.clone(), e))?;
                let file = BufReader::new(file);
                let decoder = PngDecoder::new(file)?;
                let width = decoder.dimensions().0;
                let height = decoder.dimensions().1;
                let is_high_density = util::is_retina(&icon_path);
                let dest_path = get_dest_path(width, height, is_high_density);
                DebIcon {
                    width,
                    height,
                    is_high_density,
                    path: dest_path,
                }
            };
            if !icons_set.contains(&deb_icon) {
                let parent = deb_icon
                    .path
                    .parent()
                    .ok_or_else(|| crate::Error::ParentDirNotFound(deb_icon.path.clone()))?;
                fs::create_dir_all(parent)
                    .map_err(|e| Error::IoWithPath(parent.to_path_buf(), e))?;
                fs::copy(&icon_path, &deb_icon.path)
                    .map_err(|e| Error::CopyFile(icon_path.clone(), deb_icon.path.clone(), e))?;
                icons_set.insert(deb_icon);
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
    if let Some(template) = config.deb().and_then(|d| d.desktop_template.as_ref()) {
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
pub fn generate_data(config: &Config, data_dir: &Path) -> crate::Result<BTreeSet<DebIcon>> {
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

/// Generates the debian control file and stores it under the `control_dir`.
#[tracing::instrument(level = "trace", skip(config))]
fn generate_control_file(
    config: &Config,
    arch: &str,
    control_dir: &Path,
    data_dir: &Path,
) -> crate::Result<()> {
    // For more information about the format of this file, see
    // https://www.debian.org/doc/debian-policy/ch-controlfields.html
    let dest_path = control_dir.join("control");
    let mut file = util::create_file(&dest_path)?;

    let pkg_name = AsKebabCase(&config.product_name).to_string();
    let pkg_name: String = config
        .deb()
        .map(|x| x.package_name.clone().unwrap_or(pkg_name.clone()))
        .unwrap_or(pkg_name);

    writeln!(file, "Package: {}", pkg_name)?;
    writeln!(file, "Version: {}", &config.version)?;
    writeln!(file, "Architecture: {}", arch)?;
    // Installed-Size must be divided by 1024, see https://www.debian.org/doc/debian-policy/ch-controlfields.html#installed-size
    writeln!(file, "Installed-Size: {}", get_size(data_dir)? / 1024)?;
    if let Some(authors) = &config.authors {
        writeln!(file, "Maintainer: {}", authors.join(", "))?;
    }
    if let Some(section) = config.deb().and_then(|d| d.section.as_ref()) {
        writeln!(file, "Section: {}", section)?;
    }

    if let Some(priority) = config.deb().and_then(|d| d.priority.as_ref()) {
        writeln!(file, "Priority: {}", priority)?;
    } else {
        writeln!(file, "Priority: optional")?;
    }

    if let Some(homepage) = &config.homepage {
        writeln!(file, "Homepage: {}", homepage)?;
    }
    if let Some(depends) = config.deb().and_then(|d| d.depends.as_ref()) {
        let dependencies = depends.to_list()?;
        if !dependencies.is_empty() {
            writeln!(file, "Depends: {}", dependencies.join(", "))?;
        }
    }

    writeln!(
        file,
        "Description: {}",
        config.description.as_deref().unwrap_or("(none)")
    )?;
    for line in config
        .long_description
        .as_deref()
        .unwrap_or("(none)")
        .lines()
    {
        let line = line.trim();
        if line.is_empty() {
            writeln!(file, " .")?;
        } else {
            writeln!(file, " {}", line)?;
        }
    }
    file.flush()?;
    Ok(())
}

/// Creates an `md5sums` file in the `control_dir` containing the MD5 checksums
/// for each file within the `data_dir`.
#[tracing::instrument(level = "trace")]
fn generate_md5sums(control_dir: &Path, data_dir: &Path) -> crate::Result<()> {
    let md5sums_path = control_dir.join("md5sums");
    let mut md5sums_file = util::create_file(&md5sums_path)?;
    for entry in WalkDir::new(data_dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let mut file = File::open(path).map_err(|e| Error::IoWithPath(path.to_path_buf(), e))?;
        let mut hash = md5::Context::new();
        std::io::copy(&mut file, &mut hash)?;
        for byte in hash.compute().iter() {
            write!(md5sums_file, "{:02x}", byte)?;
        }
        let rel_path = path.strip_prefix(data_dir)?;
        let path_str = rel_path.to_str().ok_or_else(|| {
            let msg = format!("Non-UTF-8 path: {:?}", rel_path);
            std::io::Error::new(std::io::ErrorKind::InvalidData, msg)
        })?;
        writeln!(md5sums_file, "  {}", path_str)?;
    }
    Ok(())
}

fn create_tar_from_dir<P: AsRef<Path>, W: Write>(src_dir: P, dest_file: W) -> crate::Result<W> {
    use std::os::unix::fs::MetadataExt;

    let src_dir = src_dir.as_ref();
    let mut tar_builder = tar::Builder::new(dest_file);
    for entry in walkdir::WalkDir::new(src_dir) {
        let entry = entry?;
        let src_path = entry.path();
        if src_path == src_dir {
            continue;
        }
        let dest_path = src_path.strip_prefix(src_dir)?;
        let stat =
            fs::metadata(src_path).map_err(|e| Error::IoWithPath(src_path.to_path_buf(), e))?;
        let mut header = tar::Header::new_gnu();
        header.set_metadata_in_mode(&stat, HeaderMode::Deterministic);
        header.set_mtime(stat.mtime() as u64);
        if entry.file_type().is_dir() {
            tar_builder.append_data(&mut header, dest_path, &mut std::io::empty())?;
        } else {
            let mut src_file =
                File::open(src_path).map_err(|e| Error::IoWithPath(src_path.to_path_buf(), e))?;
            tar_builder.append_data(&mut header, dest_path, &mut src_file)?;
        }
    }
    tar_builder.into_inner().map_err(Into::into)
}

/// Creates a `.tar.gz` file from the given directory (placing the new file
/// within the given directory's parent directory), then deletes the original
/// directory and returns the path to the new file.
pub fn tar_and_gzip_dir<P: AsRef<Path>>(src_dir: P) -> crate::Result<PathBuf> {
    let src_dir = src_dir.as_ref();
    let dest_path = src_dir.with_additional_extension("tar.gz");
    let dest_file = util::create_file(&dest_path)?;
    let gzip_encoder = GzEncoder::new(dest_file, Compression::default());
    let gzip_encoder = create_tar_from_dir(src_dir, gzip_encoder)?;
    let mut dest_file = gzip_encoder.finish()?;
    dest_file.flush()?;
    Ok(dest_path)
}

/// Creates an `ar` archive from the given source files and writes it to the
/// given destination path.
fn create_archive(srcs: Vec<PathBuf>, dest: &Path) -> crate::Result<()> {
    let mut builder = ar::Builder::new(util::create_file(dest)?);
    for path in &srcs {
        builder.append_path(path)?;
    }
    builder.into_inner()?.flush()?;
    Ok(())
}

#[tracing::instrument(level = "trace", skip(ctx))]
pub(crate) fn package(ctx: &Context) -> crate::Result<Vec<PathBuf>> {
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

    let deb_base_name = format!("{}_{}_{}", config.main_binary_name()?, config.version, arch);
    let deb_name = format!("{deb_base_name}.deb");

    let deb_dir = intermediates_path.join(&deb_base_name);
    let deb_path = config.out_dir().join(&deb_name);

    tracing::info!("Packaging {} ({})", deb_name, deb_path.display());

    tracing::debug!("Generating data");
    let data_dir = deb_dir.join("data");
    let _ = generate_data(config, &data_dir)?;

    tracing::debug!("Copying files specified in `deb.files`");
    if let Some(files) = config.deb().and_then(|d| d.files.as_ref()) {
        copy_custom_files(files, &data_dir)?;
    }

    let control_dir = deb_dir.join("control");
    tracing::debug!("Generating control file");
    generate_control_file(config, arch, &control_dir, &data_dir)?;

    tracing::debug!("Generating md5sums");
    generate_md5sums(&control_dir, &data_dir)?;

    // Generate `debian-binary` file; see
    // http://www.tldp.org/HOWTO/Debian-Binary-Package-Building-HOWTO/x60.html#AEN66
    tracing::debug!("Creating debian-binary file");
    let debian_binary_path = deb_dir.join("debian-binary");
    let mut file = util::create_file(&debian_binary_path)?;
    file.write_all(b"2.0\n")?;
    file.flush()?;

    // Apply tar/gzip/ar to create the final package file.
    tracing::debug!("Zipping control dir using tar and gzip");
    let control_tar_gz_path = tar_and_gzip_dir(control_dir)?;

    tracing::debug!("Zipping data dir using tar and gzip");
    let data_tar_gz_path = tar_and_gzip_dir(data_dir)?;

    tracing::debug!("Creating final archive: {}", deb_path.display());
    create_archive(
        vec![debian_binary_path, control_tar_gz_path, data_tar_gz_path],
        &deb_path,
    )?;
    Ok(vec![deb_path])
}
