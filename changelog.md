## 0.6.0 (2026-02-26)

### Breaking Changes

#### Unify all workspace crates under a single shared version.

Previously each crate (`mdt_core`, `mdt_cli`, `mdt_lsp`, `mdt_mcp`) maintained its own independent version, which led to version drift — e.g., `mdt_core` at 0.5.0 while `mdt_lsp` and `mdt_mcp` were still at 0.4.1. This made it harder to reason about compatibility and complicated the release process with per-crate changelogs and tag prefixes (`mdt_cli/v0.4.1`).

All crates now inherit `version = { workspace = true }` from the root `Cargo.toml` workspace version. The knope release configuration has been consolidated from four separate `[packages.*]` sections into a single `[package]` with all versioned files and dependencies listed together. Releases now use a single changelog (`changelog.md`) and simplified version tags (`v0.5.0` instead of `mdt_cli/v0.5.0`).

This is a breaking change to the release workflow and tag format, not to the library APIs.

### Fixes

- Add integration tests for positional block arguments in LSP and MCP servers.
- Add `mdt_mcp` to workspace members so it is built, tested, and published through normal CI workflows.
