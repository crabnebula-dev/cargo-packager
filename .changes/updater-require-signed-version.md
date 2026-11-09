---
"cargo-packager-updater": minor
"@crabnebula/updater": minor
---

The updater now rejects an update whose signature does not contain a signed app version, or whose signed version does not match the version advertised by the update manifest, preventing downgrade attacks.

**Breaking:** update `cargo-packager` and `cargo-packager-updater` together. Updates signed by an older `cargo-packager` or another minisign signer cannot be installed by the new updater.
