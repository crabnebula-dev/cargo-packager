use std::{
    collections::BTreeMap,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

use handlebars::{to_json, Handlebars};

use crate::{
    config::{Config, ConfigExt, ConfigExtInternal},
    util,
};

fn donwload_dependencies(
    config: &Config,
    packager_tools_path: &Path,
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

    let user_deps = config
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
        let path = packager_tools_path.join(path);
        if !path.exists() {
            let data = util::download(&url)?;
            log::debug!(action = "Writing"; "{} and setting its permissions to 764", path.display());
            std::fs::write(&path, data)?;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o764))?;
        }
    }

    Ok(())
}

pub fn package(config: &Config) -> crate::Result<Vec<PathBuf>> {
    // generate the deb binary name
    let (arch, linuxdeploy_arch) = match config.target_arch()? {
        "x86" => ("i686", "i386"),
        "arm" => ("armhf", "arm"),
        other => (other, other),
    };

    let out_dir = config.out_dir();
    let output_path = out_dir.join("appimage");
    if output_path.exists() {
        std::fs::remove_dir_all(&output_path)?;
    }
    std::fs::create_dir_all(&output_path)?;

    let packager_tools_path = dirs::cache_dir()
        .map(|p| p.join("cargo-pacakger"))
        .unwrap_or_else(|| output_path.clone());
    std::fs::create_dir_all(&packager_tools_path)?;

    donwload_dependencies(config, &packager_tools_path, arch, linuxdeploy_arch)?;

    let appimage_deb_dir = config.out_dir().join("appimage_deb");

    // generate deb_folder structure
    log::debug!("generating data");
    let (_, icons) = super::deb::generate_data(config, &appimage_deb_dir)?;
    let icons: Vec<super::deb::DebIcon> = icons.into_iter().collect();

    let main_binary_name = config.main_binary_name()?;
    let upcase_app_name = main_binary_name.to_uppercase();
    let app_dir_path = output_path.join(format!("{}.AppDir", &main_binary_name));
    let appimage_filename = format!("{}_{}_{}.AppImage", main_binary_name, config.version, arch);
    let appimage_path = out_dir.join(&appimage_filename);

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
        to_json(packager_tools_path.display().to_string()),
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
        .strip_prefix(appimage_deb_dir.join("data"))
        .unwrap()
        .to_string_lossy()
        .to_string();
    sh_map.insert("icon_path", to_json(larger_icon_path));

    // initialize shell script template.
    let mut handlebars = Handlebars::new();
    handlebars.register_escape_fn(handlebars::no_escape);
    handlebars
        .register_template_string("appimage", include_str!("appimage"))
        .expect("Failed to register template for handlebars");
    let template = handlebars.render("appimage", &sh_map)?;
    let sh_file = output_path.join("build_appimage.sh");

    log::debug!(action = "Writing"; "{template} and setting its permissions to 764");
    std::fs::write(&sh_file, template)?;
    std::fs::set_permissions(&sh_file, std::fs::Permissions::from_mode(0o764))?;

    log::info!(action = "Packaging"; "{} ({})", appimage_filename, appimage_path.display());

    // execute the shell script to build the appimage.
    Command::new(&sh_file)
        .current_dir(output_path)
        .output()
        .expect("error running appimage.sh");

    std::fs::remove_dir_all(&appimage_deb_dir)?;
    Ok(vec![appimage_path])
}
