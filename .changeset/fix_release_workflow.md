---
mdt_cli: patch
---

Fix release workflow to checkout `main` branch instead of tag ref, and add version verification step to prevent publishing mismatches. Also add `cargo check --workspace` to the knope release workflow to catch build errors before creating tags.
