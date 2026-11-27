---
"cargo-packager": patch
"@crabnebula/packager": patch
---

Fixed `--profile dev`, `--profile test` and `--profile bench` looking for binaries in `target/<profile>` instead of the `target/debug` or `target/release` directory that cargo writes to.
