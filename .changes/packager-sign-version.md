---
"cargo-packager": minor
"@crabnebula/packager": minor
---

Record the app version in the trusted comment of update signatures, binding each signed package to the version it is released as. `cargo packager signer sign` now requires `--app-version`. Library users should sign with `package_and_sign` or the new `sign_outputs_with_version` / `sign_file_with_version` functions. Versions containing a tab or newline are rejected.
