use std::{os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

use crate::{
    config::ConfigExt,
    shell::CommandExt,
    sign,
    util::{self, download},
    Context,
};

const CREATE_DMG_URL: &str =
    "https://raw.githubusercontent.com/create-dmg/create-dmg/28867ba3563ddef62f55dcf130677103b4296c42/create-dmg";

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

    log::info!(action = "Packaging"; "{} ({})", dmg_name, dmg_path.display());

    if dmg_path.exists() {
        std::fs::remove_file(&dmg_path)?;
    }

    let dmg_tools_path = tools_path.join("DMG");

    let script_dir = dmg_tools_path.join("script");
    std::fs::create_dir_all(&script_dir)?;

    let create_dmg_script_path = script_dir.join("create-dmg");

    let support_directory_path = dmg_tools_path.join("share/create-dmg/support");
    std::fs::create_dir_all(&support_directory_path)?;

    if !dmg_tools_path.exists() {
        std::fs::create_dir_all(&dmg_tools_path)?;
    }
    if !create_dmg_script_path.exists() {
        log::debug!("Downloading create-dmg script");
        let data = download(CREATE_DMG_URL)?;
        log::debug!(action = "Writing"; "{} and setting its permissions to 764", create_dmg_script_path.display());
        std::fs::write(&create_dmg_script_path, data)?;
        std::fs::set_permissions(
            &create_dmg_script_path,
            std::fs::Permissions::from_mode(0o764),
        )?;
    }

    log::debug!(action = "Writing"; "template.applescript");
    std::fs::write(
        support_directory_path.join("template.applescript"),
        include_str!("template.applescript"),
    )?;

    log::debug!(action = "Writing"; "eula-resources-template.xml");
    std::fs::write(
        support_directory_path.join("eula-resources-template.xml"),
        include_str!("eula-resources-template.xml"),
    )?;

    let mut args = vec![
        "--volname",
        &config.product_name,
        "--icon",
        &app_bundle_file_name,
        "180",
        "170",
        "--app-drop-link",
        "480",
        "170",
        "--window-size",
        "660",
        "400",
        "--hide-extension",
        &app_bundle_file_name,
    ];

    let icns_icon_path = util::create_icns_file(&intermediates_path, config)?
        .map(|path| path.to_string_lossy().to_string());
    if let Some(icon) = &icns_icon_path {
        args.push("--volicon");
        args.push(icon);
    }

    let license_file = config.license_file.as_ref().map(|l| {
        std::env::current_dir()
            .unwrap()
            .join(l)
            .to_string_lossy()
            .to_string()
    });
    if let Some(license_path) = &license_file {
        args.push("--eula");
        args.push(license_path.as_str());
    }

    // Issue #592 - Building MacOS dmg files on CI
    // https://github.com/tauri-apps/tauri/issues/592
    if let Some(value) = std::env::var_os("CI") {
        if value == "true" {
            args.push("--skip-jenkins");
        }
    }

    log::info!(action = "Running"; "create-dmg");

    // execute the bundle script
    Command::new(&create_dmg_script_path)
        .current_dir(&out_dir)
        .args(args)
        .args(vec![dmg_name.as_str(), app_bundle_file_name.as_str()])
        .output_ok()?;

    // Sign DMG if needed
    if let Some(identity) = &config
        .macos()
        .and_then(|macos| macos.signing_identity.as_ref())
    {
        sign::try_sign(&dmg_path, identity, config, false)?;
    }

    Ok(vec![dmg_path])
}
