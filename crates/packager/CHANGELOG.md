# Changelog

## \[0.11.8]

- [`6e6a10c`](https://www.github.com/crabnebula-dev/cargo-packager/commit/6e6a10cc1692973293966034dc4b798e3976d094) ([#321](https://www.github.com/crabnebula-dev/cargo-packager/pull/321)) Allow explicitly specifying the Package name for the .deb bundle.
- [`c34de36`](https://www.github.com/crabnebula-dev/cargo-packager/commit/c34de365705db150eb101caa94adf42eff74f71a) ([#365](https://www.github.com/crabnebula-dev/cargo-packager/pull/365)) Change nsi template from using `association.ext` to `association.extensions`, to match struct field in `FileAssociation`.
  This allows file associations to be generated in `.nsi` files, and therefore in the final NSIS installer.

## \[0.11.7]

- [`d49b606`](https://www.github.com/crabnebula-dev/cargo-packager/commit/d49b606ba8a612c833233ec8a6061481a2118639) ([#353](https://www.github.com/crabnebula-dev/cargo-packager/pull/353)) Allow using notarization credentials stored on the Keychain by providing the `APPLE_KEYCHAIN_PROFILE` environment variable. See `xcrun notarytool store-credentials` for more information.
- [`b337564`](https://www.github.com/crabnebula-dev/cargo-packager/commit/b337564c0e5a9de966b4124890dddea1e353acb4) ([#362](https://www.github.com/crabnebula-dev/cargo-packager/pull/362)) Updated linuxdeploy's AppImage plugin to not require libfuse on the user's system anymore.

## \[0.11.6]

- [`b81b81f`](https://www.github.com/crabnebula-dev/cargo-packager/commit/b81b81fbd7fd185edfc7652f535d0cfacb786ac9) ([#354](https://www.github.com/crabnebula-dev/cargo-packager/pull/354)) Changed the download URL of a dependency of the AppImage bundler to Tauri's mirror to resolve 404 errors.
- [`735d6c4`](https://www.github.com/crabnebula-dev/cargo-packager/commit/735d6c4745911793cbcf5d929d8da288840bcf24) ([#345](https://www.github.com/crabnebula-dev/cargo-packager/pull/345)) Fixed a typo on the `digest_algorithm` config (was `digest-algorithim`).
- [`5205088`](https://www.github.com/crabnebula-dev/cargo-packager/commit/5205088cd78412fb6cbe5e48a715524fcc5a2ee7) ([#340](https://www.github.com/crabnebula-dev/cargo-packager/pull/340)) Enhance sign error message.
- [`55924d3`](https://www.github.com/crabnebula-dev/cargo-packager/commit/55924d3522c4ab1cfcb4436044e5ebad8adf241c) ([#334](https://www.github.com/crabnebula-dev/cargo-packager/pull/334)) Migrate from `winreg` crate to `windows-registry`. This adds new variants to the packager's `Error` type.

## \[0.11.5]

- [`17194a9`](https://www.github.com/crabnebula-dev/cargo-packager/commit/17194a92aabd59c9e075105072ff939f5d55a107) ([#313](https://www.github.com/crabnebula-dev/cargo-packager/pull/313)) Added `linux > generateDesktopEntry` config to allow disabling generating a .desktop file on Linux bundles (defaults to true).
- [`17c52f0`](https://www.github.com/crabnebula-dev/cargo-packager/commit/17c52f057d78340983689af3c00b1f2aeff3c417) ([#289](https://www.github.com/crabnebula-dev/cargo-packager/pull/289)) Added support to embedding additional apps in the macOS app bundle.
- [`17c52f0`](https://www.github.com/crabnebula-dev/cargo-packager/commit/17c52f057d78340983689af3c00b1f2aeff3c417) ([#289](https://www.github.com/crabnebula-dev/cargo-packager/pull/289)) Added support to adding an `embedded.provisionprofile` file to the macOS bundle.
- [`e010574`](https://www.github.com/crabnebula-dev/cargo-packager/commit/e010574c2efa4a1aa6b8e475a62bec46f24f2bc5) ([#318](https://www.github.com/crabnebula-dev/cargo-packager/pull/318)) Add `background-app` config setting for macOS to set `LSUIElement` to `true`.

## \[0.11.4]

- [`29b60a9`](https://www.github.com/crabnebula-dev/cargo-packager/commit/29b60a97ec14ef87aee7537fa7fbd848f853ac32) ([#305](https://www.github.com/crabnebula-dev/cargo-packager/pull/305)) Fix AppImage bundle when main binary name has spaces.

## \[0.11.3]

- [`82e690d`](https://www.github.com/crabnebula-dev/cargo-packager/commit/82e690dfce6109531391e683c8b486d0f39ea335) ([#300](https://www.github.com/crabnebula-dev/cargo-packager/pull/300)) Fix the `Exec` entry on the Linux .desktop file when the binary name contains spaces.

## \[0.11.2]

- [`fea80d5`](https://www.github.com/crabnebula-dev/cargo-packager/commit/fea80d5760882e6cdc21c8ed2f82d323e0598926) ([#264](https://www.github.com/crabnebula-dev/cargo-packager/pull/264)) Fix `pacman` package failing to install when source directory contained whitespace.

## \[0.11.1]

- [`4523722`](https://www.github.com/crabnebula-dev/cargo-packager/commit/4523722d0808faef4a91dbb227badd0354f4c71a) ([#283](https://www.github.com/crabnebula-dev/cargo-packager/pull/283)) Fixes resources paths on NSIS when cross compiling.

## \[0.11.0]

- [`41b05d0`](https://www.github.com/crabnebula-dev/cargo-packager/commit/41b05d08a635d593df4cf4eefbe921b92ace77b7) ([#277](https://www.github.com/crabnebula-dev/cargo-packager/pull/277)) Respect `target-triple` config option when packaging rust binaries.
- [`41b05d0`](https://www.github.com/crabnebula-dev/cargo-packager/commit/41b05d08a635d593df4cf4eefbe921b92ace77b7) ([#277](https://www.github.com/crabnebula-dev/cargo-packager/pull/277)) Add `--target` flag to specify target triple to package.

## \[0.10.3]

- [`3ee764d`](https://www.github.com/crabnebula-dev/cargo-packager/commit/3ee764d9193ae22331aa5894a1821453e9542992) ([#270](https://www.github.com/crabnebula-dev/cargo-packager/pull/270)) Fixes AppImage bundling failing due to missing `/usr/lib64` directory.
- [`ab41e6d`](https://www.github.com/crabnebula-dev/cargo-packager/commit/ab41e6d94af89ec721a0047636597682bd6d90f6) ([#269](https://www.github.com/crabnebula-dev/cargo-packager/pull/269)) Fix using the crate as a library without `cli` feature flag

## \[0.10.2]

- [`f836afa`](https://www.github.com/crabnebula-dev/cargo-packager/commit/f836afa699b2da8a55432ce9de1cbccbffb705fb) ([#267](https://www.github.com/crabnebula-dev/cargo-packager/pull/267)) Include notarytool log output on error message in case notarization fails.
- [`21441f3`](https://www.github.com/crabnebula-dev/cargo-packager/commit/21441f30c5a258b73926ba7a7d8126d6bf47a662) ([#262](https://www.github.com/crabnebula-dev/cargo-packager/pull/262)) Fixed dmg failed to bundle the application when out-dir does not exist.

### Dependencies

- Upgraded to `cargo-packager-utils@0.1.1`

## \[0.10.1]

- [`522f23b`](https://www.github.com/crabnebula-dev/cargo-packager/commit/522f23bd867b037eeec81c43295aafd38ebe60ec) ([#258](https://www.github.com/crabnebula-dev/cargo-packager/pull/258)) Update NSIS installer template URL.
- [`bce99ae`](https://www.github.com/crabnebula-dev/cargo-packager/commit/bce99aecb4160291a026dcd4750055f9079099f8) ([#260](https://www.github.com/crabnebula-dev/cargo-packager/pull/260)) Fix NSIS uninstaller removing the uninstall directory even if it was not empty.

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

## \[0.9.1]

- [`44a19ea`](https://www.github.com/crabnebula-dev/cargo-packager/commit/44a19eae1f5f26b1bd10ba84dd6eb3d856609a67) ([#246](https://www.github.com/crabnebula-dev/cargo-packager/pull/246)) On macOS, fix notarization skipping needed environment variables when macos specific config has been specified in the config file.

## \[0.9.0]

- [`ab53974`](https://www.github.com/crabnebula-dev/cargo-packager/commit/ab53974b683ce282202e1a550c551eed951e9ca7) ([#235](https://www.github.com/crabnebula-dev/cargo-packager/pull/235)) Added deep link support.

## \[0.8.1]

- [`1375380`](https://www.github.com/crabnebula-dev/cargo-packager/commit/1375380c7c9d2adf55ab18a2ce23917849967995)([#196](https://www.github.com/crabnebula-dev/cargo-packager/pull/196)) Always show shell commands output for `beforePackageCommand` and `beforeEachPackagingCommand` .

## \[0.8.0]

- [`2164d02`](https://www.github.com/crabnebula-dev/cargo-packager/commit/2164d022f5705e59a189007aec7c99cce98136d8)([#198](https://www.github.com/crabnebula-dev/cargo-packager/pull/198)) Allow packaging the macOS app bundle on Linux and Windows hosts (without codesign support).
- [`3057a4a`](https://www.github.com/crabnebula-dev/cargo-packager/commit/3057a4a8440bc4dc897f3038ac821ed181644d43)([#197](https://www.github.com/crabnebula-dev/cargo-packager/pull/197)) Added `Config::binaries_dir` and `--binaries-dir` so you can specify the location of the binaries without modifying the output directory.
- [`4c4d919`](https://www.github.com/crabnebula-dev/cargo-packager/commit/4c4d9194fb0bd2a814f46336747e643b1e208b52)([#195](https://www.github.com/crabnebula-dev/cargo-packager/pull/195)) Error out if we cannot find a configuration file.
- [`b04332c`](https://www.github.com/crabnebula-dev/cargo-packager/commit/b04332c8fc61427dc002a40d9d46bc5f930025c2)([#194](https://www.github.com/crabnebula-dev/cargo-packager/pull/194)) Fixes a crash when packaging `.app` if an empty file is included in the bundle.
- [`3057a4a`](https://www.github.com/crabnebula-dev/cargo-packager/commit/3057a4a8440bc4dc897f3038ac821ed181644d43)([#197](https://www.github.com/crabnebula-dev/cargo-packager/pull/197)) Added `--out-dir/-o` flags and removed the positional argument to specify where to ouput packages, use the newly added flags instead.
- [`2164d02`](https://www.github.com/crabnebula-dev/cargo-packager/commit/2164d022f5705e59a189007aec7c99cce98136d8)([#198](https://www.github.com/crabnebula-dev/cargo-packager/pull/198)) Renamed `PackageOuput` to `PackageOutput` and added `PackageOutput::new`.

## \[0.7.0]

- [`cd8898a`](https://www.github.com/crabnebula-dev/cargo-packager/commit/cd8898a93b66a4aae050fa1006089c3c3b5646f9)([#187](https://www.github.com/crabnebula-dev/cargo-packager/pull/187)) Added codesign certificate and notarization credentials configuration options under the `macos` config (for programatic usage, taking precedence over environment variables).

## \[0.6.1]

- [`2f1029b`](https://www.github.com/crabnebula-dev/cargo-packager/commit/2f1029b2032ac44fd3f3df34307554feb17043b7)([#185](https://www.github.com/crabnebula-dev/cargo-packager/pull/185)) Fix bundling NSIS on Linux and macOS failing due to the verbose flag.

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
