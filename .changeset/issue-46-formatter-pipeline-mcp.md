---
mdt_mcp: minor
---

# Run configured formatters from MCP tools

MCP `mdt_update` and `mdt_check` now honor opt-in `[[formatters]]` configuration. Agent workflows can preview, update, and verify formatter-aware template output using the same configuration as the CLI.

This keeps MCP-driven synchronization aligned with project formatters while preserving structured diagnostics for formatter failures.
