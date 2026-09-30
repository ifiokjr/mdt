# CLI Reference

```text
mdt [OPTIONS] <COMMAND>
```

Running `mdt` without a command prints a short help and exits 2. Run `mdt <command> --help` for the full text of any command.

## Global options

These options work with every command.

| Option                          | Description                                                                                                     |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `-p`, `--path <DIR>`            | Project root. Must exist, except for `mdt init`, which creates it. See [Project root](#project-root).           |
| `-v`, `--verbose`               | Print more detail, including provider lists and warnings silenced by `--ignore-*` flags.                        |
| `--no-color`                    | Disable colored output. Overrides every color environment variable.                                             |
| `--ignore-unclosed-blocks`      | Do not fail on unclosed blocks or unmatched closing tags (`mdt::unclosed_block`, `mdt::unmatched_closing_tag`). |
| `--ignore-unused-blocks`        | Silence the warning for providers with no consumers (`mdt::unused_provider`).                                   |
| `--ignore-invalid-names`        | Skip comments that look like tags but do not parse, such as `{ @name }` or `{=my.block}` (`mdt::invalid_tag`).  |
| `--ignore-invalid-transformers` | Do not fail on unknown transformers or wrong argument counts.                                                   |
| `-h`, `--help`                  | Print help.                                                                                                     |
| `-V`, `--version`               | Print the version.                                                                                              |

A `--path` that does not exist is an error:

```text
error: project path `nope` does not exist
```

Warnings (unused providers, providers outside `*.t.md` files, undefined template variables, template variables in a project without `[data]`) print by default and never change the exit status.

## Project root

Without `--path`, every command except `mdt init` walks up from the current directory to the nearest directory that contains `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml`, and uses it as the project root, like `git` and `cargo` do. With no config file in any parent directory, the current directory is the root. `mdt init` always uses the current directory (or `--path`).

## Exit codes

| Command      | 0                                    | 1                                                                                            | 2                                                      |
| ------------ | ------------------------------------ | -------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| `mdt check`  | Every consumer is linked and current | A stale consumer, stale file, orphan consumer, or render error                               | Validation, config, or data errors                     |
| `mdt update` | Updated, or already up to date       | Some consumers were skipped because their provider failed to render; the others were updated | Validation, config, or data errors; nothing is written |
| `mdt list`   | Listing printed                      |                                                                                              | Validation errors (the listing still prints)           |
| `mdt doctor` | No check failed                      | At least one check reported `FAIL`                                                           |                                                        |

Validation errors are unclosed blocks, closing tags with no opening tag, blocks nested inside a consumer, comments that look like tags but do not parse, unknown transformers, wrong transformer argument counts, and duplicate providers. Argument errors, such as an unknown flag or `--dry-run` with `--watch`, also exit 2.

## Commands

### `mdt init`

Set up a project, adding only what is missing.

```sh
mdt init
mdt init --path ./my-project
```

- Writes an annotated `mdt.toml` unless a config file (`mdt.toml`, `.mdt.toml`, or `.config/mdt.toml`) exists.
- Writes a sample `greeting` provider to `.templates/template.t.md` unless a sample template exists or the project already has providers.
- Writes `readme.md` with the `greeting` consumer already synced when the project has no README of any case or extension. An existing README is never modified.
- In a git repository, adds `.mdt/` to `.gitignore`.
- Creates the `--path` directory if it does not exist.

The project passes `mdt check` afterwards. In a new git repository:

```text
Created mdt.toml
Created .templates/template.t.md with a sample `greeting` provider
Created readme.md with a synced `greeting` consumer
Added the `.mdt/` cache directory to .gitignore

Next steps:
  1. Open readme.md to see the synced sample block
  2. Edit .templates/template.t.md, then run `mdt update`
  3. Run `mdt check` in CI to fail builds on stale docs
```

With an existing `README.md`, init leaves it unchanged and suggests adding a consumer:

```text
Left the existing README.md unchanged
Added the `.mdt/` cache directory to .gitignore

Next steps:
  1. Add a consumer to README.md: <!-- {=greeting} --> <!-- {/greeting} -->
  2. Run `mdt update` to fill it in (until then `mdt check` warns that `greeting` has no consumers)
  3. Replace the sample in .templates/template.t.md with your own providers
```

### `mdt check`

Verify that every consumer is linked to a provider and up to date. Nothing is written.

```sh
mdt check
mdt check --diff
mdt check --format json
mdt check --format github
```

| Option              | Description                                  |
| ------------------- | -------------------------------------------- |
| `--diff`            | Show a unified diff for each stale consumer. |
| `--format <FORMAT>` | `text` (default), `json`, or `github`.       |
| `--watch`           | Re-run the check when files change.          |

A passing check prints:

```text
Check passed: all consumer blocks are up to date.
```

A failing check lists each problem with its location:

```text
Check failed.
  orphan consumers: 1
  stale consumers: 1

Orphan consumers:
  consumer `greting` at README.md:9:1 has no provider (did you mean `greeting`?)

Stale consumers:
  block `greeting` at README.md:3:1

1 consumer block(s) have no provider and 1 consumer block(s) are out of date. Rename those consumers or define the providers in `*.t.md` files, then run `mdt update`.
```

The sections are `Render errors:`, `Orphan consumers:`, `Stale consumers:`, and `Stale files:`, each shown only when it has entries. Validation errors print as diagnostics, followed by `error: validation errors found; fix the errors above`.

#### `--format github`

Prints one GitHub Actions annotation per problem, so failures appear inline on pull request diffs. Errors (stale consumers, stale files, orphans, render errors, and error diagnostics) use `::error`; warning diagnostics use `::warning`:

```text
::warning file=.templates/template.t.md,line=11,col=1::provider block `unused` has no consumers
::error file=readme.md,line=10,col=1::Template render failed for block `broken`: unknown filter: filter nope is unknown (template line 2)
::error file=readme.md,line=7,col=1::consumer `intor` at readme.md:7:1 has no provider (did you mean `intro`?)
::error file=readme.md,line=3,col=1::Consumer block `intro` is out of date; run `mdt update`
```

File paths are relative to the project root, so with `--path` they are relative to that directory.

#### `--format json`

<!-- {=mdtCheckJsonOutput} -->

`mdt check --format json` prints one object with every key present:

| Key           | Entries                                                                      |
| ------------- | ---------------------------------------------------------------------------- |
| `ok`          | `true` when every consumer is linked and current                             |
| `stale`       | Stale consumers: `file`, `block`, `line`, `column`                           |
| `stale_files` | Files a formatter would change: `file`                                       |
| `orphans`     | Consumers with no provider: `file`, `block`, `line`, `column`, `suggestions` |
| `errors`      | Render errors: `file`, `block`, `line`, `column`, `message`                  |
| `diagnostics` | Errors and warnings: `severity`, `code`, `file`, `line`, `column`, `message` |

A clean project:

```json
{
	"diagnostics": [],
	"errors": [],
	"ok": true,
	"orphans": [],
	"stale": [],
	"stale_files": []
}
```

A stale consumer, an orphan, and an unused-provider warning:

```json
{
	"diagnostics": [
		{
			"code": "mdt::unused_provider",
			"column": 1,
			"file": ".templates/template.t.md",
			"line": 11,
			"message": "provider block `unused` has no consumers",
			"severity": "warning"
		}
	],
	"errors": [],
	"ok": false,
	"orphans": [
		{
			"block": "intor",
			"column": 1,
			"file": "readme.md",
			"line": 7,
			"suggestions": ["intro"]
		}
	],
	"stale": [{ "block": "intro", "column": 1, "file": "readme.md", "line": 3 }],
	"stale_files": []
}
```

When validation errors (such as an unclosed or nested block) stop the check, the same object is printed with `ok: false` and the errors in `diagnostics`, and the exit status is 2. Errors that stop the scan itself (config and data errors, duplicate providers, unreadable files) print only the error report on stderr, also with exit status 2.

<!-- {/mdtCheckJsonOutput} -->

### `mdt update`

Write the latest provider content into every consumer.

```sh
mdt update
mdt update --dry-run
mdt update --watch
```

| Option      | Description                                                                                         |
| ----------- | --------------------------------------------------------------------------------------------------- |
| `--dry-run` | Print how many blocks and which files would change, without writing. Cannot be used with `--watch`. |
| `--watch`   | Update again whenever files change.                                                                 |

Output:

```text
Updated 3 block(s) in 2 file(s).
```

```text
All consumer blocks are already up to date.
```

```text
Dry run: would update 3 block(s) in 2 file(s):
  README.md
  src/lib.rs
```

With `[[formatters]]`, files where only the formatter output changed are reported separately:

```text
Normalized 1 file(s) via formatter integration.
```

Orphan consumers are reported as warnings; they cannot be updated, so `mdt check` still fails until you fix them. While orphans exist, the up-to-date message reads `All linked consumer blocks are already up to date.` A consumer whose provider fails to render is left untouched and reported, the other consumers are still updated, and the command exits 1:

```text
error: block `a` at r.md:1:1 was not updated: unknown filter: filter truncate is unknown (template line 3)
```

`--dry-run` shows which files would change; use `mdt check --diff` to see the content.

Watch mode prints the first result, then re-runs after each change:

```text
All consumer blocks are already up to date.

Watching for file changes... (press Ctrl+C to stop)

File change detected, updating...
Updated 2 block(s) in 2 file(s).
```

### `mdt list`

List every provider and consumer with its location.

```sh
mdt list
```

```text
Providers:
  @greeting .templates/template.t.md:1 (2 consumer(s))

Consumers:
  =greeting README.md:3 [linked]
  ~version README.md:7 [inline]
  =greting README.md:9 [orphan]
  =greeting src/lib.rs:1 |trim|linePrefix:"//! ":true [linked]

1 provider(s), 4 consumer(s)
```

| Status     | Meaning                                                 |
| ---------- | ------------------------------------------------------- |
| `[linked]` | The consumer has a matching provider.                   |
| `[orphan]` | No provider has this name.                              |
| `[inline]` | An inline block that renders its own template argument. |

Transformers and their arguments appear after the location. Diagnostics print before the listing. With validation errors the listing still prints and the command exits 2; errors that stop the scan, such as a config error or a duplicate provider, print only the error.

### `mdt info`

Print a summary of the project.

```sh
mdt info
mdt info --format json
```

Sections: `Project` (root and resolved config), `Blocks` (provider, consumer, orphan, and unused counts), `Data` (namespaces and sources), `Templates` (template files and directories), `Diagnostics` (error, warning, and missing-provider totals with names), and `Cache` (artifact path and status, schema version, hash verification, and reuse statistics). JSON output has the keys `project`, `blocks`, `data`, `templates`, `diagnostics`, and `cache`.

### `mdt doctor`

Run health checks with remediation hints. Exits 1 when any check fails.

```sh
mdt doctor
mdt doctor --format json
```

```text
mdt doctor
[PASS] Config Discovery       resolved config at /path/to/project/mdt.toml
[PASS] Data Sources           no data namespaces configured
[PASS] Template Layout        found `.templates/` directory
[PASS] Duplicate Providers    provider names are unique
[FAIL] Orphan Consumers       1 consumer block(s) have no provider: consumer `greting` at README.md:11:1 has no provider (did you mean `greeting`?)
       hint: rename the consumers to match a provider, or define the providers in `*.t.md` files
[PASS] Unused Providers       all providers have at least one consumer
[PASS] Parser Diagnostics     no parser diagnostics found
[PASS] Consumer Sync          every linked consumer renders and is up to date
[PASS] Cache Artifact         cache artifact is readable and valid at /path/to/project/.mdt/cache/index-v2.json
[PASS] Cache Hash Mode        content-hash verification disabled (mtime + size fingerprints only)
       hint: set `MDT_CACHE_VERIFY_HASH=1` to validate cache keys with content hashes while troubleshooting
[PASS] Cache Efficiency       healthy cache trend: 19 reused vs 6 reparsed (76.0% reused)

summary: 10 pass, 0 warn, 1 fail, 0 skip
```

Checks, in order:

- **Config Discovery / Config Parse**: warns when no config file is found; fails when it does not parse.
- **Data Sources**: fails when a `[data]` source does not load.
- **Template Layout**: warns when there is no `.templates/` directory but there is a legacy `templates/` directory or `[templates] paths`, and suggests `.templates/`.
- **Duplicate Providers**: fails when two providers share a name.
- **Orphan Consumers**: fails when a consumer names no provider, listing each location and suggestion.
- **Unused Providers**: warns when a provider has no consumers.
- **Parser Diagnostics**: fails for error diagnostics, warns for warnings only.
- **Consumer Sync**: renders every consumer; fails on render errors, warns when consumers are stale.
- **Cache Artifact, Cache Hash Mode, Cache Efficiency**: cache health, the hash verification mode, and reuse trends.

JSON output has `ok`, `summary` (`pass`, `warn`, `fail`, `skip` counts), and `checks` (each with `id`, `title`, `status`, `message`, and `hint`).

### `mdt skill`

Print the mdt agent skill so an AI coding assistant can learn mdt. The skill is embedded in the binary, so it always matches the installed version; it is the same skill published as `@m-d-t/skills`.

```sh
mdt skill                         # print SKILL.md
mdt skill --reference             # print REFERENCE.md, the detailed reference
mdt skill --install .claude/skills
```

| Option            | Description                                                                                                                             |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `--reference`     | Print `REFERENCE.md` instead of `SKILL.md`.                                                                                             |
| `--install <DIR>` | Write `SKILL.md` and `REFERENCE.md` to `<DIR>/mdt/`, replacing any previous copy. Relative paths resolve against the current directory. |

Common skill directories are `.claude/skills` (Claude Code), `.agents/skills`, `.github/skills` (GitHub Copilot), and `.pi/skills` (Pi). See [Assistant Setup](../getting-started/assistant-setup.md).

### `mdt assist`

Print setup for an AI assistant: how to load the skill, the MCP server configuration, and suggested instructions for your repository.

```sh
mdt assist claude
mdt assist cursor --format json
```

| Assistant | MCP setup                                                                                   |
| --------- | ------------------------------------------------------------------------------------------- |
| `claude`  | `claude mcp add --transport stdio --scope project mdt -- mdt mcp`, which writes `.mcp.json` |
| `cursor`  | `.cursor/mcp.json` with an `mcpServers` entry                                               |
| `copilot` | `.vscode/mcp.json` with a top-level `servers` entry and `"type": "stdio"`                   |
| `pi`      | No built-in MCP support; use the CLI with the skill                                         |
| `generic` | A standard `mcpServers` entry for other clients                                             |

JSON output has `id`, `assistant`, `strategy`, `skill` (`print_command`, `install_command`), `mcp_config`, `mcp_config_file`, `mcp_install_command`, `repo_guidance`, and `notes`.

### `mdt lsp`

Start the language server over stdin/stdout. Configure your editor to run `mdt lsp` for markdown and source files. The server uses the editor's workspace folder as the project root.

```sh
mdt lsp
```

It publishes the same diagnostics as `mdt check`, offers a quick fix that updates a stale block exactly as `mdt update` would, and supports completion, hover, go to definition, references, rename, and document symbols. It does not run `[[formatters]]`, so with formatters configured it can report a block as stale that `mdt check` accepts. See the [`mdt_lsp` readme](https://github.com/ifiokjr/mdt/blob/main/mdt_lsp/readme.md).

### `mdt mcp`

Start the MCP server over stdin/stdout for AI assistants.

```sh
mdt mcp
mdt mcp --path ./docs-site
```

The server's project root is the `--path` directory; without it, the server finds the root like the CLI does (see [Project root](#project-root)). Tool `path` arguments must resolve inside that root. Run `mdt assist <assistant>` for client configuration, and see the [`mdt_mcp` readme](https://github.com/ifiokjr/mdt/blob/main/mdt_mcp/readme.md) for the tool list.

## Environment variables

| Variable                | Effect                                                                                                                                   |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `NO_COLOR`              | Any value disables colored output.                                                                                                       |
| `CLICOLOR`              | `0` disables colored output.                                                                                                             |
| `CLICOLOR_FORCE`        | Any value other than `0` forces colored output, even when `NO_COLOR` is set or output is not a terminal. `--no-color` still wins.        |
| `MDT_CACHE_VERIFY_HASH` | Any value adds content hashes to cache fingerprints (normally file size, modification time, and on Unix change time) for stricter reuse. |
| `MDT_LOG`               | A tracing filter for debug logs on stderr, for example `MDT_LOG=debug` or `MDT_LOG=mdt_core=trace`.                                      |

## Files

mdt writes its cache to `.mdt/cache/` in the project root (`index-v2.json` for scan results, `data-v1.json` for command output). Every command that scans the project updates it, including `check`, `list`, and `doctor`. Add `.mdt/` to `.gitignore`; `mdt init` does this in git repositories.
