// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#![allow(dead_code, unused_imports)]

use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Serialize;

const UPDATER_PRIVATE_KEY: &str = include_str!("./dummy.key");

#[derive(Serialize)]
struct PlatformUpdate {
    signature: String,
    url: &'static str,
    format: &'static str,
}

#[derive(Serialize)]
struct Update {
    version: &'static str,
    date: String,
    platforms: HashMap<String, PlatformUpdate>,
}

fn build_app(cwd: &Path, root_dir: &Path, version: &str, target: &[UpdaterFormat]) {
    let mut command = Command::new("cargo");
    command
        .args([
            "run",
            "--package",
            "cargo-packager",
            "--",
            "-f",
           &target.iter().map(|t|t.name()).collect::<Vec<_>>().join(","),
            "-c",
        ])
        .arg(format!(r#"{{"outDir":"{}","beforePackagingCommand": "cargo build", "identifier": "com.updater-app.test", "productName": "CargoPackagerAppUpdaterTest", "version": "{version}", "icons": ["32x32.png"], "binaries": [{{"path": "cargo-packager-updater-app-test", "main": true}}]}}"#, dunce::simplified(&root_dir.join("target/debug")).to_string_lossy().replace('\\', "\\\\")))
        .env("CARGO_PACKAGER_SIGN_PRIVATE_KEY", UPDATER_PRIVATE_KEY)
        .env("CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD", "")
        // This is read by the updater app test
        .env("APP_VERSION", version)
        .current_dir(cwd.join("tests/app"));

    let status = command
        .status()
        .expect("failed to run cargo-packager to package app");

    if !status.code().map(|c| c == 0).unwrap_or(true) {
        panic!("failed to package app with exit code: {:?}", status.code());
    }
}

#[derive(Copy, Clone)]
enum UpdaterFormat {
    AppImage,

    App,

    Wix,
    Nsis,
}

impl UpdaterFormat {
    fn name(self) -> &'static str {
        match self {
            Self::AppImage => "appimage",
            Self::App => "app",
            Self::Wix => "wix",
            Self::Nsis => "nsis",
        }
    }

    fn default() -> &'static [Self] {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        return &[Self::App];
        #[cfg(target_os = "linux")]
        return &[Self::AppImage];
        #[cfg(windows)]
        return &[Self::Nsis, Self::Wix];
    }
}

#[test]
#[ignore]
fn update_app() {
    let target =
        cargo_packager_updater::target().expect("running updater test in an unsupported platform");

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.join("../..").canonicalize().unwrap();

    // bundle app update
    build_app(&manifest_dir, &root_dir, "1.0.0", UpdaterFormat::default());

    #[cfg(target_os = "linux")]
    let generated_packages = vec![(
        UpdaterFormat::AppImage,
        root_dir.join("target/debug/cargo-packager-updater-app-test_1.0.0_x86_64.AppImage"),
    )];
    #[cfg(target_os = "macos")]
    let generated_packages: Vec<_> = vec![(
        UpdaterFormat::App,
        root_dir.join("target/debug/CargoPackagerAppUpdaterTest.app"),
    )];
    #[cfg(windows)]
    let generated_packages: Vec<_> = vec![
        (
            UpdaterFormat::Nsis,
            root_dir.join("target/debug/cargo-packager-updater-app-test_1.0.0_x64-setup.exe"),
        ),
        (
            UpdaterFormat::Wix,
            root_dir.join("target/debug/cargo-packager-updater-app-test_1.0.0_x64_en-US.msi"),
        ),
    ];

    for (format, update_package_path) in generated_packages {
        let ext = update_package_path.extension().unwrap().to_str().unwrap();
        let signature_path = update_package_path.with_extension(format!("{ext}.sig",));
        let signature = std::fs::read_to_string(&signature_path).unwrap_or_else(|_| {
            panic!("failed to read signature file {}", signature_path.display())
        });

        // on macOS, gnerated bundle doesn't have the version in its name
        // so we need to move it otherwise it'll be overwritten when we build the next app
        #[cfg(target_os = "macos")]
        let update_package_path = {
            let filename = update_package_path.file_name().unwrap().to_str().unwrap();
            let new_path = update_package_path.with_file_name(format!("update-{filename}",));
            std::fs::rename(&update_package_path, &new_path).expect("failed to rename bundle");
            new_path
        };

        let update_package = update_package_path.clone();
        let target = target.clone();
        std::thread::spawn(move || {
            // start the updater server
            let server =
                tiny_http::Server::http("localhost:3007").expect("failed to start updater server");

            loop {
                if let Ok(request) = server.recv() {
                    match request.url() {
                        "/" => {
                            let mut platforms = HashMap::new();

                            platforms.insert(
                                target.clone(),
                                PlatformUpdate {
                                    signature: signature.clone(),
                                    url: "http://localhost:3007/download",
                                    format: format.name(),
                                },
                            );
                            let body = serde_json::to_vec(&Update {
                                version: "1.0.0",
                                date: time::OffsetDateTime::now_utc()
                                    .format(&time::format_description::well_known::Rfc3339)
                                    .unwrap(),
                                platforms,
                            })
                            .unwrap();
                            let len = body.len();
                            let response = tiny_http::Response::new(
                                tiny_http::StatusCode(200),
                                Vec::new(),
                                std::io::Cursor::new(body),
                                Some(len),
                                None,
                            );
                            let _ = request.respond(response);
                        }
                        "/download" => {
                            let _ = request.respond(tiny_http::Response::from_file(
                                File::open(&update_package).unwrap_or_else(|_| {
                                    panic!("failed to open package {}", update_package.display())
                                }),
                            ));
                            // close server
                            return;
                        }
                        _ => (),
                    }
                }
            }
        });

        // bundle initial app version
        build_app(&manifest_dir, &root_dir, "0.1.0", &[format]);

        // install the inital app on Windows to `installdir`
        #[cfg(windows)]
        {
            let install_dir = dunce::simplified(&root_dir.join("target/debug/installdir"))
                .display()
                .to_string();

            let installer_path = root_dir.join(match format {
                UpdaterFormat::Nsis => {
                    "target/debug/cargo-packager-updater-app-test_0.1.0_x64-setup.exe"
                }
                UpdaterFormat::Wix => {
                    "target/debug/cargo-packager-updater-app-test_0.1.0_x64_en-US.msi"
                }
                _ => unreachable!(),
            });
            let installer_path = dunce::simplified(&installer_path);
            dbg!(installer_path);

            let mut installer_arg = std::ffi::OsString::new();
            installer_arg.push("\"");
            installer_arg.push(installer_path.display().to_string());
            installer_arg.push("\"");

            let status = Command::new("powershell.exe")
                .args(["-NoProfile", "-WindowStyle", "Hidden"])
                .args(["Start-Process"])
                .arg(installer_arg)
                .arg("-Wait")
                .arg("-ArgumentList")
                .arg(
                    [
                        match format {
                            UpdaterFormat::Wix => "/passive",
                            UpdaterFormat::Nsis => "/P",
                            _ => unreachable!(),
                        },
                        &format!(
                            "{}={}",
                            match format {
                                UpdaterFormat::Wix => "INSTALLDIR",
                                UpdaterFormat::Nsis => "/D",
                                _ => unreachable!(),
                            },
                            install_dir
                        ),
                    ]
                    .join(", "),
                )
                .status()
                .expect("failed to run installer");

            if !status.success() {
                panic!("failed to run installer");
            }
        }

        #[cfg(windows)]
        let app = root_dir.join("target/debug/installdir/cargo-packager-updater-app-test.exe");
        #[cfg(target_os = "linux")]
        let app =
            root_dir.join("target/debug/cargo-packager-updater-app-test_0.1.0_x86_64.AppImage");
        #[cfg(target_os = "macos")]
        let app = root_dir.join("target/debug/CargoPackagerAppUpdaterTest.app/Contents/MacOS/cargo-packager-updater-app-test");

        // save the current creation time
        let ctime1 = std::fs::metadata(&app)
            .expect("failed to read app metadata")
            .created()
            .unwrap();

        // run initial app
        Command::new(&app)
            // This is read by the updater app test
            .env("UPDATER_FORMAT", format.name())
            .status()
            .expect("failed to start initial app");

        // wait until the update is finished and the new version has been installed
        // before starting another updater test, this is because we use the same starting binary
        // and we can't use it while the updater is installing it
        let mut counter = 0;
        loop {
            // check if the main binary creation time has changed since `ctime1`
            let ctime2 = std::fs::metadata(&app)
                .expect("failed to read app metadata")
                .created()
                .unwrap();
            dbg!(ctime1, ctime2);
            if ctime1 != ctime2 {
                match Command::new(&app).output() {
                    Ok(o) => {
                        let output = String::from_utf8_lossy(&o.stdout).to_string();
                        let version = output.split_once('\n').unwrap().0;
                        if version == "1.0.0" {
                            println!("app is updated, new version: {version}");
                            break;
                        }
                        println!("unexpected output (stdout): {output}");
                        eprintln!("stderr: {}", String::from_utf8_lossy(&o.stderr));
                    }
                    Err(e) => {
                        eprintln!("failed to check if app was updated: {e}");
                    }
                }
            }

            counter += 1;
            if counter == 10 {
                panic!("updater test timedout and couldn't verify the update has happened")
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
        }

        // force a new build of the updater app test
        // so `APP_VERSION` env arg would be embedded correctly
        // for the next format test
        let _ = Command::new("cargo")
            .args(["clean", "--package", "cargo-packager-updater-app-test"])
            .current_dir(&manifest_dir)
            .output();
    }
}
