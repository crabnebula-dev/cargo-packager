use std::env;

fn main() {
    if env::var("CARGO_FEATURE_AUTO_DETECT_FORMAT").is_ok() {
        println!("cargo:rerun-if-env-changed=CARGO_PACKAGER_FORMAT");
        if let Ok(var) = env::var("CARGO_PACKAGER_FORMAT") {
            println!("cargo:rustc-check-cfg=CARGO_PACKAGER_FORMAT=\"{}\"", var);
            println!("cargo:rustc-cfg=CARGO_PACKAGER_FORMAT=\"{}\"", var);
        }
    }
}
