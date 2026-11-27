---
"@crabnebula/updater": patch
---

Fix `dangerousInsecureTransportProtocol` only applying to `checkUpdate`: the `Update` it returns now keeps the opt-in, so `download` and `downloadAndInstall` no longer reject `http` download URLs in release builds.
