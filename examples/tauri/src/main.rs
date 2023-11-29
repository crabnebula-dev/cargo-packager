// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cargo_packager_updater::{Config, Update, UpdaterBuilder};
use tauri::{AppHandle, Manager, Runtime};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

const UPDATER_PUB_KEY: &str = include_str!("../dummy.pub.key");
const UPDATER_ENDPOINT: &str = "http://localhost:2342";

#[tauri::command]
fn check_update<R: Runtime>(app: AppHandle<R>) -> Result<(bool, Option<String>), ()> {
    let config = Config {
        pubkey: UPDATER_PUB_KEY.into(),
        endpoints: vec![UPDATER_ENDPOINT.parse().unwrap()],
        ..Default::default()
    };

    let updater = {
        #[allow(unused_mut)]
        let mut updater_builder = UpdaterBuilder::new(app.package_info().version.clone(), config);
        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd"
        ))]
        {
            if let Some(appimage) = app.env().appimage {
                updater_builder = updater_builder.executable_path(appimage)
            }
        }
        updater_builder.build().unwrap()
    };

    let update = updater.check().unwrap();
    let has_update = update.is_some();
    let version = update.as_ref().map(|u| u.version.clone());
    if let Some(update) = update {
        app.manage(update);
    }

    Ok((has_update, version))
}

struct UpdateBytes(Vec<u8>);

#[derive(serde::Serialize, Clone)]
struct ProgressPayload {
    chunk_len: usize,
    content_len: Option<u64>,
}
#[tauri::command]
fn download_update<R: Runtime>(app: AppHandle<R>) -> Result<(), ()> {
    let app_1 = app.clone();
    std::thread::spawn(move || {
        let update = app.state::<Update>();
        let update_bytes = update
            .download(
                move |chunk_len, content_len| {
                    app_1
                        .emit_all(
                            "update_progress",
                            ProgressPayload {
                                chunk_len,
                                content_len,
                            },
                        )
                        .unwrap();
                },
                move || {},
            )
            .unwrap();
        app.manage(UpdateBytes(update_bytes));
    });
    Ok(())
}
#[tauri::command]
fn install_update<R: Runtime>(app: AppHandle<R>) -> Result<(), ()> {
    let update = app.state::<Update>();
    let update_bytes = app.state::<UpdateBytes>();
    update.install(update_bytes.0.clone()).unwrap();
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            greet,
            version,
            check_update,
            download_update,
            install_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
