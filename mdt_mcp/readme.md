# mdt_mcp

> MCP server for mdt (manage markdown templates)

<br />

[![Crate][crate-image]][crate-link] [![Docs][docs-image]][docs-link] [![Status][ci-status-image]][ci-status-link] [![Coverage][coverage-image]][coverage-link] [![Unlicense][unlicense-image]][unlicense-link]

<br />

`mdt_mcp` provides a [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) server for the `mdt` template engine. It exposes mdt functionality as MCP tools that can be used by AI assistants and other MCP-compatible clients.

## Tools

The server provides the following tools:

- **`mdt_check`** — Verify all consumer blocks are up-to-date
- **`mdt_update`** — Update all consumer blocks with latest provider content
- **`mdt_list`** — List all providers and consumers in the project
- **`mdt_get_block`** — Get the content of a specific block by name
- **`mdt_preview`** — Preview the result of applying transformers to a block
- **`mdt_init`** — Initialize a new mdt project with a sample template file

## Usage

The MCP server communicates over stdin/stdout and can be launched via the `mdt` CLI:

```sh
mdt mcp
```

### Configuration

Add the following to your MCP client configuration:

```json
{
	"mcpServers": {
		"mdt": {
			"command": "mdt",
			"args": ["mcp"]
		}
	}
}
```

## Installation

<!-- {=mdtMcpInstall} -->

```toml
[dependencies]
mdt_mcp = "0.4.0"
```

<!-- {/mdtMcpInstall} -->

<!-- {=mdtBadgeLinks:"mdt_mcp"} -->

[coverage-image]: https://codecov.io/gh/ifiokjr/mdt/branch/main/graph/badge.svg
[coverage-link]: https://codecov.io/gh/ifiokjr/mdt
[crate-image]: https://img.shields.io/crates/v/mdt_mcp.svg
[crate-link]: https://crates.io/crates/mdt_mcp
[docs-image]: https://docs.rs/mdt_mcp/badge.svg
[docs-link]: https://docs.rs/mdt_mcp/
[ci-status-image]: https://github.com/ifiokjr/mdt/workflows/ci/badge.svg
[ci-status-link]: https://github.com/ifiokjr/mdt/actions?query=workflow:ci
[unlicense-image]: https://img.shields.io/badge/license-Unlicence-blue.svg
[unlicense-link]: https://opensource.org/license/unlicense

<!-- {/mdtBadgeLinks} -->
