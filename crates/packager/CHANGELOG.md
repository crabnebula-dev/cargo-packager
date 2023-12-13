# Changelog

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
