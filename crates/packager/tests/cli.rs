// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#![cfg(all(feature = "cli", target_os = "linux"))]

use std::{fs, process::Command};

#[test]
fn before_packaging_hook_can_create_cli_binaries_dir() {
    let project = tempfile::tempdir().unwrap();
    let config_dir = project.path().join("packaging");
    fs::create_dir(&config_dir).unwrap();
    fs::write(
        config_dir.join("packager.json"),
        serde_json::json!({
            "productName": "example",
            "version": "1.0.0",
            "identifier": "com.example.app",
            "binaries": [{"path": "example", "main": true}],
            "beforePackagingCommand": "mkdir -p ../target/custom && touch ../target/custom/example"
        })
        .to_string(),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_cargo-packager"))
        .current_dir(project.path())
        .args([
            "--config",
            "packaging/packager.json",
            "--binaries-dir",
            "target/custom",
            "--out-dir",
            "dist",
            "--formats",
            "deb",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "cargo-packager failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fs::read_dir(project.path().join("dist"))
        .unwrap()
        .any(|entry| entry
            .unwrap()
            .path()
            .extension()
            .is_some_and(|ext| ext == "deb")));
}
