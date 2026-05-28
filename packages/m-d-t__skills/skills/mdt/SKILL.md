---
name: mdt
description: Manage markdown templates with mdt. Synchronize README sections, source-doc comments, and docs-site content from shared provider blocks. Use when editing documentation, creating provider/consumer blocks, running mdt commands, or working with mdt MCP tools.
---

# mdt — Markdown Template Management

## Quick Start

```sh
# Initialize a project (creates .templates/template.t.md and mdt.toml)
mdt init

# Check all consumer blocks are up-to-date (CI-friendly, non-zero exit on stale)
mdt check

# Update all consumer blocks with latest provider content
mdt update

# List all providers and consumers
mdt list
```

## Core Workflow

1. **Define once** — Create provider blocks in `*.t.md` files (canonical location: `.templates/`):
   ```markdown
   <!-- {@blockName} -->

   Content defined once.

   <!-- {/blockName} -->
   ```

2. **Reuse everywhere** — Add consumer blocks in markdown or source files:
   ```markdown
   <!-- {=blockName} -->

   Replaced on update.

   <!-- {/blockName} -->
   ```

3. **Sync** — Run `mdt update` to push provider content into all consumers.

4. **Verify** — Run `mdt check` in CI to catch stale docs.

## MCP Tools (via `mdt mcp`)

When using the MCP server, **always call `mdt_find_reuse` before creating a new provider**:

| Tool             | Purpose                                                     |
| ---------------- | ----------------------------------------------------------- |
| `mdt_find_reuse` | Find similar providers and reuse opportunities — call first |
| `mdt_list`       | List all providers and consumers                            |
| `mdt_check`      | Verify consumers are up-to-date                             |
| `mdt_update`     | Sync all consumers                                          |
| `mdt_preview`    | Preview rendered output before committing                   |
| `mdt_get_block`  | Get a specific block's content                              |
| `mdt_init`       | Initialize a new mdt project                                |

## Key Rules

- Provider names are **globally unique** across all `*.t.md` files.
- Providers live only in `*.t.md` files. Consumer tags in source files (`.rs`, `.ts`, `.py`, `.go`, etc.) work inside code comments.
- Use transformers to adapt content: `<!-- {=block|trim|linePrefix:"/// ":true} -->`.
- Use `[padding]` in `mdt.toml` when consumers live in source files.
- `.templates/` is the canonical template directory.
- After editing providers, run `mdt check` then `mdt update`.

## Detailed Reference

For transformer reference, data interpolation, inline blocks, configuration options, and source-file patterns, see [REFERENCE.md](REFERENCE.md).
