---
mdt_core: major
---

Add formatter-aware full-file normalization via opt-in `[[formatters]]` config in `mdt.toml`.

Public constructible structs gained new fields (`MdtConfig` and `ProjectContext`), which is a semver break for downstream crates that instantiate them with struct literals.

When configured, the engine runs matching formatter commands on the entire updated target file, in declaration order, using stdin/stdout. This supports formatter-only file drift detection and surfaces formatter failures with dedicated diagnostics.
