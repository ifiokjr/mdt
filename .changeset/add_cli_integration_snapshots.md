---
mdt: minor
mdt_cli: minor
---

Add comprehensive CLI integration tests using `insta-cmd` snapshot testing.

19 new integration tests covering `mdt check`, `mdt update`, and `mdt update --dry-run` across multiple scenarios:

- **pad_blocks with Rust doc comments**: Verifies `//!` and `///` doc comments are not mangled after update, with check/update/idempotency/diff snapshots.
- **pad_blocks with multiple languages**: Tests Rust, TypeScript (JSDoc), Python, and Go source files with data interpolation from `package.json`, ensuring all comment styles are preserved correctly.
- **Validation diagnostics**: Snapshots error output for unclosed blocks and verifies `--ignore-unclosed-blocks` bypasses the error.
- **includeEmpty on linePrefix**: Verifies the difference between `linePrefix` with and without `includeEmpty:true` — blank lines get the prefix when enabled.
- **TypeScript workspace**: Adds snapshot coverage for the existing fixture, including file content verification after update.

Also adds extra blank line padding in `pad_blocks` mode: when a comment prefix is present (e.g., `//!`, `///`, `*`), an additional blank line using that prefix is inserted between the opening tag and the content, and between the content and the closing tag.

Sorts file paths in `mdt update --dry-run` and `--verbose` output for deterministic ordering.
