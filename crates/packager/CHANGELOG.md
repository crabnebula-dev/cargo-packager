# Changelog

## \[0.6.0]

- [`57b379a`](https://www.github.com/crabnebula-dev/cargo-packager/commit/57b379ad1d9029e767848fda99d4eb6415afe51a)([#148](https://www.github.com/crabnebula-dev/cargo-packager/pull/148)) Added config option to control excluded libs when packaging AppImage
- [`947e032`](https://www.github.com/crabnebula-dev/cargo-packager/commit/947e0328c89d6f043c3ef1b1db5d2252d4f072a5) Fix CLI failing with `Failed to read cargo metadata: cargo metadata` for non-rust projects.

## \[0.5.0]

- [`9bdb953`](https://www.github.com/crabnebula-dev/cargo-packager/commit/9bdb953f1b48c8d69d86e9e42295cd36453c1648)([#137](https://www.github.com/crabnebula-dev/cargo-packager/pull/137)) Add Arch Linux package manager, `pacman` support for cargo packager.
- [`a29943e`](https://www.github.com/crabnebula-dev/cargo-packager/commit/a29943e8c95d70e8b77c23021ce52f6ee13314c8)([#140](https://www.github.com/crabnebula-dev/cargo-packager/pull/140)) Fix codesigning failing on macOS under certain circumstances when the order in which files were signed was not
  deterministic and nesting required signing files nested more deeply first.

### Dependencies

- Upgraded to `cargo-packager-utils@0.1.0`

## \[0.4.5]

- [`f08e4b8`](https://www.github.com/crabnebula-dev/cargo-packager/commit/f08e4b8972b072617fdb78f11e222427e49ebe8e) Fix the signing and notarization process for MacOS bundles
- [`bfa3b00`](https://www.github.com/crabnebula-dev/cargo-packager/commit/bfa3b00cf1087b2ee1e93d9c57b6b577f6294891)([#126](https://www.github.com/crabnebula-dev/cargo-packager/pull/126)) Add `priority` and `section` options in Debian config

## \[0.4.4]

- [`3b3ce76`](https://www.github.com/crabnebula-dev/cargo-packager/commit/3b3ce76da0581cf8d553d6edeb0df24f896c62a6)([#128](https://www.github.com/crabnebula-dev/cargo-packager/pull/128)) Fix file download not working on macOS and Windows (arm).

## \[0.4.3]

- [`2a50c8e`](https://www.github.com/crabnebula-dev/cargo-packager/commit/2a50c8ea734193036db0ab461f9005ea904cf4b7)([#124](https://www.github.com/crabnebula-dev/cargo-packager/pull/124)) Fix packaing of external binaries.

## \[0.4.2]

- [`c18bf3e`](https://www.github.com/crabnebula-dev/cargo-packager/commit/c18bf3e77f91c1c4797992b25902753deee5c986)([#117](https://www.github.com/crabnebula-dev/cargo-packager/pull/117)) Fix the `non-standard-file-perm` and `non-standard-dir-perm` issue in Debian packages

## \[0.4.1]

- [`7b083a8`](https://www.github.com/crabnebula-dev/cargo-packager/commit/7b083a8c2ae709659c03a1069d96c3a8391e0674)([#99](https://www.github.com/crabnebula-dev/cargo-packager/pull/99)) Add glob patterns support for the icons option in config.
- [`7e05d24`](https://www.github.com/crabnebula-dev/cargo-packager/commit/7e05d24a697230b1f53ee5ee2f7d217047089d97)([#109](https://www.github.com/crabnebula-dev/cargo-packager/pull/109)) Check if required files/tools for packaging are outdated or mis-hashed and redownload them.
- [`ea6c31b`](https://www.github.com/crabnebula-dev/cargo-packager/commit/ea6c31b1a3b56bb5408a78f1b2d6b2a2d9ce1161)([#114](https://www.github.com/crabnebula-dev/cargo-packager/pull/114)) Fix NSIS uninstaller leaving resources behind and failing to remove the installation directory.

## \[0.4.0]

- [`ecde3fb`](https://www.github.com/crabnebula-dev/cargo-packager/commit/ecde3fb71a8f120e71d4781c11214db750042cc4)([#58](https://www.github.com/crabnebula-dev/cargo-packager/pull/58)) Added `files` configuration under `AppImageConfig` for adding custom files on the AppImage's AppDir.
- [`ecde3fb`](https://www.github.com/crabnebula-dev/cargo-packager/commit/ecde3fb71a8f120e71d4781c11214db750042cc4)([#58](https://www.github.com/crabnebula-dev/cargo-packager/pull/58)) Renamed binary `filename` property to `path`, which supports absolute paths.
- [`f04c17f`](https://www.github.com/crabnebula-dev/cargo-packager/commit/f04c17f72a4af306f47065aff405c4bd0f7b6442)([#87](https://www.github.com/crabnebula-dev/cargo-packager/pull/87)) Add `config.dmg` to configure the DMG on macOS.
- [`21a6c9e`](https://www.github.com/crabnebula-dev/cargo-packager/commit/21a6c9ef4ddbefe9a6e6c5abf287f2ad993edffb)([#84](https://www.github.com/crabnebula-dev/cargo-packager/pull/84)) Mark most of the types as `non_exhaustive` to allow adding more field later on without having to break downstream users use the newly added helper methods on these types to modify the corresponding fields in-place.
- [`db75777`](https://www.github.com/crabnebula-dev/cargo-packager/commit/db75777d2799ca37217d568befad39b9377cfa2a) Add `config.windows.sign_command` which can be used to override signing command on windows and allows usage of tools other than `signtool.exe`.

## \[0.3.0]

- [`65b8c20`](https://www.github.com/crabnebula-dev/cargo-packager/commit/65b8c20a96877038daa4907b80cd96f96e0bfe33)([#54](https://www.github.com/crabnebula-dev/cargo-packager/pull/54)) Code sign binaries and frameworks on macOS.
- [`7ef6b7c`](https://www.github.com/crabnebula-dev/cargo-packager/commit/7ef6b7c0186e79243240cb2d1a1846fda41a1b54)([#50](https://www.github.com/crabnebula-dev/cargo-packager/pull/50)) Set `root` as the owner of control files and package files in `deb` package.
- [`8cc5b05`](https://www.github.com/crabnebula-dev/cargo-packager/commit/8cc5b05eb3eb124b385d406329eee379349faa86)([#53](https://www.github.com/crabnebula-dev/cargo-packager/pull/53)) Fixed an error message that the source path does not exist when packaging .app
- [`274a6be`](https://www.github.com/crabnebula-dev/cargo-packager/commit/274a6bec553f273934347a18e0d6e2e1ec61bbeb)([#49](https://www.github.com/crabnebula-dev/cargo-packager/pull/49)) Read `HTTP_PROXY` env var when downloading resources.
- [`6ed1312`](https://www.github.com/crabnebula-dev/cargo-packager/commit/6ed1312926d70cf449e7beddacb56a17e51a25ac)([#52](https://www.github.com/crabnebula-dev/cargo-packager/pull/52)) Read the `APPLE_TEAM_ID` environment variable for macOS notarization arguments.
- [`65b8c20`](https://www.github.com/crabnebula-dev/cargo-packager/commit/65b8c20a96877038daa4907b80cd96f96e0bfe33)([#54](https://www.github.com/crabnebula-dev/cargo-packager/pull/54)) Remove extended attributes on the macOS app bundle using `xattr -cr $PATH`.

## \[0.2.0]

- [`dde1ab3`](https://www.github.com/crabnebula-dev/cargo-packager/commit/dde1ab34b59ee614fc24e47a5caa8ebc04d92a08)([#43](https://www.github.com/crabnebula-dev/cargo-packager/pull/43)) Remove the deprecated `cargo-packager-config` dependency.

## \[0.1.2]

- [`1809f10`](https://www.github.com/crabnebula-dev/cargo-packager/commit/1809f10b4fd1720fd740196f67c3c980ade0a6bd) Respect the `config.enabled` option.

### Dependencies

- Upgraded to `cargo-packager-config@0.2.0`

## \[0.1.1]

- [`2d8b8d7`](https://www.github.com/crabnebula-dev/cargo-packager/commit/2d8b8d7c1af73202639449a00dbc51bf171effc7) Initial Release
