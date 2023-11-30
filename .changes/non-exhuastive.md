---
"cargo-packager": minor
---

Mark most of the types as `non_exhaustive` to allow adding more field later on without having to break downstream users use the newly added helper methods on these types to modify the corresponding fields in-place.
