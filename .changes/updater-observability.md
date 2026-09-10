---
"cargo-packager-updater": patch
---

Improve update observability: the updater now logs the outcome of signature verification and of installation (success and failure, with the reason) via the `log` crate, making integrity failures and update status visible to embedding applications. Also documents that, on Windows, atomicity and rollback are delegated to the spawned NSIS/WiX installer.
