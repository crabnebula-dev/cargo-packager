use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::{
    config::{Config, ConfigExt},
    shell::CommandExt,
    sign::try_sign,
    util::create_icns_file,
};

pub fn package(config: &Config) -> crate::Result<Vec<PathBuf>> {
    // get the target path
    let output_path = config.out_dir();
    let package_base_name = format!(
        "{}_{}_{}",
        config.product_name,
        config.version,
        match config.target_arch()? {
            "x86_64" => "x64",
            other => other,
        }
    );
    let dmg_name = format!("{}.dmg", &package_base_name);
    let dmg_path = output_path.join(&dmg_name);

    log::info!(action = "Packaging"; "{} ({})", dmg_name, dmg_path.display());

    if dmg_path.exists() {
        std::fs::remove_file(&dmg_path)?;
    }

    let bundle_file_name = format!("{}.app", config.product_name);

    let support_directory_path = output_path.join("support");

    std::fs::create_dir_all(&support_directory_path)?;

    // create paths for script
    let bundle_script_path = output_path.join("bundle_dmg.sh");

    // write the scripts
    std::fs::write(&bundle_script_path, include_str!("bundle_dmg"))?;
    std::fs::write(
        support_directory_path.join("template.applescript"),
        include_str!("template.applescript"),
    )?;
    std::fs::write(
        support_directory_path.join("eula-resources-template.xml"),
        include_str!("eula-resources-template.xml"),
    )?;

    // chmod script for execution
    Command::new("chmod")
        .arg("777")
        .arg(&bundle_script_path)
        .current_dir(&output_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to chmod script");

    let mut args = vec![
        "--volname",
        &config.product_name,
        "--icon",
        &bundle_file_name,
        "180",
        "170",
        "--app-drop-link",
        "480",
        "170",
        "--window-size",
        "660",
        "400",
        "--hide-extension",
        &bundle_file_name,
    ];

    let icns_icon_path =
        create_icns_file(&output_path, config)?.map(|path| path.to_string_lossy().to_string());
    if let Some(icon) = &icns_icon_path {
        args.push("--volicon");
        args.push(icon);
    }

    // we need to keep the license path string around, `args` takes references
    #[allow(unused_assignments)]
    let mut license_path_ref = "".to_string();
    if let Some(license_path) = &config.license_file {
        args.push("--eula");
        license_path_ref = std::env::current_dir()?
            .join(license_path)
            .to_string_lossy()
            .to_string();
        args.push(&license_path_ref);
    }

    // Issue #592 - Building MacOS dmg files on CI
    // https://github.com/tauri-apps/tauri/issues/592
    if let Some(value) = std::env::var_os("CI") {
        if value == "true" {
            args.push("--skip-jenkins");
        }
    }

    log::info!(action = "Running"; "bundle_dmg.sh");

    // execute the bundle script
    Command::new(&bundle_script_path)
        .current_dir(output_path.clone())
        .args(args)
        .args(vec![dmg_name.as_str(), bundle_file_name.as_str()])
        .output_ok()?;

    std::fs::rename(output_path.join(dmg_name), dmg_path.clone())?;

    // Sign DMG if needed
    if let Some(identity) = &config
        .macos()
        .and_then(|macos| macos.signing_identity.as_ref())
    {
        try_sign(dmg_path.clone(), identity, config, false)?;
    }

    Ok(vec![dmg_path])
}
