---
mdt_mcp: patch
---

# Update rmcp server result construction

`rmcp` has been updated to 1.3.0. The MCP server now uses `ServerInfo::new().with_instructions()` for server metadata and the `CallToolResult::success()` and `CallToolResult::error()` constructors for tool responses.

This keeps the MCP integration aligned with the current `rmcp` API while preserving the same tool behavior for clients.
