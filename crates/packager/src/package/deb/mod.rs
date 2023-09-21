use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use handlebars::Handlebars;
use heck::AsKebabCase;
use image::{codecs::png::PngDecoder, ImageDecoder};
use relative_path::PathExt;
use serde::Serialize;
use walkdir::WalkDir;

use super::Context;
use crate::{
    config::{Config, ConfigExt, ConfigExtInternal},
    util,
};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct DebIcon {
    pub width: u32,
    pub height: u32,
    pub is_high_density: bool,
    pub path: PathBuf,
}

/// Generate the icon files and store them under the `data_dir`.
fn generate_icon_files(config: &Config, data_dir: &Path) -> crate::Result<BTreeSet<DebIcon>> {
    let hicolor_dir = data_dir.join("usr/share/icons/hicolor");
    let get_dest_path = |width: u32, height: u32, is_high_density: bool| {
        hicolor_dir.join(format!(
            "{}x{}{}/apps/{}.png",
            width,
            height,
            if is_high_density { "@2" } else { "" },
            config.main_binary_name().unwrap()
        ))
    };
    let mut icons_set = BTreeSet::new();
    if let Some(icons) = &config.icons {
        for icon_path in icons {
            let icon_path = PathBuf::from(icon_path);
            if icon_path.extension() != Some(OsStr::new("png")) {
                continue;
            }
            // Put file in scope so that it's closed when copying it
            let deb_icon = {
                let decoder = PngDecoder::new(File::open(&icon_path)?)?;
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
                std::fs::create_dir_all(
                    deb_icon
                        .path
                        .parent()
                        .ok_or(crate::Error::ParentDirNotFound)?,
                )?;
                std::fs::copy(&icon_path, &deb_icon.path)?;
                icons_set.insert(deb_icon);
            }
        }
    }
    Ok(icons_set)
}

/// Generate the application desktop file and store it under the `data_dir`.
fn generate_desktop_file(config: &Config, data_dir: &Path) -> crate::Result<()> {
    let bin_name = config.main_binary_name()?;
    let desktop_file_name = format!("{}.desktop", bin_name);
    let desktop_file_path = data_dir
        .join("usr/share/applications")
        .join(desktop_file_name);

    // For more information about the format of this file, see
    // https://developer.gnome.org/integration-guide/stable/desktop-files.html.en
    let file = &mut util::create_file(&desktop_file_path)?;

    let mut handlebars = Handlebars::new();
    handlebars.register_escape_fn(handlebars::no_escape);
    if let Some(template) = config.deb().and_then(|d| d.desktop_template.as_ref()) {
        handlebars
            .register_template_string("main.desktop", std::fs::read_to_string(template)?)
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
        icon: &'a str,
        name: &'a str,
        mime_type: Option<String>,
    }

    let mime_type = if let Some(associations) = &config.file_associations {
        let mime_types: Vec<&str> = associations
            .iter()
            .filter_map(|association| association.mime_type.as_ref())
            .map(|s| s.as_str())
            .collect();
        Some(mime_types.join(";"))
    } else {
        None
    };

    handlebars.render_to_write(
        "main.desktop",
        &DesktopTemplateParams {
            categories: config
                .category
                .map(|category| category.gnome_desktop_categories())
                .unwrap_or(""),
            comment: config.description.as_deref(),
            exec: bin_name,
            icon: bin_name,
            name: config.product_name.as_str(),
            mime_type,
        },
        file,
    )?;

    Ok(())
}

pub fn generate_data(config: &Config, data_dir: &Path) -> crate::Result<BTreeSet<DebIcon>> {
    let bin_dir = data_dir.join("usr/bin");

    log::debug!("copying binaries");
    std::fs::create_dir_all(&bin_dir)?;
    dbg!(&bin_dir);
    dbg!(&std::fs::read_dir(config.out_dir())?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()?
        .into_iter()
        .flatten());
    for bin in config.binaries.iter() {
        let bin_path = config.binary_path(bin);
        dbg!(&bin_path);
        dbg!(&std::fs::read_dir(bin_path.parent().unwrap())?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()?
            .into_iter()
            .flatten());
        std::fs::copy(&bin_path, bin_dir.join(&bin.filename))?;
    }

    log::debug!("copying resources");
    let resource_dir = data_dir.join("usr/lib").join(config.main_binary_name()?);
    config.copy_resources(&resource_dir)?;

    log::debug!("copying external binaries");
    config.copy_external_binaries(&bin_dir)?;

    log::debug!("generating icons");
    let icons = generate_icon_files(config, data_dir)?;

    log::debug!("generating desktop file");
    generate_desktop_file(config, data_dir)?;

    Ok(icons)
}

pub fn get_size<P: AsRef<Path>>(path: P) -> crate::Result<u64> {
    let mut result = 0;
    let path = path.as_ref();

    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let path = entry?.path();
            if path.is_file() {
                result += path.metadata()?.len();
            } else {
                result += get_size(path)?;
            }
        }
    } else {
        result = path.metadata()?.len();
    }

    Ok(result)
}

/// Copies user-defined files to the deb package.
fn copy_custom_files(config: &Config, data_dir: &Path) -> crate::Result<()> {
    if let Some(files) = config.deb().and_then(|d| d.files.as_ref()) {
        for (src, target) in files.iter() {
            let src = Path::new(src).canonicalize()?;
            let target = Path::new(target);
            let target = if target.is_absolute() {
                target.strip_prefix("/").unwrap()
            } else {
                target
            };

            if src.is_file() {
                let dest = data_dir.join(target);
                let parent = dest.parent().ok_or(crate::Error::ParentDirNotFound)?;
                std::fs::create_dir_all(parent)?;
                std::fs::copy(src, dest)?;
            } else if src.is_dir() {
                for entry in walkdir::WalkDir::new(&src) {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        let relative = path.relative_to(&src)?.to_path("");
                        let parent = data_dir.join(target);
                        let dest = parent.join(relative);
                        std::fs::create_dir_all(parent)?;
                        std::fs::copy(path, dest)?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Generates the debian control file and stores it under the `control_dir`.
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
    writeln!(file, "Package: {}", AsKebabCase(&config.product_name))?;
    writeln!(file, "Version: {}", &config.version)?;
    writeln!(file, "Architecture: {}", arch)?;
    // Installed-Size must be divided by 1024, see https://www.debian.org/doc/debian-policy/ch-controlfields.html#installed-size
    writeln!(file, "Installed-Size: {}", get_size(data_dir)? / 1024)?;
    let authors = config.authors.join(", ");
    writeln!(file, "Maintainer: {}", authors)?;
    if let Some(homepage) = &config.homepage {
        writeln!(file, "Homepage: {}", homepage)?;
    }
    let dependencies = config
        .deb()
        .cloned()
        .and_then(|d| d.depends)
        .unwrap_or_default();
    if !dependencies.is_empty() {
        writeln!(file, "Depends: {}", dependencies.join(", "))?;
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
    writeln!(file, "Priority: optional")?;
    file.flush()?;
    Ok(())
}

/// Create an `md5sums` file in the `control_dir` containing the MD5 checksums
/// for each file within the `data_dir`.
fn generate_md5sums(control_dir: &Path, data_dir: &Path) -> crate::Result<()> {
    let md5sums_path = control_dir.join("md5sums");
    let mut md5sums_file = util::create_file(&md5sums_path)?;
    for entry in WalkDir::new(data_dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let mut file = File::open(path)?;
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

/// Writes a tar file to the given writer containing the given directory.
fn create_tar_from_dir<P: AsRef<Path>, W: Write>(src_dir: P, dest_file: W) -> crate::Result<W> {
    let src_dir = src_dir.as_ref();
    let mut tar_builder = tar::Builder::new(dest_file);
    for entry in WalkDir::new(src_dir) {
        let entry = entry?;
        let src_path = entry.path();
        if src_path == src_dir {
            continue;
        }
        let dest_path = src_path.strip_prefix(src_dir)?;
        if entry.file_type().is_dir() {
            tar_builder.append_dir(dest_path, src_path)?;
        } else {
            let mut src_file = std::fs::File::open(src_path)?;
            tar_builder.append_file(dest_path, &mut src_file)?;
        }
    }
    let dest_file = tar_builder.into_inner()?;
    Ok(dest_file)
}

/// Creates a `.tar.gz` file from the given directory (placing the new file
/// within the given directory's parent directory), then deletes the original
/// directory and returns the path to the new file.
fn tar_and_gzip_dir<P: AsRef<Path>>(src_dir: P) -> crate::Result<PathBuf> {
    let src_dir = src_dir.as_ref();
    let dest_path = src_dir.with_extension("tar.gz");
    let dest_file = util::create_file(&dest_path)?;
    let gzip_encoder = libflate::gzip::Encoder::new(dest_file)?;
    let gzip_encoder = create_tar_from_dir(src_dir, gzip_encoder)?;
    let mut dest_file = gzip_encoder.finish().into_result()?;
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

    log::info!(action = "Packaging"; "{} ({})", deb_name, deb_path.display());

    log::debug!("generating data");
    let data_dir = deb_dir.join("data");
    let _ = generate_data(config, &data_dir)?;

    log::debug!("copying files specifeid in `deb.files`");
    copy_custom_files(config, &data_dir)?;

    let control_dir = deb_dir.join("control");
    log::debug!("generating control file");
    generate_control_file(config, arch, &control_dir, &data_dir)?;

    log::debug!("generating md5sums");
    generate_md5sums(&control_dir, &data_dir)?;

    // Generate `debian-binary` file; see
    // http://www.tldp.org/HOWTO/Debian-Binary-Package-Building-HOWTO/x60.html#AEN66
    log::debug!("creating debian-binary file");
    let debian_binary_path = deb_dir.join("debian-binary");
    let mut file = util::create_file(&debian_binary_path)?;
    file.write_all(b"2.0\n")?;
    file.flush()?;

    // Apply tar/gzip/ar to create the final package file.
    log::debug!("tar_and_gzip control dir");
    let control_tar_gz_path = tar_and_gzip_dir(control_dir)?;

    log::debug!("tar_and_gzip data dir");
    let data_tar_gz_path = tar_and_gzip_dir(data_dir)?;

    log::debug!("creating final archive: {}", deb_path.display());
    create_archive(
        vec![debian_binary_path, control_tar_gz_path, data_tar_gz_path],
        &deb_path,
    )?;
    Ok(vec![deb_path])
}
