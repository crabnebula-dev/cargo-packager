# cargo-packager

<img src=".github/splash.png" alt="cargo-packager splash" />

Executable packager, bundler and updater. A cli tool and library to generate installers or app bundles for your executables.
It also comes with useful addons:

- an [updater](https://docs.rs/cargo-packager-updater)
- a [resource resolver](https://docs.rs/cargo-packager-resource-resolver)

#### Supported packages:

- macOS
  - DMG (.dmg)
  - Bundle (.app)
- Linux
  - Debian package (.deb)
  - AppImage (.AppImage)
- Windows
  - NSIS (.exe)
  - MSI using WiX Toolset (.msi)

### CLI

The packager is distributed on crates.io as a cargo subcommand, you can install it using cargo:

```sh
cargo install cargo-packager --locked
```

You then need to configure your app so the cli can recognize it. Configuration can be done in `Packager.toml` or `packager.json` in your project or modify Cargo.toml and include this snippet:

```toml
[package.metadata.packager]
before-packaging-command = "cargo build --release"
```

Once, you are done configuring your app, run:

```sh
cargo packager --release
```

### Configuration

By default, the packager reads its configuration from `Packager.toml` or `packager.json` if it exists, and from `package.metadata.packager` table in `Cargo.toml`.
You can also specify a custom configuration using the `-c/--config` cli argument.

For a full list of configuration options, see https://docs.rs/cargo-packager/latest/cargo_packager/config/struct.Config.html.

You could also use the [schema](./schema.json) file from GitHub to validate your configuration or have auto completions in your IDE.

### Building your application before packaging

By default, the packager doesn't build your application, so if your app requires a compilation step, the packager has an option to specify a shell command to be executed before packaing your app, `beforePackagingCommand`.

### Cargo profiles

By default, the packager looks for binaries built using the `debug` profile, if your `beforePackagingCommand` builds your app using `cargo build --release`, you will also need to
run the packager in release mode `cargo packager --release`, otherwise, if you have a custom cargo profile, you will need to specify it using `--profile` cli arg `cargo packager --profile custom-release-profile`.

### Library

This crate is also published to crates.io as a library that you can integrate into your tooling, just make sure to disable the default-feature flags.

```sh
cargo add cargo-packager --no-default-features
```

#### Feature flags

- **`cli`**: Enables the cli specifc features and dependencies. Enabled by default.
- **`tracing`**: Enables `tracing` crate integration.

## Licenses

MIT or MIT/Apache 2.0 where applicable.
