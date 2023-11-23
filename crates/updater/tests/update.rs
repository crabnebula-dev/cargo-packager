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

const UPDATER_PRIVATE_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5VU1qSHBMT0E4R0JCVGZzbUMzb3ZXeGpGY1NSdm9OaUxaVTFuajd0T2ZKZ0FBQkFBQUFBQUFBQUFBQUlBQUFBQWlhRnNPUmxKWjBiWnJ6M29Cd0RwOUpqTW1yOFFQK3JTOGdKSi9CajlHZktHajI2ZnprbEM0VUl2MHhGdFdkZWpHc1BpTlJWK2hOTWo0UVZDemMvaFlYVUM4U2twRW9WV1JHenNzUkRKT2RXQ1FCeXlkYUwxelhacmtxOGZJOG1Nb1R6b0VEcWFLVUk9Cg==";

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
            "--verbose",
            "-f",
           &target.iter().map(|t|t.name()).collect::<Vec<_>>().join(","),
            "-c",
        ])
        .arg(format!(r#"{{"outDir":"{}","beforePackagingCommand": "cargo build", "identifier": "com.updater-app.test", "productName": "CargoPackagerAppUpdaterTest", "version": "{version}", "icons": ["32x32.png"], "binaries": [{{"path": "cargo-packager-updater-app-test", "main": true}}]}}"#, root_dir.join("target/debug").to_string_lossy().replace("\\\\?\\", "").replace('\\', "\\\\")))
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

#[cfg(target_os = "linux")]
fn package_paths(root_dir: &Path, version: &str) -> Vec<(UpdaterFormat, PathBuf)> {
    vec![(
        UpdaterFormat::AppImage,
        root_dir.join(format!(
            "target/debug/cargo-packager-updater-app-test_{version}_x86_64.AppImage"
        )),
    )]
}

#[cfg(target_os = "macos")]
fn package_paths(root_dir: &Path, _version: &str) -> Vec<(UpdaterFormat, PathBuf)> {
    vec![(
        UpdaterFormat::App,
        root_dir.join("target/debug/CargoPackagerAppUpdaterTest.app"),
    )]
}

#[cfg(windows)]
fn package_paths(root_dir: &Path, version: &str) -> Vec<(UpdaterFormat, PathBuf)> {
    vec![
        (
            UpdaterFormat::Nsis,
            root_dir.join(format!(
                "target/debug/cargo-packager-updater-app-test_{version}_x64-setup.exe"
            )),
        ),
        (
            UpdaterFormat::Wix,
            root_dir.join(format!(
                "target/debug/cargo-packager-updater-app-test_{version}_x64_en-US.msi"
            )),
        ),
    ]
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

    for (updater_format, out_package_path) in package_paths(&root_dir, "1.0.0") {
        let out_package_ext = out_package_path.extension().unwrap().to_str().unwrap();

        let out_updater_path = if out_package_path.is_dir() {
            out_package_path.with_extension(format!("{}.{}", out_package_ext, "tar.gz"))
        } else {
            out_package_path.clone()
        };

        let signature_path = out_updater_path.with_extension(format!(
            "{}.sig",
            out_updater_path.extension().unwrap().to_str().unwrap()
        ));
        let signature = std::fs::read_to_string(&signature_path).unwrap_or_else(|_| {
            panic!("failed to read signature file {}", signature_path.display())
        });

        #[cfg(target_os = "macos")]
        let updater_path = {
            // we need to move it otherwise it'll be overwritten when we build the next app
            let updater_path = out_updater_path.with_file_name(format!(
                "update-{}",
                out_updater_path.file_name().unwrap().to_str().unwrap()
            ));
            std::fs::rename(&out_updater_path, &updater_path).expect("failed to rename bundle");
            updater_path
        };
        #[cfg(not(target_os = "macos"))]
        let updater_path = out_updater_path;

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
                                    format: updater_format.name(),
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
                                File::open(&updater_path).unwrap_or_else(|_| {
                                    panic!("failed to open package {}", updater_path.display())
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
        build_app(&manifest_dir, &root_dir, "0.1.0", &[updater_format]);

        // install the app through the installer
        #[cfg(windows)]
        {
            let install_dir = root_dir
                .join("target/debug")
                .display()
                .to_string()
                .replace("\\\\?\\", "");

            let mut installer_arg = std::ffi::OsString::new();
            installer_arg.push("\"");
            installer_arg.push(
                out_package_path
                    .display()
                    .to_string()
                    .replace("\\\\?\\", ""),
            );
            installer_arg.push("\"");

            let status = Command::new("powershell.exe")
                .args(["-NoProfile", "-WindowStyle", "Hidden"])
                .args(["Start-Process"])
                .arg(installer_arg)
                .arg("-ArgumentList")
                .arg(
                    [
                        match updater_format {
                            UpdaterFormat::Wix => "/passive",
                            UpdaterFormat::Nsis => "/P",
                            _ => unreachable!(),
                        },
                        &format!(
                            "{}={}",
                            match updater_format {
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

        // wait 2secs to make sure the installer have released its lock on the binary
        std::thread::sleep(std::time::Duration::from_secs(2));

        let mut binary_cmd = if cfg!(windows) {
            Command::new(root_dir.join("target/debug/cargo-packager-updater-app-test.exe"))
        } else if cfg!(target_os = "macos") {
            Command::new(
                package_paths(&root_dir, "0.1.0")
                    .first()
                    .unwrap()
                    .1
                    .join("Contents/MacOS/cargo-packager-updater-app-test"),
            )
        } else {
            Command::new(&package_paths(&root_dir, "0.1.0").first().unwrap().1)
        };

        // This is read by the updater app test
        binary_cmd.env("UPDATER_FORMAT", updater_format.name());

        let status = binary_cmd.status().expect("failed to run app");

        if !status.success() {
            panic!("failed to run app");
        }

        // wait until the update is finished and the new version has been installed
        // before starting another updater test, this is because we use the same starting binary
        // and we can't use it while the updater is installing it
        let mut counter = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            match binary_cmd.output() {
                Ok(o) => {
                    let output = String::from_utf8_lossy(&o.stdout).to_string();
                    let version = output.split_once('\n').unwrap().0;
                    if version == "1.0.0" {
                        println!("app is updated, new version: {version}");
                        break;
                    }
                    println!("unexpected output: {output}");
                    eprintln!("stderr: {}", String::from_utf8_lossy(&o.stderr));
                }
                Err(e) => {
                    eprintln!("failed to check if app was updated: {e}");
                }
            }

            counter += 1;
            if counter == 10 {
                panic!("updater test timedout and couldn't verify the update has happened")
            }
        }

        // force a new build of the updater app test
        let _ = Command::new("cargo")
            .args(["clean", "--package", "cargo-packager-updater-app-test"])
            .current_dir(&manifest_dir)
            .output();
    }
}
