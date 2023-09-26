# cargo-packager

Rust executable packager, bundler and updater.

## Installation

```
cargo install cargo-pacakger --locked
```

## Usage

```
cargo pacakger
```

## Configuration

By default, `cargo-pacakger` reads configuration from `Packager.toml` or `pacakger.json` if exists, and from `package.metadata.packager` table in `Cargo.toml`.
You can specify a custom configuration file using `-c/--config` flag. All configuration options could be either a single config or array of configs.

For full list of configuration options, see https://docs.rs/cargo-packager/latest/cargo-packager/struct.Config.html

You could also use the schema from GitHub releases to validate your configuration or have auto completions in your IDE.
