## 0.7.0 (2026-03-02)

### Breaking Changes

- Add `if` conditional transformer for selectively including block content based on data values. The `if` transformer takes a dot-separated data path as an argument and includes the block content only when the referenced value is truthy (exists and is not false, null, empty string, or zero). Example usage: `<!-- {=block|if:"config.features.enabled"} -->`.

#### Harden public API and improve crate documentation.

**Breaking (`mdt_core`):** Add `#[non_exhaustive]` to all 9 public enums (`MdtError`, `ParseDiagnostic`, `Argument`, `TransformerType`, `BlockType`, `PaddingValue`, `CodeBlockFilter`, `Token`, `DiagnosticKind`). This prevents downstream exhaustive pattern matching and allows new variants to be added in future minor releases without breaking changes. Downstream code matching on these enums must add a wildcard (`_`) arm.

**Breaking (`mdt_core`):** Make `source_scanner` module public (`pub mod source_scanner`). Previously it was private with items re-exported at the crate root via `pub use source_scanner::*`. The module is now directly accessible, fixing rustdoc link warnings for `[`source_scanner`]` references in crate-level documentation.

**`mdt_lsp`:** Add crate-level documentation using the `mdtLspOverview` template block, providing an overview of LSP capabilities and usage instructions directly in `lib.rs`.

**`mdt_mcp`:** Add crate-level documentation using the `mdtMcpOverview` template block, providing an overview of MCP tools and configuration directly in `lib.rs`.

**`mdt_cli`:** Add wildcard arms to `DiagnosticKind` match statements for forward compatibility with new diagnostic kinds.

**All crates:** Replace `tokio` `features = ["full"]` with minimal required feature sets — `mdt_cli` uses `["rt-multi-thread"]`, `mdt_lsp` uses `["rt-multi-thread", "macros", "sync", "io-std"]`, `mdt_mcp` uses `["rt-multi-thread", "macros"]`. This makes dependency requirements explicit.

**Config:** Remove stale data entries from `mdt.toml` (`cargo_mdt_core`, `cargo_mdt_cli`, `cargo_mdt_lsp`, `cargo_mdt_mcp`) that referenced individual crate `Cargo.toml` files no longer needed since crates use `version = { workspace = true }`.

#### Add a new public `Commands::Info` variant to `mdt_cli` and improve human-readable CLI output formatting (`mdt check` and new `mdt info`).

This is marked major because `Commands` is a public enum and adding a variant is a breaking change for exhaustive matches in downstream crates.

#### Expand config and data-source capabilities in `mdt_core`:

- Add config discovery precedence across `mdt.toml`, `.mdt.toml`, and `.config/mdt.toml`.
- Add typed `[data]` entries (`{ path, format }`) while keeping string-path compatibility.
- Add `ini` data format support.
- Expose new config/data APIs (`CONFIG_FILE_CANDIDATES`, `MdtConfig::resolve_path`, `DataSource`, `TypedDataSource`).

This is marked major because `MdtConfig.data` changes type from `HashMap<String, PathBuf>` to `HashMap<String, DataSource>`.

### Features

- Add `--watch` flag to `mdt check` command. When enabled, the check command monitors the project directory for file changes and automatically re-runs the check whenever files are modified or created. Uses 200ms debouncing to avoid redundant checks during rapid file changes. Unlike single-run mode, watch mode does not exit with a non-zero status code on stale consumers -- it prints the results and continues watching.
- Add `references` and `rename` support to the LSP server. `textDocument/references` returns all provider and consumer blocks sharing the same name. `textDocument/rename` renames a block name across all provider and consumer tags (both opening and closing tags) in the workspace.

#### Add cache observability across core and CLI diagnostics.

- Persist cache telemetry counters in the project index cache (scan count, full-hit count, cumulative reused/reparsed file counts, and last scan details).
- Expose cache inspection APIs from `mdt_core::project` for diagnostics surfaces.
- Extend `mdt info` with a cache section in text and JSON output (artifact health, schema/key compatibility, hash mode, cumulative metrics, and last scan summary).
- Extend `mdt doctor` with cache checks for artifact validity, hash mode guidance, and efficiency trend heuristics.
- Add unit/e2e/snapshot coverage and docs updates for the new observability output.

### Fixes

- Switch LSP text document synchronization from full to incremental mode. The server now receives only changed text ranges instead of the entire document content on each edit, improving performance for large files. Includes proper UTF-16 offset handling for LSP position conversion.

#### Fix collapsed newlines in `mdtBadgeLinks` provider block in `template.t.md`. The multi-line link reference definitions were accidentally collapsed to a single line by an external markdown formatter (`dprint fmt`) that doesn't recognize `{{ }}` template syntax in URLs as valid link definitions.

Restored the template content to its correct multi-line format. Added unit tests and CLI integration tests to verify that `mdt update` preserves newlines in multi-line content through the full scan → render → update pipeline, including idempotency after write-back.

### Documentation

- Fix broken markdown rendering in all crate READMEs. Badge reference links were collapsed onto a single line, causing them to render as plain text instead of clickable badge images. Each reference link definition now appears on its own line. Also added reusable mdt template blocks for LSP overview, MCP overview, CLI install, and contributing sections. Updated `mdt_core` title from `mdt` to `mdt_core`. Replaced stale `mdt_lsp` README content with mdt template blocks. Updated root README with crate table and streamlined contributing section.

## 0.6.0 (2026-02-26)

### Breaking Changes

#### Unify all workspace crates under a single shared version.

Previously each crate (`mdt_core`, `mdt_cli`, `mdt_lsp`, `mdt_mcp`) maintained its own independent version, which led to version drift — e.g., `mdt_core` at 0.5.0 while `mdt_lsp` and `mdt_mcp` were still at 0.4.1. This made it harder to reason about compatibility and complicated the release process with per-crate changelogs and tag prefixes (`mdt_cli/v0.4.1`).

All crates now inherit `version = { workspace = true }` from the root `Cargo.toml` workspace version. The knope release configuration has been consolidated from four separate `[packages.*]` sections into a single `[package]` with all versioned files and dependencies listed together. Releases now use a single changelog (`changelog.md`) and simplified version tags (`v0.5.0` instead of `mdt_cli/v0.5.0`).

This is a breaking change to the release workflow and tag format, not to the library APIs.

### Fixes

- Add integration tests for positional block arguments in LSP and MCP servers.
- Add `mdt_mcp` to workspace members so it is built, tested, and published through normal CI workflows.
