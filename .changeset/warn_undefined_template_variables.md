---
mdt_core: major
---

Add validation warnings for undefined template variables in provider content, helping catch typos in data references.

**BREAKING:** `CheckResult`, `UpdateResult`, `StaleEntry`, `RenderError`, and `TemplateWarning` are now `#[non_exhaustive]`, preventing struct literal construction from outside the crate. `CheckResult` and `UpdateResult` have a new `warnings` field.
