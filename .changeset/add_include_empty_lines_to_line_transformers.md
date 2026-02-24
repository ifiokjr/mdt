---
mdt_core: minor
---

Add optional `includeEmpty` boolean argument to `indent`, `linePrefix`, and `lineSuffix` transformers.

Previously, these line-based transformers always skipped empty lines, leaving them completely blank. This caused problems in contexts like Rust doc comments (`//!`, `///`) where every line — including blank separator lines — must carry the comment prefix.

Now you can pass `true` as a second argument to include empty lines:

```markdown
<!-- {=docs|linePrefix:"/// ":true} -->
```

This produces correct Rust doc comments where empty lines get `///` instead of being left blank. The default behavior (skipping empty lines) is unchanged.

All three line-based transformers support this:

- `indent:"  ":true`
- `linePrefix:"// ":true`
- `lineSuffix:";":true`
