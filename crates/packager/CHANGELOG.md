# Changelog

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
