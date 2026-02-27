# CLI Reference

## Global options

```
mdt [OPTIONS] [COMMAND]
```

| Option            | Description                                                                |
| ----------------- | -------------------------------------------------------------------------- |
| `--path <DIR>`    | Set the project root directory. Defaults to the current directory.         |
| `--verbose`       | Enable verbose output (show provider/consumer counts, file lists).         |
| `--no-color`      | Disable colored output. Also respects the `NO_COLOR` environment variable. |
| `-h`, `--help`    | Print help.                                                                |
| `-V`, `--version` | Print version.                                                             |

## Commands

### `mdt init`

Create a sample `.templates/template.t.md` file with a getting-started example.

```sh
mdt init
mdt init --path ./my-project
```

If `.templates/template.t.md` exists (or legacy `template.t.md`/`templates/template.t.md` exists), prints a message and exits without overwriting.

Creates a file containing:

```markdown
<!-- {@greeting} -->

Hello from mdt! This is a provider block.

<!-- {/greeting} -->
```

### `mdt check`

Verify that all consumer blocks are up to date. Exits with code 1 if any are stale.

```sh
mdt check
mdt check --diff
mdt check --format json
mdt check --format github
```

| Option              | Description                                           |
| ------------------- | ----------------------------------------------------- |
| `--diff`            | Show a unified diff for each stale block.             |
| `--format <FORMAT>` | Output format: `text` (default), `json`, or `github`. |

**Exit codes:**

| Code | Meaning                          |
| ---- | -------------------------------- |
| 0    | All consumers are up to date.    |
| 1    | One or more consumers are stale. |

**Output formats:**

- **`text`** — Human-readable output. Lists stale blocks with file paths. Includes diff when `--diff` is set.
- **`json`** — Machine-readable JSON. Fields: `ok` (boolean), `stale` (array of `{file, block}`).
- **`github`** — GitHub Actions `::warning` annotations. Produces inline warnings on PR diffs.

### `mdt update`

Update all consumer blocks with the latest provider content.

```sh
mdt update
mdt update --dry-run
mdt update --watch
```

| Option      | Description                                              |
| ----------- | -------------------------------------------------------- |
| `--dry-run` | Show what would be updated without writing files.        |
| `--watch`   | Watch for file changes and re-run updates automatically. |

In normal mode, prints the number of blocks and files updated:

```
Updated 3 block(s) in 2 file(s).
```

If everything is already in sync:

```
All consumer blocks are already up to date.
```

**Dry run** shows what would change without modifying files:

```
Dry run: would update 3 block(s) in 2 file(s):
  readme.md
  src/lib.rs
```

**Watch mode** keeps running after the initial update, watching for file changes with 200ms debouncing:

```
Updated 3 block(s) in 2 file(s).

Watching for file changes... (press Ctrl+C to stop)

File change detected, updating...
All consumer blocks are already up to date.
```

Watch mode is not available with `--dry-run`.

### `mdt list`

Display all provider and consumer blocks in the project.

```sh
mdt list
```

Output:

```
Providers:
  @installGuide template.t.md (2 consumer(s))
  @apiDocs template.t.md (3 consumer(s))

Consumers:
  =installGuide readme.md [linked]
  =installGuide crates/my-lib/readme.md [linked]
  =apiDocs readme.md [linked]
  =apiDocs src/lib.rs |trim|indent [linked]
  =orphanBlock docs/old.md [orphan]

2 provider(s), 5 consumer(s)
```

**Status indicators:**

| Status     | Meaning                                      |
| ---------- | -------------------------------------------- |
| `[linked]` | Consumer has a matching provider.            |
| `[orphan]` | Consumer references a non-existent provider. |

Transformers are shown after the file path when present.

### `mdt info`

Print a human-readable diagnostics summary for the current project.

```sh
mdt info
mdt info --path ./my-project
```

Includes:

- Project root and resolved config path (`mdt.toml`, `.mdt.toml`, or `.config/mdt.toml`; or `none`).
- Provider/consumer counts, orphan consumers, and unused providers.
- Data namespaces and their configured source files.
- Template file count, discovered template files, and template directory hints.
- Diagnostic totals (errors/warnings) and missing provider names.

### `mdt lsp`

Start the language server for editor integration. Communicates over stdin/stdout using the Language Server Protocol.

```sh
mdt lsp
```

The LSP provides:

- **Diagnostics** — Warnings for stale consumers, missing providers, and provider blocks in non-template files.
- **Completions** — Provider name suggestions inside consumer tags. Transformer name suggestions after `|`.
- **Hover** — Shows provider content when hovering over consumer tags. Shows consumer count when hovering over provider tags.
- **Go to definition** — Jump from a consumer tag to its provider definition.
- **Document symbols** — Lists all blocks in the current file.
- **Code actions** — Quick-fix to update a stale consumer block in place.

### `mdt mcp`

Start the MCP server for AI integrations. Communicates over stdin/stdout using the Model Context Protocol.

```sh
mdt mcp
```

Use this command when you want an AI assistant to query template providers, consumers, and render context directly from your project.

## Environment variables

| Variable   | Effect                                                                  |
| ---------- | ----------------------------------------------------------------------- |
| `NO_COLOR` | When set (to any value), disables colored output. Same as `--no-color`. |
