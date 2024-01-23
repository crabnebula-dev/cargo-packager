# cargo-packager-resource-resolver

Resource resolver for apps that was packaged by [`cargo-packager`](https://docs.rs/cargo-packager).

It resolves the root path which contains resources, which was set using the `resources` field of [cargo packager configuration](https://docs.rs/cargo-packager/latest/cargo_packager/config/struct.Config.html).

## Get the resource path

```rs
use cargo_packager_resource_resolver::{resources_dir, PackageFormat};

let resource_path = resources_dir(PackageFormat::Nsis).unwrap();
```

## Automatically detect formats

:warning: This feature is only available for Rust apps that were built with cargo packager.

1. Make sure to use the `before_each_package_command` field of [cargo packager configuration](https://docs.rs/cargo-packager/latest/cargo_packager/config/struct.Config.html) to build your app (this will not work with the `before_packaging_command` field).
2. Activete the feature `auto-detect-format` for this crate in your Cargo.toml.

```rs
use cargo_packager_resource_resolver::{resources_dir, current_format};

let resource_path = resources_dir(current_format().unwrap()).unwrap();
```

## Licenses

MIT or MIT/Apache 2.0 where applicable.
