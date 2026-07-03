---
mdt_mcp: patch
---

# Update rmcp server result construction

`rmcp` has been updated to 2.1.0. The MCP server now uses `ServerInfo::new().with_instructions()` for server metadata, the `CallToolResult::success()` and `CallToolResult::error()` constructors, and `ContentBlock` for text tool responses.

This keeps the MCP integration aligned with the current `rmcp` API while preserving the same tool behavior for clients.
