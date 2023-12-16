use std::env;

fn main() {
    track_var("CARGO_PACKAGER_FORMAT");
    track_var("CARGO_PACKAGER_MAIN_BINARY_NAME");
}

fn track_var(key: &str) {
    println!("cargo:rerun-if-env-changed={}", key);
    if let Ok(var) = env::var(key) {
        println!("cargo:rustc-cfg={}=\"{}\"", key, var);
    }
}
