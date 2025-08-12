# Changelog

## \[0.11.8]

- [`6e6a10c`](https://www.github.com/crabnebula-dev/cargo-packager/commit/6e6a10cc1692973293966034dc4b798e3976d094) ([#321](https://www.github.com/crabnebula-dev/cargo-packager/pull/321)) Allow explicitly specifying the Package name for the .deb bundle.
- [`c34de36`](https://www.github.com/crabnebula-dev/cargo-packager/commit/c34de365705db150eb101caa94adf42eff74f71a) ([#365](https://www.github.com/crabnebula-dev/cargo-packager/pull/365)) Change nsi template from using `association.ext` to `association.extensions`, to match struct field in `FileAssociation`.
  This allows file associations to be generated in `.nsi` files, and therefore in the final NSIS installer.

### Dependencies

- Upgraded to `cargo-packager@0.11.8`

## \[0.11.7]

- [`d49b606`](https://www.github.com/crabnebula-dev/cargo-packager/commit/d49b606ba8a612c833233ec8a6061481a2118639) ([#353](https://www.github.com/crabnebula-dev/cargo-packager/pull/353)) Allow using notarization credentials stored on the Keychain by providing the `APPLE_KEYCHAIN_PROFILE` environment variable. See `xcrun notarytool store-credentials` for more information.
- [`b337564`](https://www.github.com/crabnebula-dev/cargo-packager/commit/b337564c0e5a9de966b4124890dddea1e353acb4) ([#362](https://www.github.com/crabnebula-dev/cargo-packager/pull/362)) Updated linuxdeploy's AppImage plugin to not require libfuse on the user's system anymore.

### Dependencies

- Upgraded to `cargo-packager@0.11.7`

## \[0.11.6]

- [`b81b81f`](https://www.github.com/crabnebula-dev/cargo-packager/commit/b81b81fbd7fd185edfc7652f535d0cfacb786ac9) ([#354](https://www.github.com/crabnebula-dev/cargo-packager/pull/354)) Changed the download URL of a dependency of the AppImage bundler to Tauri's mirror to resolve 404 errors.
- [`5205088`](https://www.github.com/crabnebula-dev/cargo-packager/commit/5205088cd78412fb6cbe5e48a715524fcc5a2ee7) ([#340](https://www.github.com/crabnebula-dev/cargo-packager/pull/340)) Enhance sign error message.

### Dependencies

- Upgraded to `cargo-packager@0.11.6`

## \[0.11.5]

- [`17194a9`](https://www.github.com/crabnebula-dev/cargo-packager/commit/17194a92aabd59c9e075105072ff939f5d55a107) ([#313](https://www.github.com/crabnebula-dev/cargo-packager/pull/313)) Added `linux > generateDesktopEntry` config to allow disabling generating a .desktop file on Linux bundles (defaults to true).
- [`17c52f0`](https://www.github.com/crabnebula-dev/cargo-packager/commit/17c52f057d78340983689af3c00b1f2aeff3c417) ([#289](https://www.github.com/crabnebula-dev/cargo-packager/pull/289)) Added support to embedding additional apps in the macOS app bundle.
- [`17c52f0`](https://www.github.com/crabnebula-dev/cargo-packager/commit/17c52f057d78340983689af3c00b1f2aeff3c417) ([#289](https://www.github.com/crabnebula-dev/cargo-packager/pull/289)) Added support to adding an `embedded.provisionprofile` file to the macOS bundle.
- [`e010574`](https://www.github.com/crabnebula-dev/cargo-packager/commit/e010574c2efa4a1aa6b8e475a62bec46f24f2bc5) ([#318](https://www.github.com/crabnebula-dev/cargo-packager/pull/318)) Add `background-app` config setting for macOS to set `LSUIElement` to `true`.

### Dependencies

- Upgraded to `cargo-packager@0.11.5`

## \[0.11.4]

### Dependencies

- Upgraded to `cargo-packager@0.11.4`

## \[0.11.3]

### Dependencies

- Upgraded to `cargo-packager@0.11.3`

## \[0.11.2]

- [`fea80d5`](https://www.github.com/crabnebula-dev/cargo-packager/commit/fea80d5760882e6cdc21c8ed2f82d323e0598926) ([#264](https://www.github.com/crabnebula-dev/cargo-packager/pull/264)) Fix `pacman` package failing to install when source directory contained whitespace.

### Dependencies

- Upgraded to `cargo-packager@0.11.2`

## \[0.11.1]

- [`4523722`](https://www.github.com/crabnebula-dev/cargo-packager/commit/4523722d0808faef4a91dbb227badd0354f4c71a) ([#283](https://www.github.com/crabnebula-dev/cargo-packager/pull/283)) Fixes resources paths on NSIS when cross compiling.

### Dependencies

- Upgraded to `cargo-packager@0.11.1`

## \[0.11.0]

- [`41b05d0`](https://www.github.com/crabnebula-dev/cargo-packager/commit/41b05d08a635d593df4cf4eefbe921b92ace77b7) ([#277](https://www.github.com/crabnebula-dev/cargo-packager/pull/277)) Add `--target` flag to specify target triple to package.

### Dependencies

- Upgraded to `cargo-packager@0.11.0`

## \[0.10.3]

### Dependencies

- Upgraded to `cargo-packager@0.10.3`

## \[0.10.2]

### Dependencies

- Upgraded to `cargo-packager@0.10.2`
- Upgraded to `cargo-packager-utils@0.1.1`

## \[0.10.1]

- [`522f23b`](https://www.github.com/crabnebula-dev/cargo-packager/commit/522f23bd867b037eeec81c43295aafd38ebe60ec) ([#258](https://www.github.com/crabnebula-dev/cargo-packager/pull/258)) Update NSIS installer template URL.
- [`bce99ae`](https://www.github.com/crabnebula-dev/cargo-packager/commit/bce99aecb4160291a026dcd4750055f9079099f8) ([#260](https://www.github.com/crabnebula-dev/cargo-packager/pull/260)) Fix NSIS uninstaller removing the uninstall directory even if it was not empty.

### Dependencies

- Upgraded to `cargo-packager@0.10.1`

## \[0.10.0]

- [`c6207bb`](https://www.github.com/crabnebula-dev/cargo-packager/commit/c6207bba042a8a0184ddb7e12650a4cd8f415c23) ([#254](https://www.github.com/crabnebula-dev/cargo-packager/pull/254)) Allow Linux dependencies to be specified via a file path instead of just a direct String.
  This enables the list of dependencies to by dynamically generated for both Debian `.deb` packages and pacman packages,
  which can relieve the app developer from the burden of manually maintaining a fixed list of dependencies.
- [`de4dcca`](https://www.github.com/crabnebula-dev/cargo-packager/commit/de4dccaca4ae758d3adde517cc415a002873e642) ([#256](https://www.github.com/crabnebula-dev/cargo-packager/pull/256)) Automatically add an Exec arg (field code) in the `.desktop` file.

  This adds an `{exec_arg}` field to the default `main.desktop` template.
  This field is populated with a sane default value, based on the
  `deep_link_protocols` or `file_associations` in the `Config` struct.

  This allows an installed Debian package to be invoked by other
  applications with URLs or files as arguments, as expected.

### Dependencies

- Upgraded to `cargo-packager@0.10.0`

## \[0.9.1]

- [`44a19ea`](https://www.github.com/crabnebula-dev/cargo-packager/commit/44a19eae1f5f26b1bd10ba84dd6eb3d856609a67) ([#246](https://www.github.com/crabnebula-dev/cargo-packager/pull/246)) On macOS, fix notarization skipping needed environment variables when macos specific config has been specified in the config file.

### Dependencies

- Upgraded to `cargo-packager@0.9.1`

## \[0.9.0]

- [`ab53974`](https://www.github.com/crabnebula-dev/cargo-packager/commit/ab53974b683ce282202e1a550c551eed951e9ca7) ([#235](https://www.github.com/crabnebula-dev/cargo-packager/pull/235)) Added deep link support.

### Dependencies

- Upgraded to `cargo-packager@0.9.0`

## \[0.8.1]

- [`1375380`](https://www.github.com/crabnebula-dev/cargo-packager/commit/1375380c7c9d2adf55ab18a2ce23917849967995)([#196](https://www.github.com/crabnebula-dev/cargo-packager/pull/196)) Always show shell commands output for `beforePackageCommand` and `beforeEachPackagingCommand` .

### Dependencies

- Upgraded to `cargo-packager@0.8.1`

## \[0.8.0]

- [`2164d02`](https://www.github.com/crabnebula-dev/cargo-packager/commit/2164d022f5705e59a189007aec7c99cce98136d8)([#198](https://www.github.com/crabnebula-dev/cargo-packager/pull/198)) Allow packaging the macOS app bundle on Linux and Windows hosts (without codesign support).
- [`3057a4a`](https://www.github.com/crabnebula-dev/cargo-packager/commit/3057a4a8440bc4dc897f3038ac821ed181644d43)([#197](https://www.github.com/crabnebula-dev/cargo-packager/pull/197)) Added `Config::binaries_dir` and `--binaries-dir` so you can specify the location of the binaries without modifying the output directory.
- [`4c4d919`](https://www.github.com/crabnebula-dev/cargo-packager/commit/4c4d9194fb0bd2a814f46336747e643b1e208b52)([#195](https://www.github.com/crabnebula-dev/cargo-packager/pull/195)) Error out if we cannot find a configuration file.
- [`b04332c`](https://www.github.com/crabnebula-dev/cargo-packager/commit/b04332c8fc61427dc002a40d9d46bc5f930025c2)([#194](https://www.github.com/crabnebula-dev/cargo-packager/pull/194)) Fixes a crash when packaging `.app` if an empty file is included in the bundle.
- [`3057a4a`](https://www.github.com/crabnebula-dev/cargo-packager/commit/3057a4a8440bc4dc897f3038ac821ed181644d43)([#197](https://www.github.com/crabnebula-dev/cargo-packager/pull/197)) Added `--out-dir/-o` flags and removed the positional argument to specify where to ouput packages, use the newly added flags instead.

### Dependencies

- Upgraded to `cargo-packager@0.8.0`

## \[0.7.0]

- [`cd8898a`](https://www.github.com/crabnebula-dev/cargo-packager/commit/cd8898a93b66a4aae050fa1006089c3c3b5646f9)([#187](https://www.github.com/crabnebula-dev/cargo-packager/pull/187)) Added codesign certificate and notarization credentials configuration options under the `macos` config (for programatic usage, taking precedence over environment variables).

### Dependencies

- Upgraded to `cargo-packager@0.7.0`

## \[0.6.1]

### Dependencies

- Upgraded to `cargo-packager@0.6.1`

## \[0.6.0]

- [`57b379a`](https://www.github.com/crabnebula-dev/cargo-packager/commit/57b379ad1d9029e767848fda99d4eb6415afe51a)([#148](https://www.github.com/crabnebula-dev/cargo-packager/pull/148)) Added config option to control excluded libs when packaging AppImage
- [`947e032`](https://www.github.com/crabnebula-dev/cargo-packager/commit/947e0328c89d6f043c3ef1b1db5d2252d4f072a5) Fix CLI failing with `Failed to read cargo metadata: cargo metadata` for non-rust projects.
- Bumpt to `0.6.0` version directly to match the Rust crate version.

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
