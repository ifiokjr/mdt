---
mdt_core: patch
mdt_cli: patch
mdt_lsp: patch
mdt_mcp: patch
---

Extract shared app-surface helpers for project-root resolution, relative path display, and similar-name scoring into `mdt_core`, then adopt them across CLI, LSP, and MCP to reduce drift without changing user-facing behavior.
