---
mdt_cli: minor
mdt_core: major
mdt_mcp: minor
---

Add formatter-aware full-file normalization for `mdt update` and `mdt check` via opt-in `[[formatters]]` config in `mdt.toml`.

When configured, `mdt` now runs matching formatter commands on the entire updated target file, in declaration order, using stdin/stdout. This makes `mdt update` and `mdt check` converge with project formatters like dprint and Prettier, supports formatter-only file drift detection, and surfaces formatter failures with dedicated diagnostics.

`mdt_core` is marked as a major change because public constructible structs gained new fields (`MdtConfig` and `ProjectContext`), which is a semver break for downstream crates that instantiate them with struct literals.
