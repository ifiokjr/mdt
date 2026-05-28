---
mdt_core: patch
---

Update `logos` to 0.16.1 — remove the `#[logos(skip r"")]` attribute which is now rejected because an empty regex matches the empty string.
