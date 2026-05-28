---
mdt_mcp: minor
---

Initialize `tracing-subscriber` with `EnvFilter` controlled by `MDT_LOG` environment variable, outputting to stderr to avoid interfering with the MCP stdio protocol.
