---
"cargo-packager-updater": minor
"@crabnebula/updater": minor
---

Enforce secure transport when checking for and downloading updates. Update endpoints and download URLs must now use HTTPS in release builds; no host is exempt (not even loopback addresses). Plain `http` is allowed in debug builds to ease local development, with a warning that it will be rejected in release builds unless explicitly enabled. Redirects that downgrade from `https` to `http` are refused, and the number of followed redirects is capped. Set `Config::dangerousInsecureTransportProtocol` (or `UpdaterBuilder::dangerous_insecure_transport_protocol`) to opt back into insecure transport for trusted, isolated networks. The Node.js bindings expose this as `dangerousInsecureTransportProtocol`, alongside the new `signatureExpiration` option.
