---
"mdt_cli": change
---

# Make `mdt` fail loudly and precisely: orphans, exit codes, machine output, and a safer `init`

Evaluations of the CLI showed several ways a broken setup reported success. The CLI now surfaces every problem with a location and a fix.

**Exit codes.** `mdt check` exits `0` when every consumer is linked and current, `1` when a consumer is stale, names no provider, or fails to render, and `2` when validation or config errors stop the run. `mdt update` exits `1` when some consumers could not be rendered (it still updates the rest) and `2` on validation errors.

**Orphan consumers fail `mdt check`.** A consumer whose name matches no provider (for example a typo such as `featrues`) used to print one unlocated warning and pass. `check` now lists each one with its location and a did-you-mean suggestion; `update` warns with the same detail.

**Unused providers are warnings.** A provider without consumers no longer stops `check`, `update`, and `list` with exit `2`; it prints a warning that `--ignore-unused-blocks` silences. Warnings (unused providers, unmatched closing tags, providers outside `*.t.md` files) now print by default instead of only with `--verbose`.

**New diagnostics** with help text: nested blocks inside consumers, markdown comments that look like tags but do not parse (`--ignore-invalid-names` now does what it says), unmatched closing tags, and providers outside `*.t.md` files. The unclosed-block hint now points at misspelled closing tags, and the unknown-transformer hint lists every transformer, including `if`.

**Machine-readable output is complete.**

- `--format json` always returns `ok`, `stale`, `stale_files`, `orphans` (with `suggestions`), `errors` (render errors), and `diagnostics` (`severity`, `code`, `file`, `line`, `column`, `message`), including when validation errors stop the check.
- `--format github` emits `::error` for every failure (stale consumers, orphans, render errors, validation errors) and `::warning` for warnings, so validation errors now annotate pull requests too.
- In watch mode, status lines go to stderr so JSON on stdout stays parseable.

**`mdt list` always lists.** It prints every block with its line number, reports diagnostics, and only then exits `2` if there were errors, so it can be used to diagnose them.

**`--path` must exist.** A mistyped `--path` used to pass every command against an empty project and create a stray `.mdt/` there; it is now an error. `mdt init` still creates the directory.

**`mdt init` uses the shared core implementation.** It never modifies an existing README (of any case or extension), skips the sample provider when the project already has providers, syncs the sample through the engine, ignores `.mdt/` in git repositories, prints paths relative to the project, and always leaves the project passing `mdt check`.

**`mdt doctor`** reports orphan consumers once, with locations and suggestions, keeps block checks running when a data file fails to load, stops calling unused providers a parser failure, and adds a Consumer Sync check that renders every consumer to catch template errors and stale blocks.

**`mdt assist`** prints the correct setup for each client: `claude mcp add ... -- mdt mcp` and `.mcp.json` for Claude Code, `.cursor/mcp.json` for Cursor, `.vscode/mcp.json` with the `servers` key for GitHub Copilot in VS Code (it previously printed `mcpServers`), and CLI-plus-skill setup for Pi, which has no built-in MCP client. Every profile explains how to load the agent skill with `mdt skill` or `mdt skill --install <dir>`. JSON output adds `id`, `skill`, `mcp_config_file`, and `mcp_install_command`.

**Smaller fixes:** running `mdt` without a subcommand prints help; `update --dry-run` describes what it actually prints and conflicts with `--watch`; `--verbose` prints provider paths relative to the project.

**`mdt mcp --path <DIR>`** now serves that directory instead of ignoring the flag, so user-level MCP configs can pin a project.

**Running from a subdirectory finds the project.** Without `--path`, every command except `mdt init` uses the nearest directory, from the current one upward, that contains `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml` — like cargo and git. Running `mdt check` inside `docs/` no longer reports every consumer as an orphan.

`mdt list` shows transformer arguments (`|linePrefix:"//! ":true`), and the unrendered-data warning explains that a project without `[data]` copies `{{ ... }}` literally.

**CI output is complete for every failure.** `--format json` and `--format github` now also report scan failures (invalid `mdt.toml`, duplicate providers, unreadable files) instead of printing nothing on stdout, and template warnings (including unrendered `{{ ... }}` in projects without `[data]`) appear as `mdt::undefined_variables` warnings. GitHub annotation paths are relative to the working directory — the repository checkout in CI — so `mdt check --path packages/lib --format github` annotates `packages/lib/readme.md` rather than `readme.md`, and multi-line messages are escaped into a single workflow command.
