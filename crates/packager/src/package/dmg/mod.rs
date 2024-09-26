// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

use super::Context;
use crate::{
    codesign::macos as codesign,
    shell::CommandExt,
    util::{self, download},
    Error,
};

const CREATE_DMG_URL: &str =
    "https://raw.githubusercontent.com/create-dmg/create-dmg/28867ba3563ddef62f55dcf130677103b4296c42/create-dmg";

#[tracing::instrument(level = "trace", skip(ctx))]
pub(crate) fn package(ctx: &Context) -> crate::Result<Vec<PathBuf>> {
    let Context {
        config,
        tools_path,
        intermediates_path,
        ..
    } = ctx;

    let out_dir = config.out_dir();
    let intermediates_path = intermediates_path.join("dmg");
    util::create_clean_dir(&intermediates_path)?;

    let package_base_name = format!(
        "{}_{}_{}",
        config.product_name,
        config.version,
        match config.target_arch()? {
            "x86_64" => "x64",
            other => other,
        }
    );
    let app_bundle_file_name = format!("{}.app", config.product_name);
    let dmg_name = format!("{}.dmg", &package_base_name);
    let dmg_path = out_dir.join(&dmg_name);

    tracing::info!("Packaging {} ({})", dmg_name, dmg_path.display());

    if dmg_path.exists() {
        fs::remove_file(&dmg_path).map_err(|e| Error::IoWithPath(dmg_path.clone(), e))?;
    }

    let dmg_tools_path = tools_path.join("DMG");

    let script_dir = dmg_tools_path.join("script");
    fs::create_dir_all(&script_dir).map_err(|e| Error::IoWithPath(script_dir.clone(), e))?;

    let create_dmg_script_path = script_dir.join("create-dmg");

    let support_directory_path = dmg_tools_path.join("share/create-dmg/support");
    fs::create_dir_all(&support_directory_path)
        .map_err(|e| Error::IoWithPath(support_directory_path.clone(), e))?;

    if !dmg_tools_path.exists() {
        fs::create_dir_all(&dmg_tools_path)
            .map_err(|e| Error::IoWithPath(dmg_tools_path.clone(), e))?;
    }
    if !create_dmg_script_path.exists() {
        tracing::debug!("Downloading create-dmg script");
        let data = download(CREATE_DMG_URL)?;
        tracing::debug!(
            "Writing {} and setting its permissions to 764",
            create_dmg_script_path.display()
        );
        fs::write(&create_dmg_script_path, data)
            .map_err(|e| Error::IoWithPath(create_dmg_script_path.clone(), e))?;
        fs::set_permissions(&create_dmg_script_path, fs::Permissions::from_mode(0o764))
            .map_err(|e| Error::IoWithPath(create_dmg_script_path.clone(), e))?;
    }

    tracing::debug!("Writing template.applescript");
    let template_applescript = support_directory_path.join("template.applescript");
    fs::write(&template_applescript, include_str!("template.applescript"))
        .map_err(|e| Error::IoWithPath(template_applescript, e))?;

    tracing::debug!("Writing eula-resources-template.xml");
    let eula_template = support_directory_path.join("eula-resources-template.xml");
    fs::write(eula_template, include_str!("eula-resources-template.xml"))
        .map_err(|e| Error::IoWithPath(eula_template, e))?;

    let dmg = config.dmg();

    let mut bundle_dmg_cmd = Command::new(&create_dmg_script_path);

    let app_x = dmg
        .and_then(|d| d.app_position)
        .map(|p| p.x)
        .unwrap_or(180)
        .to_string();
    let app_y = dmg
        .and_then(|d| d.app_position)
        .map(|p| p.y)
        .unwrap_or(170)
        .to_string();
    let app_folder_x = dmg
        .and_then(|d| d.app_folder_position)
        .map(|p| p.x)
        .unwrap_or(480)
        .to_string();
    let app_folder_y = dmg
        .and_then(|d| d.app_folder_position)
        .map(|p| p.y)
        .unwrap_or(170)
        .to_string();
    let window_width = dmg
        .and_then(|d| d.window_size)
        .map(|s| s.width)
        .unwrap_or(600)
        .to_string();
    let window_height = dmg
        .and_then(|d| d.window_size)
        .map(|s| s.height)
        .unwrap_or(400)
        .to_string();

    bundle_dmg_cmd.args([
        "--volname",
        &config.product_name,
        "--icon",
        &app_bundle_file_name,
        &app_x,
        &app_y,
        "--app-drop-link",
        &app_folder_x,
        &app_folder_y,
        "--window-size",
        &window_width,
        &window_height,
        "--hide-extension",
        &app_bundle_file_name,
    ]);

    let window_position = dmg
        .and_then(|d| d.window_position)
        .map(|p| (p.x.to_string(), p.y.to_string()));
    if let Some((x, y)) = window_position {
        bundle_dmg_cmd.arg("--window-pos");
        bundle_dmg_cmd.arg(&x);
        bundle_dmg_cmd.arg(&y);
    }

    let background_path = match &dmg.and_then(|d| d.background.as_ref()) {
        Some(p) => Some(std::env::current_dir()?.join(p)),
        None => None,
    };

    if let Some(background_path) = &background_path {
        bundle_dmg_cmd.arg("--background");
        bundle_dmg_cmd.arg(background_path);
    }

    tracing::debug!("Creating icns file");
    let icns_icon_path = util::create_icns_file(&intermediates_path, config)?;
    if let Some(icon) = &icns_icon_path {
        bundle_dmg_cmd.arg("--volicon");
        bundle_dmg_cmd.arg(icon);
    }

    let license_file = match config.license_file.as_ref() {
        Some(l) => Some(std::env::current_dir()?.join(l)),
        None => None,
    };
    if let Some(license_path) = &license_file {
        bundle_dmg_cmd.arg("--eula");
        bundle_dmg_cmd.arg(license_path);
    }

    // Issue #592 - Building MacOS dmg files on CI
    // https://github.com/tauri-apps/tauri/issues/592
    if let Some(value) = std::env::var_os("CI") {
        if value == "true" {
            bundle_dmg_cmd.arg("--skip-jenkins");
        }
    }

    tracing::debug!("Running create-dmg");

    // execute the bundle script
    bundle_dmg_cmd
        .current_dir(&out_dir)
        .args(vec![dmg_name.as_str(), app_bundle_file_name.as_str()])
        .output_ok()
        .map_err(crate::Error::CreateDmgFailed)?;

    // Sign DMG if needed
    if let Some(identity) = &config
        .macos()
        .and_then(|macos| macos.signing_identity.as_ref())
    {
        tracing::debug!("Codesigning {}", dmg_path.display());
        codesign::try_sign(
            vec![codesign::SignTarget {
                path: dmg_path.clone(),
                is_native_binary: false,
            }],
            identity,
            config,
        )?;
    }

    Ok(vec![dmg_path])
}
