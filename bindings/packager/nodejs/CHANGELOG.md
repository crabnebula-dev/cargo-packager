# Changelog

## \[0.3.0]

- [`57b379a`](https://www.github.com/crabnebula-dev/cargo-packager/commit/57b379ad1d9029e767848fda99d4eb6415afe51a)([#148](https://www.github.com/crabnebula-dev/cargo-packager/pull/148)) Added config option to control excluded libs when packaging AppImage
- [`947e032`](https://www.github.com/crabnebula-dev/cargo-packager/commit/947e0328c89d6f043c3ef1b1db5d2252d4f072a5) Fix CLI failing with `Failed to read cargo metadata: cargo metadata` for non-rust projects.

### Dependencies

- Upgraded to `cargo-packager@0.6.0`

## \[0.2.0]

- [`9bdb953`](https://www.github.com/crabnebula-dev/cargo-packager/commit/9bdb953f1b48c8d69d86e9e42295cd36453c1648)([#137](https://www.github.com/crabnebula-dev/cargo-packager/pull/137)) Add Arch Linux package manager, `pacman` support for cargo packager.
- [`a29943e`](https://www.github.com/crabnebula-dev/cargo-packager/commit/a29943e8c95d70e8b77c23021ce52f6ee13314c8)([#140](https://www.github.com/crabnebula-dev/cargo-packager/pull/140)) Fix codesigning failing on macOS under certain circumstances when the order in which files were signed was not
  deterministic and nesting required signing files nested more deeply first.

### Dependencies

- Upgraded to `cargo-packager@0.5.0`
- Upgraded to `cargo-packager-utils@0.1.0`

## \[0.1.5]

- [`f08e4b8`](https://www.github.com/crabnebula-dev/cargo-packager/commit/f08e4b8972b072617fdb78f11e222427e49ebe8e) Fix the signing and notarization process for MacOS bundles
- [`bfa3b00`](https://www.github.com/crabnebula-dev/cargo-packager/commit/bfa3b00cf1087b2ee1e93d9c57b6b577f6294891)([#126](https://www.github.com/crabnebula-dev/cargo-packager/pull/126)) Add `priority` and `section` options in Debian config

### Dependencies

- Upgraded to `cargo-packager@0.4.5`

## \[0.1.4]

- [`3b3ce76`](https://www.github.com/crabnebula-dev/cargo-packager/commit/3b3ce76da0581cf8d553d6edeb0df24f896c62a6)([#128](https://www.github.com/crabnebula-dev/cargo-packager/pull/128)) Fix file download not working on macOS and Windows (arm).

### Dependencies

- Upgraded to `cargo-packager@0.4.4`

## \[0.1.3]

- [`2a50c8e`](https://www.github.com/crabnebula-dev/cargo-packager/commit/2a50c8ea734193036db0ab461f9005ea904cf4b7)([#124](https://www.github.com/crabnebula-dev/cargo-packager/pull/124)) Fix packaing of external binaries.

### Dependencies

- Upgraded to `cargo-packager@0.4.3`

## \[0.1.2]

- [`bd7e6fc`](https://www.github.com/crabnebula-dev/cargo-packager/commit/bd7e6fc102a74dc4da39848f44d04968b498b3cf)([#123](https://www.github.com/crabnebula-dev/cargo-packager/pull/123)) Fixes published package not including the build folder.

### Dependencies

- Upgraded to `cargo-packager@0.4.2`

## \[0.1.1]

- [`7e05d24`](https://www.github.com/crabnebula-dev/cargo-packager/commit/7e05d24a697230b1f53ee5ee2f7d217047089d97)([#109](https://www.github.com/crabnebula-dev/cargo-packager/pull/109)) Check if required files/tools for packaging are outdated or mis-hashed and redownload them.
- [`ea6c31b`](https://www.github.com/crabnebula-dev/cargo-packager/commit/ea6c31b1a3b56bb5408a78f1b2d6b2a2d9ce1161)([#114](https://www.github.com/crabnebula-dev/cargo-packager/pull/114)) Fix NSIS uninstaller leaving resources behind and failing to remove the installation directory.

### Dependencies

- Upgraded to `cargo-packager@0.4.1`

## \[0.1.0]

- [`c4fa8fd`](https://www.github.com/crabnebula-dev/cargo-packager/commit/c4fa8fd6334b7fd0c32710ea2df0b54aa6bde713) Initial release.

### Dependencies

- Upgraded to `cargo-packager@0.4.0`
