# @m-d-t/skills

The agent skill for [mdt](https://github.com/ifiokjr/mdt), the markdown template synchronization tool. It follows the [Agent Skills](https://agentskills.io) format (`skills/mdt/SKILL.md` plus `REFERENCE.md`) and teaches a coding agent how to:

- create and reuse provider and consumer blocks
- run `mdt check`, `mdt update`, and the other CLI commands, and fix what they report
- apply the right transformers for source-file doc comments (Rust, TypeScript, Python, Go, Dart, and more)
- configure `mdt.toml`, including data interpolation and formatter integration
- use the MCP tools (`mdt_find_reuse`, `mdt_preview`, `mdt_check`, and others)

## Installation

### Any agent: `mdt skill`

The same skill is embedded in the mdt CLI, so it always matches the installed version. Install it into the directory your agent loads skills from:

```sh
mdt skill --install .claude/skills   # Claude Code (~/.claude/skills for every project)
mdt skill --install .agents/skills   # Codex and other agents that read the shared directory
mdt skill --install .github/skills   # GitHub Copilot
mdt skill --install .pi/skills       # Pi
```

Each command writes `<DIR>/mdt/SKILL.md` and `<DIR>/mdt/REFERENCE.md`, replacing any previous copy. Check which directory your agent reads.

Any agent can also load the skill on demand, which is how `mdt assist cursor` sets up Cursor. Add this to `AGENTS.md` (or `.cursor/rules`):

```markdown
Before editing mdt templates or synced docs, run `mdt skill` to load the mdt agent skill, and `mdt skill --reference` for the full reference.
```

`mdt assist <claude|cursor|copilot|pi|generic>` prints the skill command and MCP setup for each client.

### Pi package

[Pi](https://pi.dev) installs this npm package directly and loads the skill from its `skills/` directory:

```sh
pi install npm:@m-d-t/skills
```

Or try it for a single session:

```sh
pi -e npm:@m-d-t/skills
```

To give every contributor the skill, add it to the project's `.pi/settings.json`:

```json
{
	"packages": ["npm:@m-d-t/skills"]
}
```

## Requirements

- The [mdt CLI](https://github.com/ifiokjr/mdt): `npm install -g @m-d-t/cli` (prebuilt) or `cargo install mdt_cli` (builds from source)
- For MCP tools: the mdt MCP server (`mdt mcp`) configured in your agent

## What the skill covers

`SKILL.md` is the everyday guide; `REFERENCE.md` (`mdt skill --reference`) has the details.

| Topic              | Covers                                                                                             |
| ------------------ | -------------------------------------------------------------------------------------------------- |
| Tags               | Provider (`{@}`), consumer (`{=}`), inline (`{~}`), and close (`{/}`) tags, block names, arguments |
| Transformers       | `trim`, `indent`, `linePrefix`, `codeBlock`, `replace`, `if`, and the rest, with argument rules    |
| Source files       | Comment prefixes for Rust, TypeScript, Python, Go, Dart, and other scanned languages               |
| Data interpolation | JSON, TOML, YAML, KDL, INI, text, and command-backed `[data]` sources                              |
| Configuration      | `mdt.toml`: `[padding]`, `[exclude]`, `[include]`, `[templates]`, `[check]`, `[[formatters]]`      |
| Diagnostics and CI | What `mdt check` failures mean, exit codes, JSON output, CI setup                                  |
| CLI and agents     | `mdt` commands including `mdt skill` and `mdt assist`, plus the MCP server tools                   |

## Links

- [mdt repository](https://github.com/ifiokjr/mdt)
- [Documentation](https://ifiokjr.github.io/mdt/)
- [CLI package](https://www.npmjs.com/package/@m-d-t/cli)
