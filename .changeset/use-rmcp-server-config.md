---
mdt: fix
---

# Use `rmcp::model::ServerConfig` instead of the deprecated `ServerInfo`

`rmcp` renamed the `ServerInfo` type alias to `ServerConfig` and deprecated the old name. The `ci/test` jobs on macOS and Windows compile with `-D warnings`, so the deprecation became a hard build failure as soon as the release lockfile picked up `rmcp` 3.4.0, blocking the release. The workspace now requires `rmcp` 3.4.0 and `mdt_mcp` uses the new type name.
