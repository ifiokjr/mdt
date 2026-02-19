---
mdt: minor
mdt_cli: minor
---

Add MCP server, security hardening, and quality improvements.

**MCP server (`mdt_mcp`):** New crate providing a Model Context Protocol server with 6 tools: `mdt_check`, `mdt_update`, `mdt_list`, `mdt_get_block`, `mdt_preview`, and `mdt_init`. The server communicates over stdin/stdout and can be launched via `mdt mcp`. Built on the `rmcp` SDK.

**Security hardening:**

- Add configurable file size limits (`max_file_size` in `mdt.toml`, default 10MB) to prevent denial-of-service from large files
- Add symlink cycle detection during directory walking to prevent infinite loops
- Normalize CRLF line endings (`\r\n` and `\r`) to LF (`\n`) on read for consistent cross-platform behavior
- Return proper errors for unconvertible float values (NaN, Infinity) in TOML and KDL data files instead of silently defaulting to 0

**Performance:**

- Optimize `offset_to_point` in the source scanner with a `LineTable` struct using binary search (O(log n) instead of O(n) per lookup)

**Quality:**

- Migrate LSP `DocumentSymbol` responses from deprecated `SymbolInformation` (flat) to hierarchical `DocumentSymbol` API
- Fix CLI exit codes: exit 1 for stale blocks (expected failure), exit 2 for actual errors

**Toolchain:**

- Bump `rust-toolchain.toml` from 1.87.0 to 1.88.0 (required by `rmcp` dependency)
