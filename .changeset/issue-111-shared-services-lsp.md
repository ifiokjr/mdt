---
mdt_lsp: patch
---

# Reuse shared core helpers in the LSP server

The language server now uses shared `mdt_core` helpers for project-root resolution and relative path display. This keeps editor diagnostics aligned with the CLI and MCP surfaces without changing the LSP protocol behavior.

Centralizing this logic reduces the chance that path formatting or project discovery diverges across integrations.
