---
mdt_cli: minor
---

Add `@ifi/mdt-skills` npm package — an official agent skill package for [Pi](https://github.com/badlogic/pi) and other harnesses supporting the [Agent Skills standard](https://agentskills.io).

The package includes a `skills/mdt/SKILL.md` with quick-start instructions and MCP tool guidance, plus a detailed `REFERENCE.md` covering the full template syntax, all transformers, data interpolation, inline blocks, configuration options, CLI commands, MCP server tools, and source-file patterns.

Install with:

```sh
pi install npm:@ifi/mdt-skills
```

Or try it for a single session:

```sh
pi -e npm:@ifi/mdt-skills
```

The skill is versioned alongside the CLI and published automatically during the npm publish workflow.

Additional changes:

- Updated `mdt assist pi` to mention the skill package in its notes.
- Updated build and publish scripts to generate and publish the skills package.
- Updated installation, assistant setup, and release documentation.
- Added integration tests for skills package generation and publishing.
