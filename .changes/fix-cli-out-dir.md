---
"cargo-packager": patch
"@crabnebula/packager": patch
---

Fixed newly created relative `--out-dir` paths to be canonicalized like existing paths, preventing packages from being written under the configuration directory.
