# cargo-packager

<img src=".github/splash.png" alt="cargo-packager splash" />

Rust executable packager, bundler and updater. A tool and library to generate installers or app bundles for your executables.
It also has a comptabile updater through [cargo-packager-updater](./crates/updater/).

## CLI

### Installation

```sh
cargo install cargo-packager --locked
```

### Usage

1. Add `Packager.toml` or `packager.json` in your project or modify Cargo.toml and include

   ```toml
   [package.metadata.packager]
   before-packaging-command = "cargo build --release"
   ```

2. Run the CLI

   ```sh
   cargo packager --release
   ```

### Supported packages

- macOS
  - DMG (.dmg)
  - Bundle (.app)
- Linux
  - Debian package (.deb)
  - AppImage (.AppImage)
- Windows
  - NSIS (.exe)
  - MSI using WiX Toolset (.msi)

### Configuration

By default, `cargo-packager` reads a configuration from `Packager.toml` or `packager.json` if it exists, and from `package.metadata.packager` table in `Cargo.toml`.
You can also specify a custom configuration file using the `-c/--config` cli argument.
All configuration options could be either a single config or array of configs.

For a full list of configuration options, see https://docs.rs/cargo-packager/latest/cargo_packager/config/struct.Config.html

You could also use the schema from GitHub releases to validate your configuration or have auto completions turned on in your IDE.

### Building your application before packaging

By default, `cargo-packager` doesn't build your application, it only looks for it inside the directory specified in `config.out_dir` or `--out-dir` cli arg,
However, `cargo-packager` has an option to specify a shell command to be executed before packaing your app, `beforePackagingCommand`.

### Cargo profiles

By default, `cargo-packager` looks for binaries built using the `debug` profile, if your `beforePackagingCommand` builds your app using `cargo build --release`, you will also need to
run `cargo-packager` in release mode `cargo packager --release`, otherwise, if you have a custom cargo profile, you will need to specify it using `--profile` cli arg `cargo packager --profile custom-release-profile`.

For more information, checkout the available [configuration options](https://docs.rs/cargo-packager/latest/cargo_packager/config/struct.Config.html) and for a list of available CLI
commands and arguments, run `cargo packager --help`.

### Examples

The [`examples`](./examples/) directory contains a number of varying examples, if you want to build them all run `cargo r -p cargo-packager -- --release` in the root of this repository. Just make sure to have the tooling for each example installed on your system. You can find what tooling they require by checking the README in each example. The README also contains a command to build this example alone if you wish.

Examples list (non-exhaustive):

- [`tauri`](./examples/tauri/)
- [`wry`](./examples/wry/)
- [`dioxus`](./examples/dioxus/)
- [`egui`](./examples/egui/)
- [`deno`](./examples/deno/)
- [`slint`](./examples/slint/)
- [`wails`](./examples/wails)

## Library

This crate is also published to crates.io as a library that you can integrate into your tooling, just make sure to disable the default-feature flags.

```sh
cargo add cargo-packager --no-default-features
```

#### Feature flags

- **`cli`**: Enables the CLI specifc features and dependencies. Enabled by default.
- **`tracing`**: Enables `tracing` crate integration.

## Licenses

MIT or MIT/Apache 2.0 where applicable.
