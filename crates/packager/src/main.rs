fn main() {
    if let Err(e) = cargo_packager::cli::try_run() {
        log::error!("{}", e);
        std::process::exit(1);
    }
}
