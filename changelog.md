## [0.9.6](https://github.com/ifiokjr/mdt/releases/tag/v0.9.6) (2026-09-30)

Grouped release for `mdt`.

### 🚀 Feature

- **Add `FormatterRuleSet` for precompiled formatter globs.** `FormatterRuleSet::compile` turns a formatter's `patterns` or `ignore` list into reusable glob matchers with gitignore-style ordered `!` negation semantics, and `FormatterConfig::matches_file` keeps its behavior. Check and update runs compile the rules once per run instead of recompiling every glob for every scanned file. _Packages:_ 🟠 _mdt_core_ _Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #194](https://github.com/ifiokjr/mdt/pull/194)
- **Support hyphenated block names and Dart source files.** Block names may now contain hyphens (`install-command`), matching kebab-case conventions; previously such tags failed tokenization and were silently ignored — they never appeared in `mdt list` or `mdt check`. `.dart` files are now scanned for consumer and inline blocks by default, like other supported source languages, so Dart doc comments can be synchronized from shared providers. Use `[include] patterns` to opt unlisted extensions in on older versions. _Packages:_ 🟠 _mdt_core_ _Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #198](https://github.com/ifiokjr/mdt/pull/198)

#### Add `init::init_project`, the shared implementation behind `mdt init` and `mdt_init`

_Packages:_ 🟠 _mdt_core_

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #201](https://github.com/ifiokjr/mdt/pull/201)

`mdt_core::init::init_project(root)` adds only what a project is missing and reports what it did through `InitReport` (`SampleOutcome`, `ConfigOutcome`, `GitignoreOutcome`):

- writes the annotated starter `mdt.toml` unless `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml` exists;
- writes a sample `greeting` provider to `.templates/template.t.md` unless a sample template or any provider already exists, so it never introduces a duplicate provider into a project that uses mdt without a config;
- writes `readme.md` with the sample consumer already synced through the engine (so an existing `[padding]` is honoured) when the project has no README of any case or extension, and never modifies an existing README;
- adds `.mdt/` to `.gitignore` in git repositories so the local cache is not committed;
- creates the root directory when it does not exist.

The CLI and the MCP server both call it, so they produce the same files and always leave the project passing `mdt check`.

#### Add `mdt skill` to load the agent skill straight from the CLI

_Packages:_ 🟠 _mdt_cli_

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #200](https://github.com/ifiokjr/mdt/pull/200)

The mdt agent skill (published separately as `@m-d-t/skills`) is now embedded in the `mdt` binary, so any AI coding agent can learn mdt without installing the skill package, and the instructions always match the installed version.

- `mdt skill` prints `SKILL.md`, the entrypoint an agent reads first.
- `mdt skill --reference` prints `REFERENCE.md`, the full syntax, transformer, and configuration reference.
- `mdt skill --install <DIR>` writes both files to `<DIR>/mdt/` (for example `mdt skill --install .claude/skills`), replacing an older copy.

```sh
mdt skill | head
mdt skill --install .claude/skills
```

`mdt --help` lists the new command so agents discover it on their own.

- **Confine MCP tool paths to the server's startup directory.** Every tool accepted a caller-supplied `path` and resolved it verbatim, so an assistant could point `mdt_check`/`mdt_update`/`mdt_init` at any directory on disk — executing whatever `[data]` shell commands and formatters that directory's `mdt.toml` declares, and reading/writing files there. Paths (including relative `..` escapes and symlink targets) must now resolve inside the directory the server started in; anything else is rejected with an invalid-params error explaining how to restart the server in the project to manage. A new `MdtMcpServer::with_base_root` constructor sets the permitted root explicitly, and tool handlers run scans, updates, and init writes on the blocking thread pool so a slow script or formatter cannot freeze the stdio transport. _Packages:_ 🟠 _mdt_mcp_ _Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #194](https://github.com/ifiokjr/mdt/pull/194)

#### Rewrite the mdt skill from end-to-end agent evaluations and ship it inside the CLI

_Packages:_ 🟠 _@m-d-t/skills_

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #202](https://github.com/ifiokjr/mdt/pull/202)

The skill was rebuilt from evaluations where agents adopted mdt in polyglot monorepos using only the skill. It is now also embedded in the `mdt` binary: agents without the skill installed can run `mdt skill` (and `mdt skill --reference`), and `mdt skill --install <dir>` installs it for Claude Code, Pi, Copilot, or any agent that reads `.agents/skills`.

- A shorter `SKILL.md` covering the workflow, block rules, transformers, per-language comment prefixes (including indented class methods and `impl` blocks), data, formatter recipes verified with dprint, prettier, and rustfmt, CI and exit codes, monorepos with shared providers, and a table mapping each `mdt check` failure to its fix.
- A complete `REFERENCE.md`: tag syntax, block arguments, the transformer table, exact padding semantics, data formats and script caching, every `mdt.toml` key, scanning and git-ignore rules, diagnostic codes, CLI exit codes and JSON output, and MCP tool contracts.
- Corrects claims that were wrong: `[include]` and `[templates] paths` add to the scan, `lenient` comparison only handles trailing whitespace and blank lines, unclosed tags are errors, blocks cannot be nested, formatter placeholders must stay double-quoted, and a `replace` example no longer closes the comment it sits in.

- **Rewrite the mdt skill around formatter conflicts, code files, and stale-doc prevention.** The skill now teaches the `mdt update → formatter → mdt check` loop with the built-in escapes (`[[formatters]]`, `[check] comparison = "lenient"`, formatter-stable providers), adds complete TypeScript, Rust, and Dart consumer examples with `[padding]` semantics, documents the block-name charset and the silent-skip behavior for unscanned extensions, adds install/upgrade guidance for `@m-d-t/cli`, corrects the closing-tag prefix behavior, and fixes broken code fences in the data-interpolation reference. _Packages:_ 🟠 _@m-d-t/skills_ _Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #198](https://github.com/ifiokjr/mdt/pull/198)

### 📝 Changed

#### Catch silent failures: orphan consumers fail checks, malformed tags are reported, unknown config keys are rejected

_Packages:_ 🟠 _mdt_core_

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #201](https://github.com/ifiokjr/mdt/pull/201)

Several mistakes used to pass `mdt check` silently. They are now reported, and the severities match how much damage each one can do.

- **Orphan consumers fail the check.** `CheckResult` gains `orphans`: consumers whose name matches no provider, each with a location and `suggestions` of similar provider names. `CheckResult::is_ok()` is false while any exist. A misspelled `<!-- {=featrues} -->` used to leave the docs silently unsynced while CI stayed green.
- **Unused providers are warnings, not errors.** `ProjectDiagnostic::is_error` no longer treats a provider without consumers as an error, so adding a provider before wiring its consumers (or running `mdt init` in a repository that already has a README) no longer blocks `check`, `update`, and `list`. The new `ProjectDiagnostic::is_ignored` reports whether `--ignore-unused-blocks` (or another ignore flag) silences a diagnostic.
- **New diagnostics** (`DiagnosticKind` and `ParseDiagnostic` variants):
  - `NestedBlock` (error): a block that opens inside a consumer or inline block. `mdt update` replaced everything between the outer tags and silently destroyed the inner block.
  - `InvalidTag` (error, silenced by `ignore_invalid_names`): a markdown comment that looks like a tag — `{` followed by `@`, `=`, `~`, or `/` — but does not parse, such as `<!-- { @name } -->` or `<!-- {=my.block} -->`. These were silently ignored.
  - `UnmatchedClosingTag` (warning): a `{/name}` with no open block of that name, usually the other half of a misspelled tag.
  - `ProviderOutsideTemplate` (warning): a provider tag outside a `*.t.md` file, which mdt ignores.
- **Unknown `mdt.toml` keys are rejected.** Every config table now denies unknown fields, so `[paddding]` or `max_filesize` fails with the offending key and the config path instead of being ignored. Config parse errors name the config file.
- **The closing tag starts on its own line by default.** Without a `[padding]` section, the default is now `before = 0`, `after = 0`. The old `after = false` default glued the closing tag to the last content line, which broke every trimmed `codeBlock` consumer (`` ```<!-- {/x} --> `` is not a closing fence, so the fence swallowed the rest of the file) and dropped the comment prefix from source-file closing tags. Existing consumers in projects without `[padding]` will report stale once; run `mdt update`.
- **Text data drops one trailing newline**, as shell `$(...)` does. `release = { command = "cat VERSION", format = "text" }` now renders `1.2.3` rather than `1.2.3\n`, so inline values stay on one line.
- **TOML and KDL integers stay integers.** `port = 8080` renders as `8080`, not `8080.0`, and large integers keep full precision.

The index cache schema moves to version 3 so cached scans pick up the new diagnostics.

- **Unmatched closing tags are errors.** A `{/name}` with no open block is the other half of a misspelled or malformed opening tag, so that block silently stops syncing — in source-file comments nothing else reports it. `--ignore-unclosed-blocks` downgrades it like an unclosed block.
- **Unrendered data references warn.** In a project without `[data]`, providers are copied verbatim; a used provider containing namespaced variables such as `{{ pkg.version }}` now produces a `TemplateWarning` with `template_rendered: false`, instead of copying the braces silently (typically a sub-project reusing shared providers without declaring its own `[data]`).
- **Shared providers are never "unused".** Providers read from a `[templates] paths` directory outside the project are a shared library, so each project may use only some of them without warnings.
- `DiagnosticKind::code()` returns the stable `mdt::*` code for every diagnostic kind, and duplicate-provider errors name both definitions as project-relative `file:line`.
- **Blocks never nest.** `NestedBlock` now also covers blocks inside a provider: their tags would be copied into every consumer, where they nest. The error now appears at the provider instead of in every consumer.

#### Make `mdt` fail loudly and precisely: orphans, exit codes, machine output, and a safer `init`

_Packages:_ 🟠 _mdt_cli_

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #201](https://github.com/ifiokjr/mdt/pull/201)

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

Upward config discovery stays inside the current git repository (and does not happen outside one), so a stray `mdt.toml` in a parent directory such as `$HOME` can never become the project root. When the root is not the current directory, commands print `note: using the mdt project at <path>`. `mdt init` warns when it creates a project nested inside another one, and init failures name the directory.

#### MCP tools now agree with the CLI and report every failure as a structured result

_Packages:_ 🟠 _mdt_mcp_

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #201](https://github.com/ifiokjr/mdt/pull/201)

The MCP tools re-implemented parts of `mdt check`, `mdt update`, and `mdt init`, and drifted from them. They now call the same `mdt_core` functions, so an agent sees exactly what the CLI would report.

- **Staleness matches `mdt check`.** `mdt_list`, `mdt_get_block`, and `mdt_preview` take each block's status from `check_project`, so `[padding]`, lenient comparison, and `[[formatters]]` are honoured. Consumer entries gain `type` (`consumer` or `inline`), `line`, `column`, `arguments`, and `status` (`current`, `stale`, `render_error`, `orphan`); inline blocks get a status too.
- **Validation is no longer skipped.** `mdt_check`, `mdt_update`, and `mdt_list` return `diagnostics` (`kind`, `severity`, `file`, `line`, `column`, `message`), and `ok` is false when any is an error. `mdt_update` refuses to write while validation errors exist, like the CLI. All three accept `ignore_unclosed_blocks`, `ignore_unused_blocks`, `ignore_invalid_names`, and `ignore_invalid_transformers`, mirroring the `--ignore-*` flags.
- **Orphan consumers fail `mdt_check`** and are listed in `orphans` with suggested provider names.
- **Render errors are reported, not hidden.** `mdt_update` returns `render_errors` and syncs everything else instead of failing the request; `mdt_get_block` and `mdt_preview` report `render_error` with `ok: false` instead of falling back to the raw template.
- **`mdt_init` is `mdt init`.** It calls `mdt_core::init::init_project` and returns the `config`, `sample`, and `gitignore` outcomes, `written_files` relative to the initialized root, and `next_steps` with correctly written consumer tags.
- **Tool-level failures are `isError` results**, not JSON-RPC errors that many clients hide from the model: an invalid `mdt.toml`, a missing data file, duplicate providers, a failing formatter, or a bad `path` return `{ ok: false, action, summary, error: { code, message, help? } }` with the diagnostic code (for example `mdt::config_parse`). A `path` that does not exist or is not a directory is now an error instead of an empty, passing project.
- **Consistent responses.** Every tool returns `ok`, `action`, and `summary`. `mdt_get_block` returns one object shape, `{ ok, action, summary, block_name, provider, consumers }`, instead of a bare array for consumer lookups.
- **Smaller `mdt_list`.** Provider bodies are omitted unless `include_content` is true.
- **Better reuse search.** `mdt_find_reuse` ranks exact names, then names equal up to case and separators, prefixes, substrings, and close spellings, leaves unrelated providers out, reports the `match` kind, and accepts a `content_query` to search provider bodies. The `limit` schema now declares its 1–20 range.
- **Protocol.** The server identifies as `mdt` with the crate version, read-only tools carry `readOnlyHint`, tool descriptions say what each returns, and the instructions point agents at `mdt skill`.
- **Startup.** `run_server_in(root)` serves a chosen directory (`run_server()` still serves the current one), and `MDT_LOG=info mdt mcp` no longer panics when the CLI has already installed a tracing subscriber.

Tool handlers now return `CallToolResult` directly, and `PathParam` is replaced by `CheckParam` and `ListParam`.

- Diagnostics include the same `code` as `mdt check --format json` (for example `mdt::unclosed_block`), and `warnings` entries report `template_rendered`.

### 🐛 Fixed

- **Feed formatter stdin from a dedicated thread.** Formatter commands received their full input through the pipe before stdout was drained. A formatter that fills its stdout pipe while mdt is still writing a large file deadlocked both processes. stdin is now written from a spawned thread while the main thread reads output, and a formatter that exits early is reported through its exit status instead of a broken-pipe I/O error. _Packages:_ 🟢 _mdt_core_ _Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #194](https://github.com/ifiokjr/mdt/pull/194)

#### Fix scanning boundaries, symlinks, backtick-safe code transformers, and padding edge cases

_Packages:_ 🟢 _mdt_core_

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #201](https://github.com/ifiokjr/mdt/pull/201)

- A directory directly under the project root with its own `mdt.toml` is now treated as a separate project. Only directories two or more levels down were isolated, so `tools/mdt.toml` leaked its providers into the parent and caused duplicate-provider errors.
- Symlinked directories no longer abort the scan with a false "symlink cycle" error. Each directory is visited once by canonical path, so aliases (common in iOS `Pods/` and vendored trees) are scanned once, real cycles terminate, and the `[include]` walk no longer loops forever on a cycle. `MdtError::SymlinkCycle` is deprecated.
- `codeBlock` picks a fence longer than any backtick run in the content, and `code` picks a delimiter that does not occur in it, so a provider containing its own fence or inline code can no longer break the surrounding document.
- `replace` with an empty search string leaves content unchanged instead of inserting the replacement between every character.
- `[exclude] blocks` no longer makes `[[formatters]]` fail with "formatter pipeline changed the number of consumer blocks".
- A closing tag indented with whitespace only (in a list item or docstring) keeps its indentation.
- With `before` padding of 1 or more, the first blank-line comment prefix no longer lands on the opening tag's line when the content starts with a newline.
- `[include]` now honours `.gitignore` like the default scan does, and a file that cannot be read as UTF-8 text fails with `mdt::read_file` naming the file.
- `.mjs`, `.cjs`, `.mts`, `.cts`, `.cc`, `.cxx`, `.hh`, and `.hpp` files are scanned by default.
- `compute_updates` no longer aborts on the first provider that fails to render. `UpdateResult::render_errors` lists every consumer that was left untouched, with its location, and all other consumers are still updated. Render messages drop the internal template name and the duplicated "template rendering failed" prefix, and report the template line.
- The new `expected_consumer_content` function (and `ExpectedContent` enum) computes exactly what `mdt update` writes for one consumer. The CLI, MCP server, and language server share it.
- Repeated KDL nodes with the same name (`dep "a"`, `dep "b"`) collect into an array instead of the last one winning.
- The scan cache can no longer write to the wrong file. The project root is always made absolute and normalized (`resolve_root`), and the cache key includes it, so running `mdt update --path ..` from a subdirectory, or scanning a project that moved, rescans instead of reusing file paths recorded from another location. Previously this could splice provider content into an unrelated file.
- Edits that restore the modification time (`cp -p`, `rsync -t`, `touch -r`, archive extraction) are detected: file fingerprints include the inode change time on Unix.
- A script data source whose `watch` entries are not all existing files (a typo, a glob, or a directory) re-runs every time instead of serving cached output forever.
- Git ignore rules match git: nested `.gitignore` files, the ancestors' `.gitignore` files up to the repository root (so `mdt check --path packages/lib` skips the root's `dist/`), and `.git/info/exclude` all apply. Outside a git repository only the root `.gitignore` applies, as before.
- Symlinked files are scanned once (a symlinked `*.t.md` no longer causes a duplicate-provider error), and dangling symlinks are skipped instead of aborting the scan.
- `[templates] paths` adds only `*.t.md` files from each directory, uses the project's `[exclude]` and git ignore rules, and fails with `mdt::templates_path` when an entry is not a directory. A sub-project can point it at a shared template directory outside its root; files there are never treated as its consumers.
- `[exclude] blocks` also silences structural diagnostics (unclosed, unmatched, or nested tags) for the excluded names, and invalid `[include] patterns` globs are rejected when the config loads instead of being dropped.
- Files without any `<!--` are no longer parsed, which removes the dominant cost of scanning large tag-free markdown files.
- `TransformerType::NAMES` lists every transformer's canonical name, so help text can no longer drift from the implementation.
- `content_matches` is public, so tools compare consumer content exactly as `mdt check` does under `[check] comparison`.
- Consumers inside a shared `*.t.md` read through `[templates] paths` from outside the project are no longer treated as the project's consumers, so `mdt update --path <sub-project>` can never rewrite files outside that project.
- In markdown files, a closing tag only keeps indentation from its line: `#` (a heading) and `*` (a bullet) are no longer mistaken for comment prefixes and copied onto the closing tag's new line.
- In source files, closing-tag text inside a string literal or a backtick code span on its line (`"<!-- {/x} -->"`) is not reported as an unmatched closing tag.
- Files without `<!--` are skipped before decoding, so tag-free files in other encodings (such as Latin-1 C sources) no longer fail the scan.
- Nested `.gitignore` files apply only inside a git repository, as in git; outside one, only the project root's `.gitignore` applies.
- `init_project` adds `.mdt/` to `.gitignore` in any directory inside a git repository and reports `InitReport::enclosing_project` when it creates a config inside another mdt project. New `formatter_applies` reports whether a `[[formatters]]` entry formats a file.

- **`mdt init` leaves a green project and clean config.** The sample `readme.md` created by `mdt init` now contains the sample provider's content instead of a placeholder, so a freshly initialized project passes `mdt check` immediately. The generated annotated `mdt.toml` is also rebuilt: the mangled glob examples (`src/**/_.ts`, `packages/_/readme.md`) are restored to `src/**/*.ts` and `packages/*/readme.md`, and the doubled blank lines introduced by markdown formatting of the template source are gone. _Packages:_ 🟢 _mdt_cli_ _Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #198](https://github.com/ifiokjr/mdt/pull/198)
- **Stop watch mode from re-triggering itself.** Every scan rewrites the index cache artifact under `.mdt/cache/`, and the recursive watcher treated those writes as project changes — so each check/update scheduled another one after the debounce window, turning watch mode into a busy loop of CPU and disk churn. Watchers now ignore events inside `<root>/.mdt/`. `mdt list` also counts consumers in one pass instead of scanning the consumer list per provider. _Packages:_ 🟢 _mdt_cli_ _Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #194](https://github.com/ifiokjr/mdt/pull/194)

#### Make editor diagnostics and quick fixes agree with `mdt check` and `mdt update`

_Packages:_ 🟢 _mdt_lsp_

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #201](https://github.com/ifiokjr/mdt/pull/201)

- Stale-block diagnostics, the "Update block" quick fix, and the hover preview now compute expected content through the same engine as `mdt update`, including `[padding]` and the closing tag's comment prefix. Files that `mdt update` just wrote are no longer reported as out of date, and the quick fix no longer glues content onto the tags (for example ``<!-- {=x} -->```sh`` or `* <!-- {=x} --> * text`).
- `[check] comparison = "lenient"` and `[exclude] markdown_codeblocks` are honoured, so blocks that pass `mdt check` or that the scanner ignores are not diagnosed.
- A provider that fails to render is reported as an error at the consumer instead of falling back to the raw template, and no quick fix offers to write the raw `{{ … }}` text.
- Consumers without a provider are errors (they fail `mdt check`), still with did-you-mean suggestions. Providers outside `*.t.md` files are warnings.
- New diagnostics for closing tags without an opening tag (warning), comments that look like tags but do not parse (error), and blocks nested inside a consumer or inline block (error).
- `MDT_LOG=info mdt lsp` no longer panics at startup: the server keeps a tracing subscriber the CLI already installed.
- The quick fix computes its edit range from byte offsets in the open document, so lines with non-ASCII text before a tag are no longer corrupted, and it writes CRLF into CRLF documents.
- Files formatted by a `[[formatters]]` entry get no stale diagnostics or quick fixes, because `mdt check` compares formatter output the server does not compute on every keystroke; run `mdt check` or `mdt update` for them.
- Markdown headings and bullets before a closing tag are no longer treated as comment prefixes.

#### Fix completion panics on multi-byte lines and stale providers

_Packages:_ 🟢 _mdt_lsp_

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #194](https://github.com/ifiokjr/mdt/pull/194)

- Completion context sliced the cursor line by byte index using a UTF-16 column, so a cursor after any multi-byte character panicked the whole language server. Columns are now converted to byte offsets (sharing the same conversion as the rest of the server), and incremental-change ranges that fail to map are logged instead of silently dropped.
- Rename/prepare-rename ranges measure tag prefixes and names in UTF-16 code units instead of bytes, so edits land correctly on non-ASCII documents.
- Saving a template file now removes that file's previous providers before re-adding the current ones — deleted or renamed providers no longer linger in completions, go-to-definition, and rename edits until the next full rescan.
- Project scans (on initialize and config saves) run on the blocking thread pool and apply their results under a short lock instead of blocking the async runtime while holding it.

<details>
<summary><strong>📖 Documentation</strong></summary>

#### Rewrite the documentation to match actual behavior

_Packages:_ ⚪ _mdt_core_, ⚪ _mdt_cli_, ⚪ _mdt_lsp_, ⚪ _mdt_mcp_, ⚪ _@m-d-t/skills_

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #202](https://github.com/ifiokjr/mdt/pull/202)

An audit ran every documented command, flag, default, and example against the CLI. The docs site, crate readmes, the annotated `mdt.toml` that `mdt init` writes, and generated doc comments now match the implementation:

- `[include]` and `[templates] paths` are documented as additive (they were described as narrowing the scan), `[check] comparison = "lenient"` no longer claims to normalize tables or JSON, exclude negation examples work, and formatter patterns are documented as plain globs.
- Exit codes (`0`/`1`/`2`), the global `--ignore-*` flags, project-root discovery, `mdt skill`, per-client `mdt assist` output, the complete JSON payload, GitHub annotations, and every quoted command output are current.
- Source-file examples re-apply comment prefixes with `linePrefix:"...":true`, so copied examples compile.
- Terminology is provider/consumer throughout.
- Crate readme badges render again (link definitions were joined onto one line).
- Links that left the book now point at GitHub, and the Pi link points at pi.dev.

</details>

<details>
<summary><strong>⚡ Performance</strong></summary>

#### Cut repeated work in scan, parse, and render hot paths

_Packages:_ ⚪ _mdt_core_

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #194](https://github.com/ifiokjr/mdt/pull/194)

- The four grammar patterns used to classify token groups are now built once per thread instead of per HTML comment, eliminating hundreds of closure and string allocations for every comment scanned.
- Source-file comment extraction now locifies `<!--`/`-->` via `memchr` (SIMD) instead of a byte-window scan per offset — previously the dominant cost of scanning large non-markdown files.
- Provider templates without parameters render once per run instead of once per consumer, and the base data map is no longer deep-cloned per consumer.
- GFM parse options are constructed once per thread rather than per markdown file.
- Lenient comparisons skip whitespace normalization when bytes already match.
- The index cache artifact serializes as compact JSON instead of pretty JSON.
- File collection deduplication is set-based instead of a linear `contains` scan; comment extraction no longer over-allocates by file size.

</details>

<details>
<summary><strong>🔒 Security</strong></summary>

#### Stop `[[formatters]]` from passing file names through the shell

_Packages:_ ⚪ _mdt_core_

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #201](https://github.com/ifiokjr/mdt/pull/201)

`{{ filePath }}`, `{{ relativeFilePath }}`, and `{{ rootDirectory }}` in a formatter `command` used to be pasted into the `sh -c` (or `cmd /C`) command line as raw text. A file named `docs/a$(touch pwned).md` ran `touch pwned` during `mdt check` or `mdt update`, so a hostile file name in a pull request could execute code in CI.

The placeholders now render as references to environment variables that mdt sets for each formatter run (`MDT_FILE_PATH`, `MDT_RELATIVE_FILE_PATH`, `MDT_ROOT_DIRECTORY`), so the shell expands the path as data and never parses it. The documented form keeps working unchanged:

```toml
[[formatters]]
command = "dprint fmt --stdin \"{{ filePath }}\""
patterns = ["**/*.md"]
```

Keep placeholders in double quotes. A placeholder inside single quotes (`'{{ filePath }}'`) now reaches the formatter as the literal text `${MDT_FILE_PATH}`; switch those to double quotes.

- **Stop `mdt_init` from writing outside the server root through a symlink.** A tool `path` that did not exist yet could not be canonicalized, so confinement fell back to the lexical path: with `esc` symlinked to a directory outside the server root, `mdt_init` with `path: "esc/sub"` passed the containment check and wrote there. Paths are now resolved through their deepest existing ancestor before the check, and a dangling symlink along the path is rejected (`mdt::path_unresolvable`) because creating directories through it would follow it wherever it points. _Packages:_ ⚪ _mdt_mcp_ _Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #201](https://github.com/ifiokjr/mdt/pull/201)

</details>

## [0.9.5](https://github.com/ifiokjr/mdt/releases/tag/v0.9.5) (2026-09-20)

Grouped release for `mdt`.

### 🐛 Fixed

- 🟢 **mdt_cli**: **Show the mdt command name in version and help output.** `mdt --version` printed the crate name (`mdt_cli 0.9.4`) instead of the command users invoke. The clap command is now explicitly named `mdt`, so `--version`, `--help`, and usage errors all display `mdt`. The published crate and install command (`cargo install mdt_cli`) are unchanged. _Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #186](https://github.com/ifiokjr/mdt/pull/186)

## [0.9.4](https://github.com/ifiokjr/mdt/releases/tag/v0.9.4) (2026-09-16)

Grouped release for `mdt`.

### 🐛 Fixed

- **Keep tag spans byte-accurate when string arguments decode escapes.** Token groups derived their end offset from each token's `Display` output, which re-renders a decoded string shorter than its raw source text. Once escape decoding landed, any tag carrying an escape such as `indent:"\t"` computed an opening span one byte short, so `mdt update` spliced the tag's closing `>` out of the file and `mdt check` could no longer match the consumer. Token group ends are now derived from the raw source slice the lexer consumes, keeping offsets byte-accurate regardless of decoded value length. _Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #184](https://github.com/ifiokjr/mdt/pull/184) · _Related issues:_ [#181](https://github.com/ifiokjr/mdt/issues/181)

## [0.9.3](https://github.com/ifiokjr/mdt/releases/tag/v0.9.3) (2026-09-16)

Grouped release for `mdt`.

### 🐛 Fixed

- **Decode escape sequences in transformer string arguments.** Transformer string arguments kept their backslashes literal, so `indent:"\t"` injected a backslash and a `t` instead of a tab, and `suffix:"\n"` never appended a newline. The lexer stripped the surrounding quotes before handing the value to `snailquote`, which only expands escapes while it is inside a quoted region, so no escape was ever decoded. Arguments are now decoded from the full quoted slice, which makes `\t`, `\n`, `\\` and `\"` work inside double-quoted arguments; single-quoted arguments and unrecognised escapes keep their backslashes so existing tags are unaffected. _Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #181](https://github.com/ifiokjr/mdt/pull/181)
- **Use `rmcp::model::ServerConfig` instead of the deprecated `ServerInfo`.** `rmcp` renamed the `ServerInfo` type alias to `ServerConfig` and deprecated the old name. The `ci/test` jobs on macOS and Windows compile with `-D warnings`, so the deprecation became a hard build failure as soon as the release lockfile picked up `rmcp` 3.4.0, blocking the release. The workspace now requires `rmcp` 3.4.0 and `mdt_mcp` uses the new type name. _Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #183](https://github.com/ifiokjr/mdt/pull/183)

## [0.9.2](https://github.com/ifiokjr/mdt/releases/tag/v0.9.2) (2026-09-07)

Grouped release for `mdt`.

### 📝 Changed

#### Update rmcp to 3.2.0 and rstest to 0.27.0

_Packages:_ _mdt_

Bumps the MCP SDK (rmcp 2.x -> 3.2.0) and the test harness (rstest 0.26 -> 0.27), plus a devenv flake input refresh. No public API changes.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #171](https://github.com/ifiokjr/mdt/pull/171)

### 🐛 Fixed

#### Preserve CRLF line endings when updating consumer files

_Packages:_ _mdt_

mdt update parsed consumer files after LF-normalization but spliced into the raw CRLF text, truncating consumer tags and leaving residual old content. All read paths now normalize before splicing and written files keep their original line-ending style. Fixes #169.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #170](https://github.com/ifiokjr/mdt/pull/170) · _Closed issues:_ [#169](https://github.com/ifiokjr/mdt/issues/169)

#### Render paths with forward slashes on every platform

_Packages:_ _mdt_core_, _mdt_cli_, _mdt_lsp_

CLI diagnostics, JSON reports, and doctor messages now normalize path separators so output is identical across operating systems, keeping the Unix-recorded snapshot corpus valid on Windows. The LSP server also resolves file URIs for stored and synthetic paths consistently across platforms when computing goto definition, references, and duplicate provider diagnostics.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #172](https://github.com/ifiokjr/mdt/pull/172)

## [0.9.1](https://github.com/ifiokjr/mdt/releases/tag/v0.9.1) (2026-08-21)

Grouped release for `mdt`.

### 🐛 Fixed

#### Fix comment prefix loss when migrating inline blocks to padded layout.

_Packages:_ _mdt_

When a project enables `[padding]` after previously rendering blocks without it, the closing tag was pushed to its own line without the comment prefix (e.g., `///`, `//!`), producing invalid source code. The closing-tag prefix is now recovered from the closing tag's line in the source file instead of from the block content, so the migration produces valid comments. Markdown files are unaffected.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #165](https://github.com/ifiokjr/mdt/pull/165)

<details>
<summary><strong>📖 Documentation</strong></summary>

#### Documentation rewrite

_Packages:_ _mdt_core_, _mdt_cli_, _mdt_lsp_, _mdt_mcp_, _@m-d-t/skills_

Rewrite documentation across the repo: tighten prose, drop AI-flavored phrasing, fix stale references (knope to monochange, version numbers, missing `mdt list` command, license badge typo), align the annotated config docs with the actual strict-default plus formatters setup, and add the missing `if` transformer docs. Template-driven content was updated in `.templates/` and synced via `mdt update`.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #167](https://github.com/ifiokjr/mdt/pull/167)

</details>

## [0.9.0](https://github.com/ifiokjr/mdt/releases/tag/v0.9.0) (2026-07-04)

Grouped release for `mdt`.

### 💥 Breaking Change

#### Add formatter-aware full-file normalization

_Packages:_ _mdt_core_

`mdt_core` now supports opt-in `[[formatters]]` configuration in `mdt.toml`. Matching formatter commands run against the entire updated target file in declaration order using stdin/stdout, enabling formatter-aware drift detection and update output.

This is a major release because public constructible structs such as `MdtConfig` and `ProjectContext` gained fields. Downstream crates that build these structs with literals must add the new fields or use defaults/builders where available.

```rust
let config = MdtConfig {
    formatters: Vec::new(),
    ..existing_config
};
```

Formatter failures now surface as dedicated diagnostics so callers can distinguish rendering problems from formatter command failures.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Tighten core error handling and API ergonomics

_Packages:_ _mdt_core_

`mdt_core` now applies a set of Rust API and implementation best practices across error handling, allocation behavior, and public function documentation. The changes remove broad `AnyError` style aliases in favor of `MdtResult`, add `# Errors` documentation to result-returning public functions, mark useful return values with `#[must_use]`, and pre-allocate vectors in hot paths.

This is a major release because public aliases were removed and `render_template` now returns `Cow<'_, str>` to avoid allocations when no template syntax is present. Downstream callers may need to adjust type annotations or convert borrowed results when an owned `String` is required.

```rust
let rendered = render_template(template, &data)?;
let owned: String = rendered.into_owned();
```

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Add official assistant setup profiles

_Packages:_ _mdt_cli_

The CLI now includes an `mdt assist` command that prints official assistant setup profiles. It focuses on practical adoption by producing ready-to-copy MCP configuration snippets and suggested repo-local guidance for Claude, Cursor, Copilot, Pi, and generic MCP clients.

This is a major release because the public CLI command model gains a new `Commands::Assist` variant. Downstream crates that exhaustively match command variants will need to handle the new case.

```rust
match command {
    Commands::Assist(args) => run_assist(args),
    other => run_existing_command(other),
}
```

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #117](https://github.com/ifiokjr/mdt/pull/117) · _Closed issues:_ [#109](https://github.com/ifiokjr/mdt/issues/109) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Publish the CLI through npm packages

_Packages:_ _@m-d-t/cli-darwin-arm64_, _@m-d-t/cli-darwin-x64_, _@m-d-t/cli-linux-arm64-gnu_, _@m-d-t/cli-linux-arm64-musl_, _@m-d-t/cli-linux-x64-gnu_, _@m-d-t/cli-linux-x64-musl_, _@m-d-t/cli-win32-x64-msvc_, _@m-d-t/cli-win32-arm64-msvc_

`mdt` now has an official npm distribution channel. Releases prepare a top-level `@m-d-t/cli` package plus platform-specific binary packages for Linux, macOS, and Windows.

Users can install the CLI globally with npm or run it on demand through npx, making adoption easier in JavaScript-heavy projects and environments that do not already have Rust tooling installed.

```bash
npx @m-d-t/cli init
```

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #121](https://github.com/ifiokjr/mdt/pull/121) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

### 🚀 Feature

#### Instrument core template processing with tracing

_Packages:_ _mdt_core_

`mdt_core` now emits structured tracing spans and events around important template-processing boundaries. Public API entry points are annotated with `#[instrument]`, and the engine records `debug!`, `trace!`, and `warn!` events while loading projects, resolving providers, rendering consumers, and reporting notable processing states.

This makes failures and performance issues easier to diagnose from CLI, LSP, and MCP callers without changing the core API or the rendered markdown output.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Add lenient block comparison to configuration

_Packages:_ _mdt_core_

`mdt_core` now supports `[check] comparison = "lenient"` for whitespace-tolerant block comparison. In lenient mode, the engine normalizes blank-line counts and trailing whitespace before comparing expected and actual consumer content.

This reduces false-positive stale-block reports after external formatter rewrites. Update operations remain exact and continue to write the rendered provider output byte-for-byte.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Expose structured CLI logs with MDT_LOG

_Packages:_ _mdt_cli_

The CLI now initializes `tracing-subscriber` with an `EnvFilter` sourced from `MDT_LOG`. This gives operators and contributors a consistent way to inspect command execution without adding ad-hoc debug output or changing normal terminal output.

The subscriber is installed at process startup and defaults to quiet behavior unless the environment variable is set. Users can opt into targeted diagnostics for parser, project-loading, update, or check flows while preserving the existing user-facing command experience.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Support lenient whitespace comparison in check

_Packages:_ _mdt_cli_

`mdt check` now honors `[check] comparison = "lenient"` for whitespace-tolerant verification. This mode allows projects to keep external formatters enabled without reporting stale blocks for harmless whitespace rewrites.

The command still reports meaningful content drift, while `mdt update` continues to write exact rendered bytes regardless of the comparison setting.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Run configured formatters during check and update

_Packages:_ _mdt_cli_

`mdt update` and `mdt check` now support opt-in `[[formatters]]` configuration. When a formatter matches a target file, `mdt` runs the formatter against the full updated file so template output converges with project tools such as dprint or Prettier.

This lets teams keep normal formatting workflows enabled while still detecting formatter-aware template drift during checks.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Publish the CLI through npm packages

_Packages:_ _mdt_cli_

`mdt` now has an official npm distribution channel. Releases prepare a top-level `@m-d-t/cli` package plus platform-specific binary packages for Linux, macOS, and Windows.

Users can install the CLI globally with npm or run it on demand through npx, making adoption easier in JavaScript-heavy projects and environments that do not already have Rust tooling installed.

```bash
npx @m-d-t/cli init
```

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #121](https://github.com/ifiokjr/mdt/pull/121) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Publish official agent skills for mdt

_Packages:_ _mdt_cli_

`mdt` now publishes an official `@m-d-t/skills` npm package for Pi and other harnesses that support the Agent Skills standard. The package includes quick-start instructions, MCP tool guidance, and a detailed reference for template syntax, transformers, interpolation, inline blocks, configuration, CLI commands, MCP tools, and source-file patterns.

The release tooling now generates and publishes the skills package alongside the CLI, and `mdt assist pi` points users toward the packaged skill.

```sh
pi install npm:@m-d-t/skills
pi -e npm:@m-d-t/skills
```

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #128](https://github.com/ifiokjr/mdt/pull/128) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Expose LSP diagnostics through MDT_LOG tracing

_Packages:_ _mdt_lsp_

The language server now initializes `tracing-subscriber` with an `EnvFilter` sourced from `MDT_LOG`. Logs are written to stderr so tracing never interferes with the JSON-RPC protocol carried over stdio.

This gives editor integrations a safe opt-in diagnostics path for initialization, document updates, and template checks while preserving the default quiet behavior expected by LSP clients.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Expose MCP diagnostics through MDT_LOG tracing

_Packages:_ _mdt_mcp_

The MCP server now initializes `tracing-subscriber` with an `EnvFilter` sourced from `MDT_LOG`. Logs are emitted to stderr so tracing does not corrupt MCP stdio messages or structured tool responses.

This makes agent-driven workflows easier to debug when a check, update, preview, or init call behaves unexpectedly, while leaving normal tool output unchanged unless logging is explicitly enabled.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Return structured JSON envelopes from MCP tools

_Packages:_ _mdt_mcp_

MCP tools now return more consistent JSON-oriented `structured_content` envelopes for `mdt_check`, `mdt_update`, `mdt_preview`, and `mdt_init`. Text content is still preserved for clients that display plain tool messages.

`mdt_preview` now behaves more like an authoring workflow by returning per-consumer rendered output, parameterized previews, render details, and mismatch information. Check and update responses also surface undefined-variable warnings so agents can reason about template problems without scraping text.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #116](https://github.com/ifiokjr/mdt/pull/116) · _Closed issues:_ [#108](https://github.com/ifiokjr/mdt/issues/108) · _Related issues:_ [#112](https://github.com/ifiokjr/mdt/issues/112), [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Support lenient comparison in MCP checks

_Packages:_ _mdt_mcp_

The MCP `mdt_check` tool now honors `[check] comparison = "lenient"`, matching the CLI behavior for whitespace-tolerant verification. Agent workflows can therefore distinguish real template drift from harmless formatter whitespace changes.

The MCP response still reports mismatches when normalized content differs, and update behavior remains exact.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Run configured formatters from MCP tools

_Packages:_ _mdt_mcp_

MCP `mdt_update` and `mdt_check` now honor opt-in `[[formatters]]` configuration. Agent workflows can preview, update, and verify formatter-aware template output using the same configuration as the CLI.

This keeps MCP-driven synchronization aligned with project formatters while preserving structured diagnostics for formatter failures.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

### 🐛 Fixed

#### Extract shared app-surface helpers into core

_Packages:_ _mdt_core_

`mdt_core` now exposes shared helpers for project-root resolution, relative path display, and similar-name scoring. These utilities centralize behavior that was previously reimplemented by multiple application surfaces.

The extraction keeps user-facing behavior stable while making CLI, LSP, and MCP code easier to maintain consistently.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Add package repository metadata

_Packages:_ _mdt_core_, _mdt_cli_, _mdt_lsp_, _@m-d-t/cli_, _@m-d-t/cli-darwin-arm64_, _@m-d-t/cli-darwin-x64_, _@m-d-t/cli-linux-arm64-gnu_, _@m-d-t/cli-linux-arm64-musl_, _@m-d-t/cli-linux-x64-gnu_, _@m-d-t/cli-linux-x64-musl_, _@m-d-t/cli-win32-x64-msvc_, _@m-d-t/cli-win32-arm64-msvc_, _@m-d-t/skills_

Cargo and npm package manifests now include package-specific repository URLs. This keeps package metadata aligned with monochange manifest linting and points registry users directly to each package's source directory.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #153](https://github.com/ifiokjr/mdt/pull/153)

#### Update logos and remove the empty skip regex

_Packages:_ _mdt_core_

`logos` has been updated to 0.16.1. The tokenizer no longer uses the `#[logos(skip r"")]` attribute because the newer release rejects empty regular expressions that can match the empty string.

This keeps the lexer compatible with the current `logos` API without changing tokenization behavior for valid input.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Respect terminal color settings in check output

_Packages:_ _mdt_cli_

`mdt check` now applies ANSI color handling consistently across diagnostics and stale-block summaries. Color is enabled when the terminal supports it or `CLICOLOR_FORCE` is set, and it remains disabled when users pass `--no-color`, set `NO_COLOR`, or set `CLICOLOR=0`.

The result is clearer interactive output without surprising color in scripts or environments that explicitly request plain text.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #120](https://github.com/ifiokjr/mdt/pull/120) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Reuse shared core helpers in CLI surfaces

_Packages:_ _mdt_cli_

The CLI now uses shared `mdt_core` helpers for project-root resolution, relative path display, and similar-name scoring. This reduces duplicated logic between app surfaces while preserving the same command behavior and diagnostics users already expect.

Keeping these concerns in one place makes future fixes less likely to drift between CLI, LSP, and MCP integrations.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Rename npm packages under the m-d-t scope

_Packages:_ _mdt_cli_

The npm distribution has moved from the `@ifi` scope to the `@m-d-t` organization. The top-level CLI package is now `@m-d-t/cli`, the skills package is `@m-d-t/skills`, and all platform-specific binary packages now use the `@m-d-t/cli-*` naming pattern.

This aligns npm package names with the project name and makes the distribution easier to recognize in package registries and install commands.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #130](https://github.com/ifiokjr/mdt/pull/130) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Reuse shared core helpers in the LSP server

_Packages:_ _mdt_lsp_

The language server now uses shared `mdt_core` helpers for project-root resolution and relative path display. This keeps editor diagnostics aligned with the CLI and MCP surfaces without changing the LSP protocol behavior.

Centralizing this logic reduces the chance that path formatting or project discovery diverges across integrations.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Respect existing template locations during MCP init

_Packages:_ _mdt_mcp_

The MCP initialization flow now follows the same bootstrap rules as the CLI. It detects existing canonical and legacy template locations before writing starter content, avoiding unnecessary root-level `template.t.md` files in projects that already have a usable template directory.

Generated READMEs and initialization guidance now consistently describe `.templates/template.t.md` as the preferred starter path.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Reuse shared core helpers in MCP tools

_Packages:_ _mdt_mcp_

MCP tools now use shared `mdt_core` helpers for project-root resolution and relative path display. Agent-facing responses therefore stay aligned with CLI and LSP behavior while preserving the existing MCP tool contract.

The cleanup reduces duplicated app-surface code and makes future fixes easier to apply consistently.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Update rmcp server result construction

_Packages:_ _mdt_mcp_

`rmcp` has been updated to 2.1.0. The MCP server now uses `ServerInfo::new().with_instructions()` for server metadata, the `CallToolResult::success()` and `CallToolResult::error()` constructors, and `ContentBlock` for text tool responses.

This keeps the MCP integration aligned with the current `rmcp` API while preserving the same tool behavior for clients.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #141](https://github.com/ifiokjr/mdt/pull/141)

<details>
<summary><strong>📖 Documentation</strong></summary>

#### Improve first-time installation and quick-start guidance

_Packages:_ _mdt_cli_

The installation and quick-start documentation now better serves users who want `mdt` without building Rust source locally. It recommends prebuilt release binaries for non-Rust projects, removes stale version-pinned snippets, and presents a concise first-run workflow.

The new quick start shows how to keep a README section and a source-doc comment synchronized from a single provider, giving new users a practical end-to-end success path.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #113](https://github.com/ifiokjr/mdt/pull/113) · _Closed issues:_ [#106](https://github.com/ifiokjr/mdt/issues/106) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Clarify the documentation drift problem

_Packages:_ _mdt_cli_

The README and guide introduction now lead with the core problem `mdt` solves: keeping README sections, source-doc comments, and docs-site content synchronized as projects evolve. The positioning is clearer for library and tool maintainers evaluating whether markdown templates fit their workflow.

The updated copy also explains how editor and agent integrations support a human-first documentation process instead of replacing normal authoring practices.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #114](https://github.com/ifiokjr/mdt/pull/114) · _Closed issues:_ [#107](https://github.com/ifiokjr/mdt/issues/107) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

#### Add adoption walkthroughs for real documentation flows

_Packages:_ _mdt_cli_

The documentation now includes proof-of-value and migration walkthroughs that show how to adopt `mdt` in realistic projects. The examples cover synchronizing README content, source documentation, and docs-site pages without forcing teams to rewrite their documentation process.

These guides make it easier to evaluate the tool, migrate incrementally, and understand where provider and consumer blocks fit in existing markdown and source files.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #118](https://github.com/ifiokjr/mdt/pull/118) · _Closed issues:_ [#110](https://github.com/ifiokjr/mdt/issues/110) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

</details>

<details>
<summary><strong>🔨 Refactor</strong></summary>

#### Remove the legacy npm source folder

_Packages:_ _@m-d-t/cli_, _@m-d-t/skills_

The old `npm/` tree has been removed now that npm packages live under `packages/`. Tests and repository metadata now point at the generated package launcher and package directories under `packages/`.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #147](https://github.com/ifiokjr/mdt/pull/147) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

</details>

## 0.7.0 (2026-03-02)

### Breaking Changes

- Add `if` conditional transformer for selectively including block content based on data values. The `if` transformer takes a dot-separated data path as an argument and includes the block content only when the referenced value is truthy (exists and is not false, null, empty string, or zero). Example usage: `<!-- {=block|if:"config.features.enabled"} -->`.

#### Harden public API and improve crate documentation.

**Breaking (`mdt_core`):** Add `#[non_exhaustive]` to all 9 public enums (`MdtError`, `ParseDiagnostic`, `Argument`, `TransformerType`, `BlockType`, `PaddingValue`, `CodeBlockFilter`, `Token`, `DiagnosticKind`). This prevents downstream exhaustive pattern matching and allows new variants to be added in future minor releases without breaking changes. Downstream code matching on these enums must add a wildcard (`_`) arm.

**Breaking (`mdt_core`):** Make `source_scanner` module public (`pub mod source_scanner`). Previously it was private with items re-exported at the crate root via `pub use source_scanner::*`. The module is now directly accessible, fixing rustdoc link warnings for `[`source_scanner`]` references in crate-level documentation.

**`mdt_lsp`:** Add crate-level documentation using the `mdtLspOverview` template block, providing an overview of LSP capabilities and usage instructions directly in `lib.rs`.

**`mdt_mcp`:** Add crate-level documentation using the `mdtMcpOverview` template block, providing an overview of MCP tools and configuration directly in `lib.rs`.

**`mdt_cli`:** Add wildcard arms to `DiagnosticKind` match statements for forward compatibility with new diagnostic kinds.

**All crates:** Replace `tokio` `features = ["full"]` with minimal required feature sets — `mdt_cli` uses `["rt-multi-thread"]`, `mdt_lsp` uses `["rt-multi-thread", "macros", "sync", "io-std"]`, `mdt_mcp` uses `["rt-multi-thread", "macros"]`. This makes dependency requirements explicit.

**Config:** Remove stale data entries from `mdt.toml` (`cargo_mdt_core`, `cargo_mdt_cli`, `cargo_mdt_lsp`, `cargo_mdt_mcp`) that referenced individual crate `Cargo.toml` files no longer needed since crates use `version = { workspace = true }`.

#### Add a new public `Commands::Info` variant to `mdt_cli` and improve human-readable CLI output formatting (`mdt check` and new `mdt info`).

This is marked major because `Commands` is a public enum and adding a variant is a breaking change for exhaustive matches in downstream crates.

#### Expand config and data-source capabilities in `mdt_core`:

- Add config discovery precedence across `mdt.toml`, `.mdt.toml`, and `.config/mdt.toml`.
- Add typed `[data]` entries (`{ path, format }`) while keeping string-path compatibility.
- Add `ini` data format support.
- Expose new config/data APIs (`CONFIG_FILE_CANDIDATES`, `MdtConfig::resolve_path`, `DataSource`, `TypedDataSource`).

This is marked major because `MdtConfig.data` changes type from `HashMap<String, PathBuf>` to `HashMap<String, DataSource>`.

### Features

- Add `--watch` flag to `mdt check` command. When enabled, the check command monitors the project directory for file changes and automatically re-runs the check whenever files are modified or created. Uses 200ms debouncing to avoid redundant checks during rapid file changes. Unlike single-run mode, watch mode does not exit with a non-zero status code on stale consumers -- it prints the results and continues watching.
- Add `references` and `rename` support to the LSP server. `textDocument/references` returns all provider and consumer blocks sharing the same name. `textDocument/rename` renames a block name across all provider and consumer tags (both opening and closing tags) in the workspace.

#### Add cache observability across core and CLI diagnostics.

- Persist cache telemetry counters in the project index cache (scan count, full-hit count, cumulative reused/reparsed file counts, and last scan details).
- Expose cache inspection APIs from `mdt_core::project` for diagnostics surfaces.
- Extend `mdt info` with a cache section in text and JSON output (artifact health, schema/key compatibility, hash mode, cumulative metrics, and last scan summary).
- Extend `mdt doctor` with cache checks for artifact validity, hash mode guidance, and efficiency trend heuristics.
- Add unit/e2e/snapshot coverage and docs updates for the new observability output.

### Fixes

- Switch LSP text document synchronization from full to incremental mode. The server now receives only changed text ranges instead of the entire document content on each edit, improving performance for large files. Includes proper UTF-16 offset handling for LSP position conversion.

#### Fix collapsed newlines in `mdtBadgeLinks` provider block in `template.t.md`. The multi-line link reference definitions were accidentally collapsed to a single line by an external markdown formatter (`dprint fmt`) that doesn't recognize `{{ }}` template syntax in URLs as valid link definitions.

Restored the template content to its correct multi-line format. Added unit tests and CLI integration tests to verify that `mdt update` preserves newlines in multi-line content through the full scan → render → update pipeline, including idempotency after write-back.

### Documentation

- Fix broken markdown rendering in all crate READMEs. Badge reference links were collapsed onto a single line, causing them to render as plain text instead of clickable badge images. Each reference link definition now appears on its own line. Also added reusable mdt template blocks for LSP overview, MCP overview, CLI install, and contributing sections. Updated `mdt_core` title from `mdt` to `mdt_core`. Replaced stale `mdt_lsp` README content with mdt template blocks. Updated root README with crate table and streamlined contributing section.

## 0.6.0 (2026-02-26)

### Breaking Changes

#### Unify all workspace crates under a single shared version.

Previously each crate (`mdt_core`, `mdt_cli`, `mdt_lsp`, `mdt_mcp`) maintained its own independent version, which led to version drift — e.g., `mdt_core` at 0.5.0 while `mdt_lsp` and `mdt_mcp` were still at 0.4.1. This made it harder to reason about compatibility and complicated the release process with per-crate changelogs and tag prefixes (`mdt_cli/v0.4.1`).

All crates now inherit `version = { workspace = true }` from the root `Cargo.toml` workspace version. The knope release configuration has been consolidated from four separate `[packages.*]` sections into a single `[package]` with all versioned files and dependencies listed together. Releases now use a single changelog (`changelog.md`) and simplified version tags (`v0.5.0` instead of `mdt_cli/v0.5.0`).

This is a breaking change to the release workflow and tag format, not to the library APIs.

### Fixes

- Add integration tests for positional block arguments in LSP and MCP servers.
- Add `mdt_mcp` to workspace members so it is built, tested, and published through normal CI workflows.
