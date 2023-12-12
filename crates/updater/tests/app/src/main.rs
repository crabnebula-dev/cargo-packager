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
            pubkey: include_str!("../../dummy.pub.key").into(),
            endpoints: vec!["http://localhost:3007".parse().unwrap()],
            ..Default::default()
        },
    );
    let format = std::env::var("UPDATER_FORMAT").unwrap_or_default();

    match format.as_str() {
        "nsis" | "wix" => {
            // NOTE: we only need this because this is an integration test and we don't want to install the app in the programs folder
            builder = builder.installer_args(vec![format!(
                "{}={}",
                if format == "nsis" { "/D" } else { "INSTALLDIR" },
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
            if let Err(e) = update.download_and_install() {
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
