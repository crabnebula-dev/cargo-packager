// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

use handlebars::{to_json, Handlebars};

use super::{deb, Context};
use crate::{shell::CommandExt, util, Error};

#[tracing::instrument(level = "trace", skip(ctx))]
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
            fs::write(&path, data).map_err(|e| Error::IoWithPath(path.clone(), e))?;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o764))
                .map_err(|e| Error::IoWithPath(path, e))?;
        }
    }

    Ok(())
}

#[tracing::instrument(level = "trace", skip(ctx))]
pub(crate) fn package(ctx: &Context) -> crate::Result<Vec<PathBuf>> {
    let Context {
        config,
        intermediates_path,
        tools_path,
        ..
    } = ctx;

    let mut config = config.clone();
    let main_binary_name = config.main_binary_name()?;

    // if binary file name contains spaces, we must change it to kebab-case
    if main_binary_name.contains(' ') {
        let main_binary = config.main_binary_mut()?;

        let main_binary_name_kebab = heck::AsKebabCase(main_binary_name).to_string();
        let new_path = intermediates_path.join(&main_binary_name_kebab);
        fs::copy(&main_binary.path, &new_path)?;

        main_binary.path = new_path;
    }

    // generate the deb binary name
    let (arch, linuxdeploy_arch) = match config.target_arch()? {
        "x86" => ("i686", "i386"),
        "arm" => ("armhf", "arm"),
        other => (other, other),
    };

    let appimage_tools_path = tools_path.join("AppImage");
    fs::create_dir_all(&appimage_tools_path)
        .map_err(|e| Error::IoWithPath(appimage_tools_path.clone(), e))?;

    donwload_dependencies(ctx, &appimage_tools_path, arch, linuxdeploy_arch)?;

    let appimage_deb_data_dir = intermediates_path.join("appimage_deb").join("data");
    let intermediates_path = intermediates_path.join("appimage");

    // generate deb_folder structure
    tracing::debug!("Generating data");
    let icons = deb::generate_data(&config, &appimage_deb_data_dir)?;
    tracing::debug!("Copying files specified in `appimage.files`");
    if let Some(files) = config.appimage().and_then(|d| d.files.as_ref()) {
        deb::copy_custom_files(files, &appimage_deb_data_dir)?;
    }
    let icons: Vec<deb::DebIcon> = icons.into_iter().collect();

    let main_binary_name = config.main_binary_name()?;
    let upcase_app_name = main_binary_name.to_uppercase();
    let app_dir_path = intermediates_path.join(format!("{}.AppDir", &main_binary_name));
    let appimage_filename = format!("{}_{}_{}.AppImage", main_binary_name, config.version, arch);
    let appimage_path = config.out_dir().join(&appimage_filename);

    fs::create_dir_all(&app_dir_path).map_err(|e| Error::IoWithPath(app_dir_path.clone(), e))?;

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

    let excluded_libraries = config
        .appimage()
        .and_then(|a| a.excluded_libs.clone())
        .unwrap_or_default()
        .into_iter()
        .map(|library| format!("--exclude-library {library}"))
        .collect::<Vec<_>>()
        .join(" ");
    sh_map.insert("excluded_libs", to_json(excluded_libraries));

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
    fs::write(&sh_file, template).map_err(|e| Error::IoWithPath(sh_file.clone(), e))?;
    fs::set_permissions(&sh_file, fs::Permissions::from_mode(0o764))
        .map_err(|e| Error::IoWithPath(sh_file.clone(), e))?;

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

    Ok(vec![appimage_path])
}
