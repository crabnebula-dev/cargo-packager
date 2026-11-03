---
"cargo-packager": patch
"@crabnebula/packager": patch
---

Fixed `--config` failing with `os error 267` on Windows. The package name is now
looked up next to the config file instead of using the config file itself as the
working directory, and is taken as an unquoted string.
