# Assistant Setup

mdt's official assistant profiles are intentionally lightweight: they provide ready-to-copy MCP configuration snippets and repo-local guidance, not a marketplace or plugin registry.

## Recommended workflow

1. Run `mdt assist <assistant>` to print an official setup profile.
2. Copy the MCP snippet into your assistant's configuration.
3. Add the suggested repo-local guidance to your project instructions.
4. Let the assistant inspect and synchronize docs through `mdt mcp`.

## Example

```sh
mdt assist claude
```

This prints:

- an MCP server configuration snippet that runs `mdt mcp`
- repo-local guidance such as reusing providers before creating new ones
- assistant-specific notes for the selected profile

## Why this approach

The goal is to reduce setup friction without inventing a new extension ecosystem.

The first official profiles focus on:

- **portable MCP configuration** — the same `mdt mcp` server can be reused across assistants
- **repo-local guidance** — your assistant should follow the same mdt workflow every time
- **human-controlled adoption** — you can copy, review, and customize the generated setup before using it

## Repo-local guidance to keep

Regardless of assistant, keep guidance like this close to your project instructions:

- Prefer reuse before creation: run `mdt_find_reuse` or `mdt_list` before introducing a new provider block.
- Use `.templates/` as the canonical template location.
- Use `mdt_preview` to inspect provider and consumer output before syncing changes.
- Run `mdt_check` after documentation edits and `mdt_update` when consumer blocks are stale.

## Agent skill package

For [Pi](https://github.com/badlogic/pi) users, install the official mdt skill package to give your agent full knowledge of template syntax, MCP tools, CLI workflows, and configuration:

```sh
pi install npm:@ifi/mdt-skills
```

Or try it for a single session:

```sh
pi -e npm:@ifi/mdt-skills
```

The skill package teaches agents how to create and manage provider/consumer blocks, apply transformers for source-file doc comments, use MCP tools with best practices, and configure `mdt.toml`.

For project-level adoption, add it to `.pi/settings.json` so every contributor gets the skill automatically:

```json
{
  "packages": ["npm:@ifi/mdt-skills"]
}
```

## Supported first-slice profiles

- `generic`
- `claude`
- `cursor`
- `copilot`
- `pi`

As the project evolves, these profiles can grow into richer setup helpers, but the initial focus is pragmatic: make assistant setup reproducible and easy to adopt.
