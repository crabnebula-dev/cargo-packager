---
"cargo-packager": patch
"@crabnebula/packager": patch
---

Allow using notarization credentials stored on the Keychain by providing the `APPLE_KEYCHAIN_PROFILE` environment variable. See `xcrun notarytool store-credentials` for more information.
