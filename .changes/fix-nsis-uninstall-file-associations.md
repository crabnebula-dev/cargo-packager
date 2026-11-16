---
"cargo-packager": patch
"@crabnebula/packager": patch
---

Fixed NSIS uninstaller leaving associations registered: it iterated `association.ext` instead of `association.extensions`.
