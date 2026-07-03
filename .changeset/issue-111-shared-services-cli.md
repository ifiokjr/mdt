---
mdt_cli: patch
---

# Reuse shared core helpers in CLI surfaces

The CLI now uses shared `mdt_core` helpers for project-root resolution, relative path display, and similar-name scoring. This reduces duplicated logic between app surfaces while preserving the same command behavior and diagnostics users already expect.

Keeping these concerns in one place makes future fixes less likely to drift between CLI, LSP, and MCP integrations.
