# mdt_mcp

> MCP server for mdt (manage markdown templates)

<br />

[![Crate][crate-image]][crate-link] [![Docs][docs-image]][docs-link] [![Status][ci-status-image]][ci-status-link] [![Coverage][coverage-image]][coverage-link] [![Unlicense][unlicense-image]][unlicense-link]

<br />

<!-- {=mdtMcpOverview} -->

`mdt_mcp` is a [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) server for [mdt](https://github.com/ifiokjr/mdt). It gives AI assistants structured, JSON-first access to a project's providers and consumers.

### Tools

- **`mdt_list`**: providers and consumers with locations, transformers, arguments, and status (`current`, `stale`, `render_error`, or `orphan`). Pass `include_content: true` to include block content.
- **`mdt_check`**: stale consumers, stale files, orphans with suggestions, render errors, and diagnostics.
- **`mdt_update`**: sync every consumer; `dry_run: true` previews. It refuses to write while validation errors exist.
- **`mdt_find_reuse`**: rank existing providers by name (`block_name`) or content (`content_query`) before you create a new one.
- **`mdt_preview`**: a provider and each consumer's rendered and current content.
- **`mdt_get_block`**: a provider (or `null`) and its consumers, by name.
- **`mdt_init`**: set up a project the same way `mdt init` does.

Every tool accepts an optional project `path`. `mdt_list`, `mdt_check`, and `mdt_update` also accept `ignore_unclosed_blocks`, `ignore_unused_blocks`, `ignore_invalid_names`, and `ignore_invalid_transformers`.

### Responses

Every response is a JSON object with `ok`, `action`, and `summary`. A failure is a normal tool result with `ok: false` and `error: { code, message, help }`, for example `mdt::config_parse` or `mdt::path_outside_root`, rather than a protocol error.

### Agent Workflow

- Call `mdt_find_reuse` (or `mdt_list`) before creating a provider, and reuse one that fits.
- Use `mdt_preview` to see what each consumer will contain before syncing.
- Keep provider names unique across the project.
- After editing providers, run `mdt_update`, then `mdt_check`.
- Run `mdt skill` for the full agent guide.

### Usage

Start the MCP server through the CLI. Its project root is the directory given with `--path`; without it, the server walks up from its working directory to the nearest directory with an mdt config file, like the CLI. Tool `path` arguments must resolve inside that root.

```sh
mdt mcp
```

Add it to your MCP client configuration:

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

`mdt assist <generic|claude|cursor|copilot|pi>` prints the exact setup for each client.

<!-- {/mdtMcpOverview} -->

## Installation

<!-- {=mdtMcpInstall} -->

```toml
[dependencies]
mdt_mcp = "0.9.6"
```

<!-- {/mdtMcpInstall} -->

<!-- {=mdtBadgeLinks:"mdt_mcp"} -->

[crate-image]: https://img.shields.io/crates/v/mdt_mcp.svg
[crate-link]: https://crates.io/crates/mdt_mcp
[docs-image]: https://docs.rs/mdt_mcp/badge.svg
[docs-link]: https://docs.rs/mdt_mcp/
[ci-status-image]: https://github.com/ifiokjr/mdt/workflows/ci/badge.svg
[ci-status-link]: https://github.com/ifiokjr/mdt/actions?query=workflow:ci
[coverage-image]: https://codecov.io/gh/ifiokjr/mdt/branch/main/graph/badge.svg
[coverage-link]: https://codecov.io/gh/ifiokjr/mdt
[unlicense-image]: https://img.shields.io/badge/license-Unlicense-blue.svg
[unlicense-link]: https://opensource.org/license/unlicense

<!-- {/mdtBadgeLinks} -->
