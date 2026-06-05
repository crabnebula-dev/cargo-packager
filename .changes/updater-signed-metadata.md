---
"cargo-packager": minor
"cargo-packager-updater": minor
---

Cryptographically bind update metadata to the signed artifact to defeat downgrade, freeze and replay attacks. `cargo-packager` now embeds the release `version` (and a `timestamp`) into the (authenticated) minisign trusted comment when signing packages, and the updater verifies that the version advertised by the (unsigned) manifest matches the signed one before installing.

The updater now **always requires** this authenticated metadata: updates whose signatures do not carry a signed `version` and `timestamp` are rejected. **Action required:** update the `cargo-packager` CLI in your release pipeline so your artifacts are signed with this metadata — packages signed by an older `cargo-packager` (or a third-party minisign signer) will be refused by updated clients.

New signer entry points `sign_file_with_version`, `sign_file_with_secret_key_and_version` and `sign_outputs_with_version` produce the version binding (the existing functions still work and now embed the version when invoked through the packaging pipeline). The optional `UpdaterBuilder::signature_expiration` additionally rejects updates whose authenticated signing timestamp is older than a given age.
