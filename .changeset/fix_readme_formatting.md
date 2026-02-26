---
mdt_core: docs
mdt_cli: docs
mdt_lsp: docs
mdt_mcp: docs
---

Fix broken markdown rendering in all crate READMEs. Badge reference links were collapsed onto a single line, causing them to render as plain text instead of clickable badge images. Each reference link definition now appears on its own line. Also added reusable mdt template blocks for LSP overview, MCP overview, CLI install, and contributing sections. Updated `mdt_core` title from `mdt` to `mdt_core`. Replaced stale `mdt_lsp` README content with mdt template blocks. Updated root README with crate table and streamlined contributing section.
