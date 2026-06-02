---
mdt_mcp: patch
---

# Reuse shared core helpers in MCP tools

MCP tools now use shared `mdt_core` helpers for project-root resolution and relative path display. Agent-facing responses therefore stay aligned with CLI and LSP behavior while preserving the existing MCP tool contract.

The cleanup reduces duplicated app-surface code and makes future fixes easier to apply consistently.
