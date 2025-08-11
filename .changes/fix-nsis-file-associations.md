---
"cargo-packager": patch
"@crabnebula/packager": patch
---

Change nsi template from using `association.ext` to `association.extensions`, to match struct field in `FileAssociation`.
This allows file associations to be generated in `.nsi` files, and therefore in the final NSIS installer.