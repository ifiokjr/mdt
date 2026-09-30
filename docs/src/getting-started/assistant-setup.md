# Assistant Setup

Coding agents need two things to work well with mdt:

- **The agent skill** teaches the agent mdt's tags, transformers, configuration, and workflow. It works with any agent that can run a shell command.
- **The MCP server** (`mdt mcp`, optional) gives agents that support the Model Context Protocol structured tools for listing, previewing, checking, and updating blocks.

## Teach any agent with `mdt skill`

The skill is embedded in the `mdt` binary, so it always matches the installed version:

```sh
mdt skill              # print the skill (SKILL.md)
mdt skill --reference  # print the full reference (REFERENCE.md)
```

An agent can run these itself to load the skill into its conversation. To make the skill available to an agent automatically, install it into the directory the agent loads skills from:

```sh
mdt skill --install .claude/skills
```

This writes `.claude/skills/mdt/SKILL.md` and `.claude/skills/mdt/REFERENCE.md`, replacing any previous copy. Re-run it after upgrading mdt. Common directories:

| Agent                 | Skills directory                                           |
| --------------------- | ---------------------------------------------------------- |
| Claude Code           | `.claude/skills` (or `~/.claude/skills` for every project) |
| Shared / other agents | `.agents/skills`                                           |
| Pi                    | `.pi/skills`                                               |
| GitHub Copilot        | `.github/skills`                                           |

The same skill is published on npm as [`@m-d-t/skills`](https://github.com/ifiokjr/mdt/tree/main/packages/m-d-t__skills).

## Add project guidance

Agents that do not load skills automatically still pick up mdt when your project instructions tell them to. Add this to `AGENTS.md` (or `CLAUDE.md`, `.cursor/rules`, and so on):

```markdown
## Documentation (mdt)

- Before editing mdt templates or synced docs, run `mdt skill` to load the mdt agent skill (tag syntax, transformers, configuration, and workflow for the installed version).
- Edit providers in `.templates/*.t.md`, never the synced copies, and prefer reusing an existing provider (`mdt list` or `mdt_find_reuse`) over creating a new one.
- After documentation edits, run `mdt check`; run `mdt update` when consumers are stale.
```

## Per-client setup with `mdt assist`

`mdt assist <generic|claude|cursor|copilot|pi>` prints the skill command, MCP configuration, the guidance above, and client-specific notes. Add `--format json` for machine-readable output with `assistant`, `id`, `skill`, `mcp_config`, `mcp_config_file`, `mcp_install_command`, `notes`, and `repo_guidance`.

### Claude Code

```sh
mdt skill --install .claude/skills
claude mcp add --transport stdio --scope project mdt -- mdt mcp
```

The second command writes `.mcp.json` at the repository root; commit it to share the server with your team. Equivalent `.mcp.json`:

```json
{
	"mcpServers": {
		"mdt": {
			"args": ["mcp"],
			"command": "mdt",
			"type": "stdio"
		}
	}
}
```

### Cursor

Add to `.cursor/mcp.json` (this project) or `~/.cursor/mcp.json` (every project):

```json
{
	"mcpServers": {
		"mdt": {
			"args": ["mcp"],
			"command": "mdt"
		}
	}
}
```

Add the project guidance to `.cursor/rules` or `AGENTS.md` so the agent loads the skill with `mdt skill`.

### GitHub Copilot (VS Code)

```sh
mdt skill --install .github/skills
```

Add to `.vscode/mcp.json`. VS Code keys servers by `servers`, not `mcpServers`:

```json
{
	"servers": {
		"mdt": {
			"args": ["mcp"],
			"command": "mdt",
			"type": "stdio"
		}
	}
}
```

### Pi

[Pi](https://pi.dev) has no built-in MCP client, so it works with mdt through the CLI and the skill:

```sh
mdt skill --install .pi/skills
```

Or install the skill package from npm:

```sh
pi install npm:@m-d-t/skills   # install
pi -e npm:@m-d-t/skills        # try it for one session
```

To give every contributor the package, add it to `.pi/settings.json`:

```json
{
	"packages": ["npm:@m-d-t/skills"]
}
```

### Other MCP clients

```sh
mdt skill --install .agents/skills
```

Check which directory your agent loads skills from. Then register a stdio server in the client's MCP settings:

```json
{
	"mcpServers": {
		"mdt": {
			"args": ["mcp"],
			"command": "mdt"
		}
	}
}
```

## How the servers find your project

`mdt mcp` serves one project root. Without `--path`, it starts from the directory the client launches it in and walks up, within that git repository, to the nearest directory containing `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml`; outside a git repository, or with no config found, it uses the launch directory. A project-level config (launched in the repository) needs no extra arguments.

To point a client at another project, such as a sub-directory project in a monorepo or a user-level config, pass `--path`:

```json
{
	"mcpServers": {
		"mdt": {
			"args": ["mcp", "--path", "packages/docs"],
			"command": "mdt"
		}
	}
}
```

A relative `--path` resolves against the launch directory; use an absolute path in a user-level config. The directory must exist. Tool calls cannot reach paths outside the server root.

`mdt lsp` uses the first workspace folder your editor opens.

## MCP tools

The MCP server exposes `mdt_find_reuse`, `mdt_list`, `mdt_get_block`, `mdt_preview`, `mdt_check`, `mdt_update`, and `mdt_init`. Ask agents to call `mdt_find_reuse` or `mdt_list` before creating a new provider.

Every response is JSON with `ok`, `action`, and `summary`. A failure is returned as a tool result with `ok: false` and an `error` object (`code`, `message`, and sometimes `help`), for example `mdt::path_outside_root` for a path outside the server root.
