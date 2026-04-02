# @ifi/mdt-skills

Agent skills for [mdt](https://github.com/ifiokjr/mdt) — the markdown template synchronization tool.

## What's inside

This package provides a **pi-compatible agent skill** that teaches your coding agent how to work with mdt. When installed, agents can:

- Create and manage provider/consumer template blocks
- Run `mdt check`, `mdt update`, and other CLI commands correctly
- Use MCP tools (`mdt_find_reuse`, `mdt_preview`, `mdt_check`, etc.) with best practices
- Apply the right transformers for source-file doc comments (Rust, TypeScript, Python, Go, etc.)
- Configure `mdt.toml` for data interpolation, padding, and scanning rules

## Installation

### As a pi package

```sh
pi install npm:@ifi/mdt-skills
```

Or try it for a single session:

```sh
pi -e npm:@ifi/mdt-skills
```

### As a project-level dependency

Add to your `.pi/settings.json`:

```json
{
  "packages": ["npm:@ifi/mdt-skills"]
}
```

Then any contributor running `pi` in the project will automatically get the skill.

## Requirements

- [mdt CLI](https://github.com/ifiokjr/mdt) installed (`npm install -g @ifi/mdt` or `cargo install mdt_cli`)
- For MCP integration: an mdt MCP server configured in your agent (`mdt mcp`)

## What the skill covers

| Topic | Description |
|-------|-------------|
| Template syntax | Provider (`{@}`), consumer (`{=}`), inline (`{~}`), close (`{/}`) tags |
| Transformers | `trim`, `indent`, `linePrefix`, `codeBlock`, `replace`, `if`, and more |
| Data interpolation | Pull values from JSON, TOML, YAML, KDL, INI, or script output |
| Source file support | Consumer blocks in `.rs`, `.ts`, `.py`, `.go`, `.java`, etc. |
| MCP tools | Full reference for all 7 MCP server tools with best practices |
| Configuration | `mdt.toml` sections: `[data]`, `[padding]`, `[exclude]`, `[templates]` |
| CLI commands | `init`, `check`, `update`, `list`, `info`, `doctor`, `assist`, `lsp`, `mcp` |

## Links

- [mdt repository](https://github.com/ifiokjr/mdt)
- [Documentation](https://ifiokjr.github.io/mdt/)
- [CLI binary package](https://www.npmjs.com/package/@ifi/mdt)
