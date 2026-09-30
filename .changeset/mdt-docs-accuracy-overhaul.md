---
"mdt_cli": docs
"mdt_core": docs
"mdt_lsp": docs
"mdt_mcp": docs
"@m-d-t/skills": docs
---

# Rewrite the documentation to match actual behavior

An audit ran every documented command, flag, default, and example against the CLI. The docs site, crate readmes, the annotated `mdt.toml` that `mdt init` writes, and generated doc comments now match the implementation:

- `[include]` and `[templates] paths` are documented as additive (they were described as narrowing the scan), `[check] comparison = "lenient"` no longer claims to normalize tables or JSON, exclude negation examples work, and formatter patterns are documented as plain globs.
- Exit codes (`0`/`1`/`2`), the global `--ignore-*` flags, project-root discovery, `mdt skill`, per-client `mdt assist` output, the complete JSON payload, GitHub annotations, and every quoted command output are current.
- Source-file examples re-apply comment prefixes with `linePrefix:"...":true`, so copied examples compile.
- Terminology is provider/consumer throughout.
- Crate readme badges render again (link definitions were joined onto one line).
- Links that left the book now point at GitHub, and the Pi link points at pi.dev.
