---
"mdt_mcp": feat
---

# Confine MCP tool paths to the server's startup directory

Every tool accepted a caller-supplied `path` and resolved it verbatim, so an assistant could point `mdt_check`/`mdt_update`/`mdt_init` at any directory on disk — executing whatever `[data]` shell commands and formatters that directory's `mdt.toml` declares, and reading/writing files there. Paths (including relative `..` escapes and symlink targets) must now resolve inside the directory the server started in; anything else is rejected with an invalid-params error explaining how to restart the server in the project to manage. A new `MdtMcpServer::with_base_root` constructor sets the permitted root explicitly, and tool handlers run scans, updates, and init writes on the blocking thread pool so a slow script or formatter cannot freeze the stdio transport.
