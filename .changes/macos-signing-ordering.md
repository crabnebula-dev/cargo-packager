---
"cargo-packager": patch
"@crabnebula/packager": patch
---

Fix codesigning failing on macOS under certain circumstances when the order in which files were signed was not
deterministic and nesting required signing files nested more deeply first.
