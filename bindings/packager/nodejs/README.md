# @crabnebula/packager

Executable packager, bundler and updater. A cli tool and library to generate installers or app bundles for your executables.
It also has a compatible updater through [@crabnebula/updater](https://www.npmjs.com/package/@crabnebula/updater).

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

## Rust

### CLI

The packager is distrubuted on NPM as a CLI, you can install it:

```sh
# pnpm
pnpm add -D @crabnebula/packager
# pnpm
yarn add -D @crabnebula/packager
# npm
npm i -D @crabnebula/packager
```

You then need to configure your app so the CLI can recognize it.
Configuration can be done in `Packager.toml` or `packager.json` in your project or `packager` key in `packager.json`
Once, you are done configuring your app, run:

```sh
# pnpm
pnpm packager
# pnpm
yarn packager
# npm
npx packager
```

### Configuration

By default, the packager reads its configuration from `Packager.toml` or `packager.json` if it exists, and from `packager.json` keyin `packager.json`,
You can also specify a custom configuration using the `-c/--config` cli argument.

For a full list of configuration options, see https://docs.rs/cargo-packager/latest/cargo_packager/config/struct.Config.html.

You could also use the [schema](./schema.json) file from GitHub to validate your configuration or have auto completions in your IDE.

### Building your application before packaging

By default, the packager doesn't build your application, so if your app requires a compilation step, the packager has an option to specify a shell command to be executed before packaing your app, `beforePackagingCommand`.

### Library

The packager is also a library that you can import and integrate into your tooling.

## Licenses

MIT or MIT/Apache 2.0 where applicable.
