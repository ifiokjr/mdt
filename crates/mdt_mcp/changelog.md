# Changelog

This file is maintained by `knope`.

## 0.0.1 (2026-02-20)

### Features

#### Add MCP server for mdt.

Provides a Model Context Protocol server with 6 tools: `mdt_check`, `mdt_update`, `mdt_list`, `mdt_get_block`, `mdt_preview`, and `mdt_init`. The server communicates over stdin/stdout and can be launched via `mdt mcp`. Built on the `rmcp` SDK.
