// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

pub fn main() -> Result<(), Box<dyn Error>> {
    let schema = schemars::schema_for!(cargo_packager_config::Config);
    let schema_str = serde_json::to_string_pretty(&schema).unwrap();
    let crate_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    for file in ["../packager/schema.json", "../config/schema.json"] {
        let mut schema_file = BufWriter::new(File::create(crate_dir.join(file))?);
        write!(schema_file, "{schema_str}")?;
    }
    Ok(())
}
