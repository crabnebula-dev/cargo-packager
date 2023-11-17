// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

fn main() {
    #[allow(clippy::option_env_unwrap)]
    let version = option_env!("APP_VERSION").unwrap();
    let mut builder = cargo_packager_updater::UpdaterBuilder::new(
        version.parse().unwrap(),
        cargo_packager_updater::Config {
            pubkey: "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDQ2Njc0OTE5Mzk2Q0ExODkKUldTSm9XdzVHVWxuUmtJdjB4RnRXZGVqR3NQaU5SVitoTk1qNFFWQ3pjL2hZWFVDOFNrcEVvVlcK".into(),
            endpoints: vec!["http://localhost:3007".parse().unwrap()],
            ..Default::default()
        },
    );
    let format = std::env::var("UPDATER_FORMAT").unwrap_or_default();

    match format.as_str() {
        "nsis" => {
            // /D sets the default installation directory ($INSTDIR),
            // overriding InstallDir and InstallDirRegKey.
            // It must be the last parameter used in the command line and must not contain any quotes, even if the path contains spaces.
            // Only absolute paths are supported.
            // NOTE: we only need this because this is an integration test and we don't want to install the app in the programs folder
            builder = builder.installer_args(vec![format!(
                "/D={}",
                std::env::current_exe().unwrap().parent().unwrap().display()
            )]);
        }
        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        "appimage" => {
            if let Some(p) = std::env::var_os("APPIMAGE") {
                builder = builder.executable_path(p);
            }
        }
        _ => {}
    }

    let updater = builder.build().unwrap();

    println!("{version}");

    match updater.check() {
        Ok(Some(update)) => {
            if let Err(e) = update.download_and_install(|_, _| {}, || {}) {
                println!("{e}");
                std::process::exit(1);
            }

            std::process::exit(0);
        }
        Ok(None) => {
            std::process::exit(0);
        }
        Err(e) => {
            println!("{e}");
            std::process::exit(1);
        }
    }
}
