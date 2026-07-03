---
mdt_mcp: minor
---

# Expose MCP diagnostics through MDT_LOG tracing

The MCP server now initializes `tracing-subscriber` with an `EnvFilter` sourced from `MDT_LOG`. Logs are emitted to stderr so tracing does not corrupt MCP stdio messages or structured tool responses.

This makes agent-driven workflows easier to debug when a check, update, preview, or init call behaves unexpectedly, while leaving normal tool output unchanged unless logging is explicitly enabled.
