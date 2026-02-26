---
mdt_core: major
mdt_cli: patch
mdt_lsp: minor
mdt_mcp: minor
---

Harden public API and improve crate documentation.

**Breaking (`mdt_core`):** Add `#[non_exhaustive]` to all 9 public enums (`MdtError`, `ParseDiagnostic`, `Argument`, `TransformerType`, `BlockType`, `PaddingValue`, `CodeBlockFilter`, `Token`, `DiagnosticKind`). This prevents downstream exhaustive pattern matching and allows new variants to be added in future minor releases without breaking changes. Downstream code matching on these enums must add a wildcard (`_`) arm.

**Breaking (`mdt_core`):** Make `source_scanner` module public (`pub mod source_scanner`). Previously it was private with items re-exported at the crate root via `pub use source_scanner::*`. The module is now directly accessible, fixing rustdoc link warnings for `[`source_scanner`]` references in crate-level documentation.

**`mdt_lsp`:** Add crate-level documentation using the `mdtLspOverview` template block, providing an overview of LSP capabilities and usage instructions directly in `lib.rs`.

**`mdt_mcp`:** Add crate-level documentation using the `mdtMcpOverview` template block, providing an overview of MCP tools and configuration directly in `lib.rs`.

**`mdt_cli`:** Add wildcard arms to `DiagnosticKind` match statements for forward compatibility with new diagnostic kinds.

**All crates:** Replace `tokio` `features = ["full"]` with minimal required feature sets — `mdt_cli` uses `["rt-multi-thread"]`, `mdt_lsp` uses `["rt-multi-thread", "macros", "sync", "io-std"]`, `mdt_mcp` uses `["rt-multi-thread", "macros"]`. This makes dependency requirements explicit.

**Config:** Remove stale data entries from `mdt.toml` (`cargo_mdt_core`, `cargo_mdt_cli`, `cargo_mdt_lsp`, `cargo_mdt_mcp`) that referenced individual crate `Cargo.toml` files no longer needed since crates use `version = { workspace = true }`.
