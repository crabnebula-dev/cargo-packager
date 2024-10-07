# Changelog

## \[0.2.2]

- [`af990f8`](https://www.github.com/crabnebula-dev/cargo-packager/commit/af990f848b78fa07fe2aa8f4cc32599557af9bf7) ([#281](https://www.github.com/crabnebula-dev/cargo-packager/pull/281)) Relax `url` dependency version requirement from `2.5` to `2`.

## \[0.2.1]

- [`2b6dd55`](https://www.github.com/crabnebula-dev/cargo-packager/commit/2b6dd55eac6733715a4f717af54ff167e1fdcdf8) ([#266](https://www.github.com/crabnebula-dev/cargo-packager/pull/266)) Fix `process-relaunch-dangerous-allow-symlink-macos` feature usage.

### Dependencies

- Upgraded to `cargo-packager-utils@0.1.1`

## \[0.2.0]

- [`c16d17a`](https://www.github.com/crabnebula-dev/cargo-packager/commit/c16d17ae190f49be3f9e78c5441bee16c0f8fc69) Enable `rustls-tls` feature flag by default.

## \[0.1.4]

- [`3ee2290`](https://www.github.com/crabnebula-dev/cargo-packager/commit/3ee2290df518103056b295dae426b38a65293048)([#147](https://www.github.com/crabnebula-dev/cargo-packager/pull/147)) Prevent powershell window from opening when the msi and nsis installer are executed.

## \[0.1.3]

- [`0e00ca2`](https://www.github.com/crabnebula-dev/cargo-packager/commit/0e00ca25fc0e71cad4bb7085edda067a184e5ec7)([#146](https://www.github.com/crabnebula-dev/cargo-packager/pull/146)) Enable native certificates via `rustls-native-certs`.

## \[0.1.2]

### Dependencies

- Upgraded to `cargo-packager-utils@0.1.0`

## \[0.1.1]

- [`feb53a2`](https://www.github.com/crabnebula-dev/cargo-packager/commit/feb53a2f16ef2c8d93ff2d73a4eb318490f33471)([#102](https://www.github.com/crabnebula-dev/cargo-packager/pull/102)) Fix NSIS updater failing to launch when using `basicUi` mode.
- [`e58c7e2`](https://www.github.com/crabnebula-dev/cargo-packager/commit/e58c7e2af586927848965aace34139fbe2b7abc4)([#113](https://www.github.com/crabnebula-dev/cargo-packager/pull/113)) Add `process-relaunch-dangerous-allow-symlink-macos` feature flag to control whether to allow relaunching if executable path contains a symlink or not.

## \[0.1.0]

- [`c4fa8fd`](https://www.github.com/crabnebula-dev/cargo-packager/commit/c4fa8fd6334b7fd0c32710ea2df0b54aa6bde713) Initial release.
