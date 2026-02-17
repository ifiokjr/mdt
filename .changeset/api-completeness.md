---
mdt: minor
mdt_cli: minor
---

Comprehensive API completeness improvements across all crates.

**New transformers:** Added `suffix`, `linePrefix`, and `lineSuffix` transformers for more granular content manipulation. `linePrefix` and `lineSuffix` apply prefixes/suffixes to each non-empty line, while `suffix` appends to the entire content. All support both camelCase and snake_case names.

**Duplicate provider detection:** The project scanner now detects and reports duplicate provider block names across template files, with clear error messages indicating both file locations.

**Rich diagnostics:** All error types now include `#[help(...)]` attributes with actionable guidance. Added `UnknownTransformer` and `InvalidTransformerArgs` error variants for better error reporting.

**Transformer validation:** New `validate_transformers()` function checks argument counts against expected ranges for each transformer type.

**Block PartialEq:** `Block`, `Transformer`, and `Argument` types now derive `PartialEq` for easier testing and comparison. Introduced `OrderedFloat` wrapper for approximate float equality.

**CLI enhancements:**

- `mdt check --diff` shows a colorized unified diff of stale consumer blocks
- `mdt check --format json|github|text` supports machine-readable output formats including GitHub Actions annotations
- `mdt list` command displays all providers and consumers with relationship status and transformer info
- `mdt update --watch` watches for file changes and re-runs updates with 200ms debouncing
- Colored terminal output with `NO_COLOR` env var and `--no-color` flag support

**Config improvements:** Added `[include]` patterns and `[templates]` paths sections to `mdt.toml` for controlling which files are scanned and where template files are located.

**LSP incremental updates:** The language server now performs incremental document updates on save instead of full project rescans, with full rescans only when `mdt.toml` changes.
