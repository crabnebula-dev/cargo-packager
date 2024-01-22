// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use handlebars::{to_json, Handlebars};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::{
    collections::BTreeMap,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};
use tar::HeaderMode;

use crate::util::PathExt;
use crate::{shell::CommandExt, util};

use super::Context;

#[tracing::instrument(level = "trace")]
fn donwload_dependencies(
    ctx: &Context,
    appimage_tools_path: &Path,
    arch: &str,
    linuxdeploy_arch: &str,
) -> crate::Result<()> {
    let internal_deps = vec![
        (
            format!("AppRun-{arch}"),
            format!("https://github.com/AppImage/AppImageKit/releases/download/continuous/AppRun-{arch}")
        ),
        (
            format!("linuxdeploy-{linuxdeploy_arch}.AppImage"),
            format!("https://github.com/tauri-apps/binary-releases/releases/download/linuxdeploy/linuxdeploy-{linuxdeploy_arch}.AppImage")
        ),
    ];

    let user_deps = ctx
        .config
        .appimage()
        .and_then(|a| a.linuxdeploy_plugins.clone())
        .unwrap_or_default()
        .into_iter()
        .map(|mut p| {
            p.0 = format!("linuxdeploy-plugin-{}.sh", p.0);
            p
        })
        .collect();

    for (path, url) in [internal_deps, user_deps].concat() {
        let path = appimage_tools_path.join(path);
        if !path.exists() {
            let data = util::download(&url)?;
            tracing::debug!(
                "Writing {} and setting its permissions to 764",
                path.display()
            );
            std::fs::write(&path, data)?;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o764))?;
        }
    }

    Ok(())
}

fn tar_and_gzip_file<P: AsRef<Path>>(file: P) -> crate::Result<PathBuf> {
    let file = file.as_ref();
    let dest_path = file.with_additional_extension("tar.gz");
    let dest_file = util::create_file(&dest_path)?;
    let gzip_encoder = libflate::gzip::Encoder::new(dest_file)?;
    let gzip_encoder = create_tar_from_file(file, gzip_encoder)?;
    let mut dest_file = gzip_encoder.finish().into_result()?;
    dest_file.flush()?;
    Ok(dest_path)
}

fn create_tar_from_file<P: AsRef<Path>, W: Write>(src_file: P, dest_file: W) -> crate::Result<W> {
    let src_file = src_file.as_ref();
    let mut tar_builder = tar::Builder::new(dest_file);
    let dest_path = src_file.file_name().unwrap();
    let stat = std::fs::metadata(src_file)?;
    let mut header = tar::Header::new_gnu();
    header.set_metadata_in_mode(&stat, HeaderMode::Deterministic);
    header.set_mtime(stat.mtime() as u64);
    let mut src_file = std::fs::File::open(src_file)?;
    tar_builder.append_data(&mut header, dest_path, &mut src_file)?;
    tar_builder.into_inner().map_err(Into::into)
}

#[tracing::instrument(level = "trace")]
pub(crate) fn package(ctx: &Context) -> crate::Result<Vec<PathBuf>> {
    let Context {
        config,
        intermediates_path,
        tools_path,
        ..
    } = ctx;

    // generate the deb binary name
    let (arch, linuxdeploy_arch) = match config.target_arch()? {
        "x86" => ("i686", "i386"),
        "arm" => ("armhf", "arm"),
        other => (other, other),
    };

    let appimage_tools_path = tools_path.join("AppImage");
    std::fs::create_dir_all(&appimage_tools_path)?;

    donwload_dependencies(ctx, &appimage_tools_path, arch, linuxdeploy_arch)?;

    let appimage_deb_data_dir = intermediates_path.join("appimage_deb").join("data");
    let intermediates_path = intermediates_path.join("appimage");

    // generate deb_folder structure
    tracing::debug!("Generating data");
    let icons = super::deb::generate_data(config, &appimage_deb_data_dir)?;
    tracing::debug!("Copying files specified in `appimage.files`");
    if let Some(files) = config.appimage().and_then(|d| d.files.as_ref()) {
        super::deb::copy_custom_files(files, &appimage_deb_data_dir)?;
    }
    let icons: Vec<super::deb::DebIcon> = icons.into_iter().collect();

    let main_binary_name = config.main_binary_name()?;
    let upcase_app_name = main_binary_name.to_uppercase();
    let app_dir_path = intermediates_path.join(format!("{}.AppDir", &main_binary_name));
    let appimage_filename = format!("{}_{}_{}.AppImage", main_binary_name, config.version, arch);
    let appimage_path = config.out_dir().join(&appimage_filename);

    std::fs::create_dir_all(app_dir_path)?;

    // setup data to insert into shell script
    let mut sh_map = BTreeMap::new();
    sh_map.insert("arch", to_json(arch));
    sh_map.insert("linuxdeploy_arch", to_json(linuxdeploy_arch));
    sh_map.insert("app_name", to_json(main_binary_name));
    sh_map.insert("app_name_uppercase", to_json(upcase_app_name));
    sh_map.insert("appimage_path", to_json(&appimage_path));
    sh_map.insert(
        "packager_tools_path",
        to_json(appimage_tools_path.display().to_string()),
    );

    let libs = config
        .appimage()
        .and_then(|c| c.libs.clone())
        .unwrap_or_default();
    sh_map.insert("libs", to_json(libs));

    let bins = config
        .appimage()
        .and_then(|c| c.bins.clone())
        .unwrap_or_default();
    sh_map.insert("bins", to_json(bins));

    let linuxdeploy_plugins = config
        .appimage()
        .and_then(|a| a.linuxdeploy_plugins.clone())
        .unwrap_or_default()
        .into_keys()
        .map(|name| format!("--plugin {name}"))
        .collect::<Vec<_>>()
        .join(" ");
    sh_map.insert("linuxdeploy_plugins", to_json(linuxdeploy_plugins));

    let larger_icon = icons
        .iter()
        .filter(|i| i.width == i.height)
        .max_by_key(|i| i.width)
        .ok_or(crate::Error::AppImageSquareIcon)?;
    let larger_icon_path = larger_icon
        .path
        .strip_prefix(appimage_deb_data_dir)
        .unwrap()
        .to_string_lossy()
        .to_string();
    sh_map.insert("icon_path", to_json(larger_icon_path));

    // initialize shell script template.
    let mut handlebars = Handlebars::new();
    handlebars.register_escape_fn(handlebars::no_escape);
    handlebars
        .register_template_string("appimage", include_str!("appimage"))
        .map_err(Box::new)?;
    let template = handlebars.render("appimage", &sh_map)?;

    let sh_file = intermediates_path.join("build_appimage.sh");
    tracing::debug!(
        "Writing {} and setting its permissions to 764",
        sh_file.display()
    );
    std::fs::write(&sh_file, template)?;
    std::fs::set_permissions(&sh_file, std::fs::Permissions::from_mode(0o764))?;

    tracing::info!(
        "Packaging {} ({})",
        appimage_filename,
        appimage_path.display()
    );

    // execute the shell script to build the appimage.
    Command::new(&sh_file)
        .current_dir(intermediates_path)
        .output_ok()
        .map_err(crate::Error::AppImageScriptFailed)?;

    let appimage_targz = tar_and_gzip_file(&appimage_path)?;
    Ok(vec![appimage_path, appimage_targz])
}
