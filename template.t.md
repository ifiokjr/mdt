<!-- {@mdtPackageDocumentation} -->

`mdt` helps library and tool maintainers keep README sections, source-doc comments, and docs-site content synchronized across a project. Define content once with comment-based template tags, then reuse it across markdown files, code documentation comments, READMEs, mdbook docs, and more so your docs do not drift.

<!-- {/mdtPackageDocumentation} -->

<!-- {@mdtCliUsage} -->

### CLI Commands

- `mdt init [--path <dir>]` — Create a sample `.templates/template.t.md` file and starter `mdt.toml`.
- `mdt check [--path <dir>] [--verbose]` — Verify all target blocks are up-to-date. Exits non-zero if any are stale.
- `mdt update [--path <dir>] [--verbose] [--dry-run]` — Update all target blocks with latest source content.
- `mdt info [--path <dir>]` — Print project diagnostics and cache observability metrics.
- `mdt doctor [--path <dir>] [--format text|json]` — Run health checks with actionable hints, including cache validity and efficiency.
- `mdt assist <assistant> [--format text|json]` — Print an official assistant setup profile with MCP config and repo-local guidance.
- `mdt lsp` — Start the mdt language server (LSP) for editor integration. Communicates over stdin/stdout.
- `mdt mcp` — Start the mdt MCP server for AI assistants. Communicates over stdin/stdout.

### Diagnostics Workflow

- Run `mdt info` first to inspect project shape, diagnostics totals, and cache reuse telemetry.
- Run `mdt doctor` when you need actionable health checks and remediation hints (config/data/layout/cache).
- Use `MDT_CACHE_VERIFY_HASH=1` when troubleshooting cache consistency issues and comparing reuse behavior.

<!-- {/mdtCliUsage} -->

<!-- {@mdtTemplateSyntax} -->

### Template Syntax

**Source tag** (defines a template block in `*.t.md` definition files):

```markdown
<!-- {@blockName} -->

Content to inject

<!-- {/blockName} -->
```

**Target tag** (marks where content should be injected):

```markdown
<!-- {=blockName} -->

This content gets replaced

<!-- {/blockName} -->
```

**Inline tag** (source-free interpolation using configured data):

```markdown
Current version: <!-- {~version:"{{ "{{" }} package.version {{ "}}" }}"} -->0.0.0<!-- {/version} -->
```

```markdown
| Artifact | Version                                                                   |
| -------- | ------------------------------------------------------------------------- |
| mdt_cli  | <!-- {~cliVersion:"{{ "{{" }} package.version {{ "}}" }}"} -->0.0.0<!-- {/cliVersion} --> |
```

**Filters and pipes:** Template values support pipe-delimited transformers:

```markdown
<!-- {=block|prefix:"\n"|indent:"  "} -->
```

Available transformers: `trim`, `trimStart`, `trimEnd`, `indent`, `prefix`, `suffix`, `linePrefix`, `lineSuffix`, `wrap`, `codeBlock`, `code`, `replace`, `if`.

<!-- {/mdtTemplateSyntax} -->

<!-- {@mdtInlineBlocksGuide} -->

Inline blocks are useful when you need dynamic content in-place without creating a separate source. Typical examples include versions, toolchain values, environment metadata, and short computed strings.

Inline blocks render minijinja template content from the block's first argument:

```markdown
<!-- {~version:"{{ "{{" }} pkg.version {{ "}}" }}"} -->0.0.0<!-- {/version} -->
```

During `mdt update`, mdt evaluates the template argument with your configured `[data]` context, then replaces the content between the opening and closing tags.

Because inline blocks are source-free, they are ideal for one-off values that still need to stay synchronized.

<!-- {/mdtInlineBlocksGuide} -->

<!-- {@mdtInlineBlocksLimits} -->

- Inline blocks must include a first argument that is the template string to render.
- Inline blocks do not resolve source content; everything comes from the inline template argument and current data context.
- Inline rendering still supports transformers (`|trim`, `|code`, etc.) after template evaluation.
- In markdown, inline blocks work in normal content (paragraphs, lists, headings, tables) where HTML comments are parsed.
- Tags shown inside fenced markdown code blocks are treated as examples and are not interpreted as live blocks.
- In source files, inline tags follow source scanning rules and respect `[exclude] markdown_codeblocks` filtering.

<!-- {/mdtInlineBlocksLimits} -->

<!-- {@mdtInlineBlocksExamples} -->

### Inline value in prose

```markdown
Install version <!-- {~releaseVersion:"{{ "{{" }} pkg.version {{ "}}" }}"} -->0.0.0<!-- {/releaseVersion} --> today.
```

### Inline value in a table cell

```markdown
| Package | Version                                                               |
| ------- | --------------------------------------------------------------------- |
| mdt     | <!-- {~mdtVersion:"{{ "{{" }} pkg.version {{ "}}" }}"} -->0.0.0<!-- {/mdtVersion} --> |
```

### Inline value with a transformer

```markdown
CLI version: <!-- {~cliVersionCode:"{{ "{{" }} pkg.version {{ "}}" }}"|code} -->`0.0.0`<!-- {/cliVersionCode} -->
```

### Inline value from a script-backed data source

```toml
[data]
release = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

```markdown
Release: <!-- {~releaseValue:"{{ "{{" }} release {{ "}}" }}"} -->0.0.0<!-- {/releaseValue} -->
```

When `VERSION` is unchanged, mdt reuses cached script output from `.mdt/cache/data-v1.json`.

<!-- {/mdtInlineBlocksExamples} -->

<!-- {@mdtScriptDataSourcesGuide} -->

`[data]` entries can run shell commands and use stdout as template data. This is useful for values that come from tooling (for example Nix, git metadata, or generated version files).

```toml
[data]
release = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

- `command`: shell command executed from the project root.
- `format`: parser for stdout (`text`, `json`, `toml`, `yaml`, `yml`, `kdl`, `ini`).
- `watch`: files that control cache invalidation.

When `watch` files are unchanged, mdt reuses cached script output from `.mdt/cache/data-v1.json` instead of re-running the command.

<!-- {/mdtScriptDataSourcesGuide} -->

<!-- {@mdtScriptDataSourcesNotes} -->

- Script outputs are cached per namespace, command, format, and watch list.
- If `watch` is empty, mdt re-runs the script every load (no cache hit).
- A non-zero script exit status fails data loading with an explicit error.

<!-- {/mdtScriptDataSourcesNotes} -->

<!-- {@mdtFormatterPipelineDocs} -->

Formatter entries make `mdt update` and `mdt check` converge with your formatter's canonical **full-file** output instead of comparing raw injected block text.

This is the recommended long-term fix for the `mdt update → formatter → mdt check` cycle described in issue #46, and the best way to keep CI green when external formatters rewrite synced files.

Each matching formatter entry:

- reads the full candidate file from stdin
- writes the full replacement file to stdout
- runs from the project root
- runs after block injection during `mdt update`
- runs before expected-output comparison during `mdt check`
- runs in declaration order when multiple entries match the same file

`command` is rendered with minijinja before execution. Available variables:

- `{{ "{{" }} filePath {{ "}}" }}` — absolute path to the file being formatted
- `{{ "{{" }} relativeFilePath {{ "}}" }}` — path relative to the project root
- `{{ "{{" }} rootDirectory {{ "}}" }}` — absolute project root

`patterns` and `ignore` are ordered gitignore-style rule lists. Leading `!` entries negate a prior match, so later rules can re-include paths for a single formatter stage.

If a formatter command fails, exits non-zero, or renders an invalid minijinja command template, mdt returns an explicit formatter error instead of silently falling back to unformatted output.

```toml
[[formatters]]
command = "dprint fmt --stdin \"{{ "{{" }} filePath {{ "}}" }}\""
patterns = ["**/*.md", "!docs/generated/**"]
ignore = ["vendor/**", "docs/generated/**", "!docs/generated/keep.md"]
```

Repositories without configured formatters keep the legacy fast path, so formatter support only adds work when you opt in.

<!-- {/mdtFormatterPipelineDocs} -->

<!-- {@mdtFormatterOnlyStaleDocs} -->

Formatter-aware checking can also report **formatter-only** drift. This happens when the formatter would rewrite the full file, but no individual managed block body is stale.

In that case mdt reports the file in `stale_files` so automation can distinguish surrounding-formatting drift from block-content drift. The CLI JSON output and MCP responses include `stale_files` for this reason.

<!-- {/mdtFormatterOnlyStaleDocs} -->

<!-- {@mdtCheckJsonOutput} -->

`mdt check --format json` returns:

- `ok` — overall success boolean
- `stale` — block-level drift entries with `file` and `block`
- `stale_files` — formatter-only file drift entries with `file`

When formatter-aware normalization would change the full file without changing any managed block body, `stale_files` is populated and `stale` can remain empty.

Clean output:

```json
{"ok":true,"stale":[],"stale_files":[]}
```

Formatter-only drift example:

```json
{
  "ok": false,
  "stale": [],
  "stale_files": [{ "file": "docs/readme.md" }]
}
```

<!-- {/mdtCheckJsonOutput} -->

<!-- {@mdtAnnotatedConfiguration} -->

{% raw %}
# mdt.toml
#
# This file is intentionally verbose: active entries show one working setup,
# and commented entries document every configuration option currently
# supported by the codebase.
#
# Rule for contributors: when config behavior changes, update this annotated
# file and the synced configuration guide in the same PR.

# Top-level safety limit for scanned files, in bytes.
# Omit this to use the built-in default of 10 MB.
# Raise it for unusually large generated docs; lower it if you want earlier
# failure on oversized files.
# max_file_size = 10485760

# By default mdt respects `.gitignore` so it behaves like the repo itself.
# Set this to `true` only when ignored/generated files should still be scanned,
# or when you want `[include]` and `[exclude]` to be the only scanning rules.
# disable_gitignore = true

# Padding controls the blank lines between tags and injected content.
# Supported values for both `before` and `after`:
# - false -> keep content inline with the tag
# - 0     -> move content to the next line with no blank line
# - 1     -> one blank line
# - 2+    -> two or more blank lines
#
# This repo uses `0`/`0` because it keeps comment-based targets formatter-stable
# without introducing extra blank lines for dprint/rustfmt to rewrite.
[padding]
before = 0
after = 0

[data]
# String values are file-backed namespaces.
# The parser is inferred from the extension: `.json`, `.toml`, `.yaml`,
# `.yml`, `.kdl`, and `.ini` are supported.
#
# This repo exposes Cargo metadata as `{{ cargo.package.* }}` so templates can
# stay synchronized with workspace package information.
cargo = "Cargo.toml"

# Typed data sources let you force a parser when the extension is missing,
# unusual, or intentionally generic.
# release = { path = "release-info", format = "json" }

# Script-backed data sources shell out and parse stdout.
# `format` accepts: `text`, `string`, `raw`, `txt`, `json`, `toml`, `yaml`,
# `yml`, `kdl`, or `ini`.
# `watch` lists files that invalidate the cached result in
# `.mdt/cache/data-v1.json`.
#
# Use this when the source of truth comes from tooling instead of a checked-in
# file.
# version = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
# git = { command = "git rev-parse --short HEAD", format = "text" }

[exclude]
# Gitignore-style patterns skip files or directories during scanning.
# Supports `!negation`, trailing `/` for directories, `*`, `**`, and character
# classes.
#
# This repo excludes test-only fixtures and snapshot directories so mdt only
# scans files that can contain real, maintained blocks.
patterns = [
	"**/tests/",
	"**/__tests.rs",
	"**/snapshots/",
]

# `markdown_codeblocks` only affects fenced code blocks that appear inside
# source-file comments. It exists so docs/examples can show mdt tags without
# accidentally turning those examples into live targets.
#
# Supported values:
# - false        -> process tags in fenced code blocks normally (default)
# - true         -> ignore tags in all fenced code blocks
# - "..."        -> ignore code blocks whose info string contains that substring
# - ["...", ...] -> ignore code blocks whose info string matches any substring
#
# This repo uses `true` because source-comment examples should stay
# illustrative, not executable.
markdown_codeblocks = true

# `blocks` excludes specific block names everywhere, even if their files are
# scanned. Use it when a block name is temporary, experimental, or
# intentionally unmanaged.
# blocks = ["draftSection", "experimentalApi"]

# `include` narrows scanning to only matching files. Use it to opt into a
# smaller search space in large repos once you know exactly where mdt tags live.
# [include]
# patterns = ["docs/**/*.rs", "src/**/*.ts", "packages/*/readme.md"]

# `templates.paths` restricts where `*.t.md` source files are discovered.
# Leave it unset to find template files anywhere in the project.
# Use it when a repo wants a dedicated source-of-truth directory layout.
# [templates]
# paths = [".templates", "shared/templates"]

# `[[formatters]]` lets `mdt update` and `mdt check` compare against your
# formatter's canonical output instead of raw injected text.
#
# Formatter commands are rendered with minijinja before execution.
# Available variables:
# - `{{ filePath }}`         -> absolute path to the file being formatted
# - `{{ relativeFilePath }}` -> path relative to the project root
# - `{{ rootDirectory }}`    -> absolute path to the project root
#
# `patterns` and `ignore` are both ordered rule lists with gitignore-like
# globs. A leading `!` negates a prior match, so later rules can re-include
# paths.
#
# This repo enables dprint for generated markdown targets to prevent the
# formatter cycle from issue #46, where `mdt update` and `dprint fmt` would
# otherwise keep disagreeing in CI.
[[formatters]]
command = "dprint fmt --stdin \"{{ filePath }}\""
patterns = ["**/*.md"]
ignore = ["**/*.t.md"]

# Add more formatter stages when different file types need different tools.
# [[formatters]]
# command = "prettier --stdin-filepath \"{{ filePath }}\""
# patterns = ["**/*.ts", "**/*.tsx"]
{% endraw %}

<!-- {/mdtAnnotatedConfiguration} -->

<!-- {@mdtInitAnnotatedConfiguration} -->

{% raw %}
# mdt.toml
#
# Welcome to mdt. This starter config is intentionally fully annotated so you
# can discover every supported option in one place.
#
# Uncomment only what your project needs. mdt works without a config file, but
# `mdt.toml` becomes useful once you want data interpolation, custom scanning
# rules, padding control, or formatter-aware convergence.
#
# When in doubt, start with a sample template + target block, run `mdt update`,
# and then come back here to enable the options that match your workflow.

# Maximum file size (in bytes) that mdt will scan before failing fast.
# Leave this commented to use the built-in default of 10 MB.
# max_file_size = 10485760

# By default mdt respects `.gitignore` and skips ignored files.
# Uncomment this only when ignored/generated files should still be scanned, or
# when you want `[include]` / `[exclude]` to be your only scanning rules.
# disable_gitignore = true

# `[padding]` controls the whitespace between tags and injected content.
# Supported values for `before` and `after`:
# - false -> keep content inline with the tag
# - 0     -> put content on the next line with no blank line
# - 1     -> add one blank line
# - 2+    -> add two or more blank lines
#
# Recommended when your targets live in source-code comments or when formatters
# tend to rewrite surrounding whitespace.
# [padding]
# before = 0
# after = 0

# `[data]` maps namespaces to external data sources. These values are available
# in source blocks through minijinja templates like `{{ package.version }}`.
#
# String values are file-backed sources. The parser is inferred from the file
# extension (`.json`, `.toml`, `.yaml`, `.yml`, `.kdl`, `.ini`).
# [data]
# package = "package.json"
# cargo = "Cargo.toml"
# config = "config.yaml"
#
# Typed data sources force a parser when the extension is missing or unusual.
# release = { path = "release-info", format = "json" }
#
# Script-backed data sources run a shell command from the project root and parse
# stdout. `format` accepts: `text`, `string`, `raw`, `txt`, `json`, `toml`,
# `yaml`, `yml`, `kdl`, or `ini`.
# `watch` files control cache invalidation in `.mdt/cache/data-v1.json`.
# version = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
# git = { command = "git rev-parse --short HEAD", format = "text" }

# `[exclude]` skips files, directories, or block names during scanning.
# `patterns` use gitignore-style syntax, including `!negation`, trailing `/`,
# `*`, `**`, and character classes.
# [exclude]
# patterns = ["vendor/", "dist/", "generated/", "!generated/keep.md"]
#
# `markdown_codeblocks` only affects fenced code blocks inside source-file
# comments. Supported values:
# - false        -> process tags in code blocks normally (default)
# - true         -> ignore tags in all fenced code blocks
# - "..."        -> ignore code blocks whose info string contains that text
# - ["...", ...] -> ignore code blocks matching any listed info-string text
# markdown_codeblocks = true
# markdown_codeblocks = "ignore"
# markdown_codeblocks = ["ignore", "example"]
#
# `blocks` excludes specific block names everywhere, even if their files are
# still scanned.
# blocks = ["draftSection", "deprecatedApi"]

# `[include]` narrows scanning to only matching files. Use it when you want a
# smaller, more predictable scan surface in large repos.
# [include]
# patterns = ["docs/**/*.rs", "src/**/*.ts", "packages/*/readme.md"]

# `[templates]` restricts where `*.t.md` provider files are discovered.
# Leave it commented to allow template discovery anywhere in the project.
# [templates]
# paths = [".templates", "shared/templates"]

# `[[formatters]]` makes `mdt update` and `mdt check` converge with your
# formatter's canonical output.
#
# This is the recommended fix when `mdt update`, your formatter, and
# `mdt check` would otherwise bounce back and forth in CI.
#
# Formatter `command` values are rendered with minijinja before execution.
# Available variables:
# - `{{ filePath }}`         -> absolute path to the file being formatted
# - `{{ relativeFilePath }}` -> path relative to the project root
# - `{{ rootDirectory }}`    -> absolute path to the project root
#
# `patterns` and `ignore` are both ordered gitignore-style rule lists. A
# leading `!` negates a prior match, so later rules can re-include paths.
#
# Start with one catch-all formatter when your repo already uses a router like
# dprint. Add more entries when different file types need different tools.
# [[formatters]]
# command = "dprint fmt --stdin \"{{ filePath }}\""
# patterns = ["**/*.md"]
# ignore = ["**/*.t.md", "**/*.snap"]
#
# [[formatters]]
# command = "prettier --stdin-filepath \"{{ filePath }}\""
# patterns = ["**/*.ts", "**/*.tsx"]
# ignore = ["dist/**"]
{% endraw %}

<!-- {/mdtInitAnnotatedConfiguration} -->

<!-- {@mdtInitAnnotatedConfigurationRust} -->

{% raw %}
pub(crate) const DEFAULT_MDT_TOML: &str = r####"# mdt.toml
#
# Welcome to mdt. This starter config is intentionally fully annotated so you
# can discover every supported option in one place.
#
# Uncomment only what your project needs. mdt works without a config file, but
# `mdt.toml` becomes useful once you want data interpolation, custom scanning
# rules, padding control, or formatter-aware convergence.
#
# When in doubt, start with a sample template + target block, run `mdt update`,
# and then come back here to enable the options that match your workflow.

# Maximum file size (in bytes) that mdt will scan before failing fast.
# Leave this commented to use the built-in default of 10 MB.
# max_file_size = 10485760

# By default mdt respects `.gitignore` and skips ignored files.
# Uncomment this only when ignored/generated files should still be scanned, or
# when you want `[include]` / `[exclude]` to be your only scanning rules.
# disable_gitignore = true

# `[padding]` controls the whitespace between tags and injected content.
# Supported values for `before` and `after`:
# - false -> keep content inline with the tag
# - 0     -> put content on the next line with no blank line
# - 1     -> add one blank line
# - 2+    -> add two or more blank lines
#
# Recommended when your targets live in source-code comments or when formatters
# tend to rewrite surrounding whitespace.
# [padding]
# before = 0
# after = 0

# `[data]` maps namespaces to external data sources. These values are available
# in source blocks through minijinja templates like `{{ package.version }}`.
#
# String values are file-backed sources. The parser is inferred from the file
# extension (`.json`, `.toml`, `.yaml`, `.yml`, `.kdl`, `.ini`).
# [data]
# package = "package.json"
# cargo = "Cargo.toml"
# config = "config.yaml"
#
# Typed data sources force a parser when the extension is missing or unusual.
# release = { path = "release-info", format = "json" }
#
# Script-backed data sources run a shell command from the project root and parse
# stdout. `format` accepts: `text`, `string`, `raw`, `txt`, `json`, `toml`,
# `yaml`, `yml`, `kdl`, or `ini`.
# `watch` files control cache invalidation in `.mdt/cache/data-v1.json`.
# version = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
# git = { command = "git rev-parse --short HEAD", format = "text" }

# `[exclude]` skips files, directories, or block names during scanning.
# `patterns` use gitignore-style syntax, including `!negation`, trailing `/`,
# `*`, `**`, and character classes.
# [exclude]
# patterns = ["vendor/", "dist/", "generated/", "!generated/keep.md"]
#
# `markdown_codeblocks` only affects fenced code blocks inside source-file
# comments. Supported values:
# - false        -> process tags in code blocks normally (default)
# - true         -> ignore tags in all fenced code blocks
# - "..."        -> ignore code blocks whose info string contains that text
# - ["...", ...] -> ignore code blocks matching any listed info-string text
# markdown_codeblocks = true
# markdown_codeblocks = "ignore"
# markdown_codeblocks = ["ignore", "example"]
#
# `blocks` excludes specific block names everywhere, even if their files are
# still scanned.
# blocks = ["draftSection", "deprecatedApi"]

# `[include]` narrows scanning to only matching files. Use it when you want a
# smaller, more predictable scan surface in large repos.
# [include]
# patterns = ["docs/**/*.rs", "src/**/*.ts", "packages/*/readme.md"]

# `[templates]` restricts where `*.t.md` provider files are discovered.
# Leave it commented to allow template discovery anywhere in the project.
# [templates]
# paths = [".templates", "shared/templates"]

# `[[formatters]]` makes `mdt update` and `mdt check` converge with your
# formatter's canonical output.
#
# This is the recommended fix when `mdt update`, your formatter, and
# `mdt check` would otherwise bounce back and forth in CI.
#
# Formatter `command` values are rendered with minijinja before execution.
# Available variables:
# - `{{ filePath }}`         -> absolute path to the file being formatted
# - `{{ relativeFilePath }}` -> path relative to the project root
# - `{{ rootDirectory }}`    -> absolute path to the project root
#
# `patterns` and `ignore` are both ordered gitignore-style rule lists. A
# leading `!` negates a prior match, so later rules can re-include paths.
#
# Start with one catch-all formatter when your repo already uses a router like
# dprint. Add more entries when different file types need different tools.
# [[formatters]]
# command = "dprint fmt --stdin \"{{ filePath }}\""
# patterns = ["**/*.md"]
# ignore = ["**/*.t.md", "**/*.snap"]
#
# [[formatters]]
# command = "prettier --stdin-filepath \"{{ filePath }}\""
# patterns = ["**/*.ts", "**/*.tsx"]
# ignore = ["dist/**"]
"####;
{% endraw %}

<!-- {/mdtInitAnnotatedConfigurationRust} -->

<!-- {@mdtLspOverview} -->

`mdt_lsp` is a [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) implementation for the [mdt](https://github.com/ifiokjr/mdt) template engine. It provides real-time editor integration for managing markdown template blocks.

### Capabilities

- **Diagnostics** — reports stale target blocks, missing sources (with name suggestions), duplicate sources, unclosed blocks, unknown transformers, invalid arguments, unused sources, and source blocks in non-template files.
- **Completions** — suggests block names after `{=`, `{~`, `{@`, and `{/` tags, and transformer names after `|`.
- **Hover** — shows provider source, rendered content, transformer chain, and consumer count when hovering over a block tag.
- **Go to definition** — navigates from a target block to its provider, or from a source to all of its consumers.
- **References** — finds all source, target, and inline blocks sharing the same name.
- **Rename** — renames a block across all provider and target tags (both opening and closing) in the workspace.
- **Document symbols** — lists source, target, and inline blocks in the outline/symbol view.
- **Code actions** — offers a quick-fix to update stale target blocks in place.

### Usage

Start the language server via the CLI:

```sh
mdt lsp
```

The server communicates over stdin/stdout using the Language Server Protocol.

<!-- {/mdtLspOverview} -->

<!-- {@mdtMcpOverview} -->

`mdt_mcp` is a [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) server for the [mdt](https://github.com/ifiokjr/mdt) template engine. It exposes mdt functionality as MCP tools that can be used by AI assistants and other MCP-compatible clients.

### Tools

- **`mdt_check`** — Verify all target blocks are up-to-date.
- **`mdt_update`** — Update all target blocks with latest source content.
- **`mdt_list`** — List all sources and targets in the project.
- **`mdt_find_reuse`** — Find similar providers and where they are already consumed in markdown and source files to encourage reuse.
- **`mdt_get_block`** — Get the content of a specific block by name.
- **`mdt_preview`** — Preview the result of applying transformers to a block.
- **`mdt_init`** — Initialize a new mdt project with a sample `.templates/template.t.md` file and starter `mdt.toml`.

### Agent Workflow

- Prefer reuse before creation: call `mdt_find_reuse` (or `mdt_list`) before introducing a new source block.
- Use the JSON-first tool responses as the source of truth. The MCP server returns structured payloads so agents can inspect results without parsing prose.
- Use `mdt_preview` as an authoring workflow: inspect the source template plus each target's rendered output before deciding whether to reuse, edit, or sync.
- Keep source names global and unique in the project to avoid collisions.
- After edits, run `mdt_check` (and optionally `mdt_update`) so target blocks stay synchronized.

### Usage

Start the MCP server via the CLI:

```sh
mdt mcp
```

Add the following to your MCP client configuration:

```json
{
	"mcpServers": {
		"mdt": {
			"command": "mdt",
			"args": ["mcp"]
		}
	}
}
```

<!-- {/mdtMcpOverview} -->

<!-- {@mdtContributing} -->

[`devenv`](https://devenv.sh/) is used to provide a reproducible development environment for this project. Follow the [getting started instructions](https://devenv.sh/getting-started/).

To automatically load the environment you should [install direnv](https://devenv.sh/automatic-shell-activation/) and then load the `direnv`.

```bash
# The security mechanism didn't allow to load the `.envrc`.
# Since we trust it, let's allow it execution.
direnv allow .
```

At this point you should see the `nix` commands available in your terminal. Run `install:all` to install all tooling and dependencies.

<!-- {/mdtContributing} -->

<!-- {@mdtCoreInstall} -->

```toml
[dependencies]
mdt_core = "{{ cargo.workspace.package.version }}"
```

<!-- {/mdtCoreInstall} -->

<!-- {@mdtLspInstall} -->

```toml
[dependencies]
mdt_lsp = "{{ cargo.workspace.package.version }}"
```

<!-- {/mdtLspInstall} -->

<!-- {@mdtMcpInstall} -->

```toml
[dependencies]
mdt_mcp = "{{ cargo.workspace.package.version }}"
```

<!-- {/mdtMcpInstall} -->

<!-- {@mdtCliInstall} -->

- Install with npm:

```sh
npm install -g @m-d-t/cli
```

- Or run it without installing:

```sh
npx @m-d-t/cli --help
```

- Or download a prebuilt binary from the [latest GitHub release](https://github.com/ifiokjr/mdt/releases/latest)
- Or install with Cargo:

```sh
cargo install mdt_cli
```

<!-- {/mdtCliInstall} -->

<!-- {@mdtCoreOverview} -->

`mdt_core` is the core library for the [mdt](https://github.com/ifiokjr/mdt) template engine. It provides the lexer, parser, project scanner, and template engine for processing markdown template tags. Content defined once in source blocks can be distributed to target blocks across markdown files, code documentation comments, READMEs, and more.

## Processing Pipeline

```text
Markdown / source file
  → Lexer (tokenizes HTML comments into TokenGroups)
  → Pattern matcher (validates token sequences)
  → Parser (classifies groups, extracts names + transformers, matches open/close into Blocks)
  → Project scanner (walks directory tree, builds source→content map + target list)
  → Engine (matches targets to sources, applies transformers, replaces content)
```

## Modules

- [`config`] — Configuration loading from `mdt.toml`, including data source mappings, exclude/include patterns, and template paths.
- [`project`] — Project scanning and directory walking. Discovers provider and target blocks across all files in a project.
- [`source_scanner`] — Source file scanning for target tags inside code comments (Rust, TypeScript, Python, Go, Java, etc.).

## Key Types

- [`Block`] — A parsed template block (source or target) with its name, type, position, and transformers.
- [`Transformer`] — A pipe-delimited content filter (e.g., `trim`, `indent`, `linePrefix`) applied during injection.
- [`ProjectContext`] — A scanned project together with its loaded template data, ready for checking or updating.
- [`MdtConfig`] — Configuration loaded from `mdt.toml`.
- [`CheckResult`] — Result of checking a project for stale targets.
- [`UpdateResult`] — Result of computing updates for target blocks.

## Data Interpolation

Provider content supports [`minijinja`](https://docs.rs/minijinja) template variables populated from project files. The `mdt.toml` config maps source files to namespaces:

```toml
[data]
pkg = "package.json"
cargo = "Cargo.toml"
```

Then in source blocks: `{{ "{{" }} pkg.version {{ "}}" }}` or `{{ "{{" }} cargo.package.edition {{ "}}" }}`.

Supported sources: files and script commands. Supported formats: text, JSON, TOML, YAML, KDL, and INI.

## Quick Start

```rust,no_run
use mdt_core::project::scan_project_with_config;
use mdt_core::{check_project, compute_updates, write_updates};
use std::path::Path;

let ctx = scan_project_with_config(Path::new(".")).unwrap();

// Check for stale targets
let result = check_project(&ctx).unwrap();
if !result.is_ok() {
    eprintln!("{} stale target(s) found", result.stale.len());
}

// Update all target blocks
let updates = compute_updates(&ctx).unwrap();
write_updates(&updates).unwrap();
```

<!-- {/mdtCoreOverview} -->

<!-- {@mdtBlockDocs} -->

A parsed template block representing either a source or consumer.

Source blocks are defined in `*.t.md` template files using `{@name}...{/name}` tag syntax (wrapped in HTML comments). They supply content that gets distributed to matching consumers.

Target blocks appear in any scanned file using `{=name}...{/name}` tag syntax (wrapped in HTML comments). Their content is replaced with the matching source's content (after applying any transformers) when `mdt update` is run.

Each block tracks its [`name`](Block::name) for source-target matching, its [`BlockType`], the [`Position`] of its opening and closing tags, and any [`Transformer`]s to apply during content injection.

<!-- {/mdtBlockDocs} -->

<!-- {@mdtTransformerDocs} -->

A content transformer applied to source content during injection into a target block.

Transformers are specified using pipe-delimited syntax after the block name in a target tag:

```markdown
<!-- {=blockName|trim|indent:"  "|linePrefix:"/// "} -->
```

Transformers are applied in left-to-right order. Each transformer has a [`TransformerType`] and zero or more [`Argument`]s passed via colon-delimited syntax (e.g., `indent:"  "`).

Available transformers: `trim`, `trimStart`, `trimEnd`, `indent`, `prefix`, `suffix`, `linePrefix`, `lineSuffix`, `wrap`, `codeBlock`, `code`, `replace`, `if`.

<!-- {/mdtTransformerDocs} -->

<!-- {@mdtArgumentDocs} -->

An argument value passed to a [`Transformer`].

Arguments are specified after the transformer name using colon-delimited syntax:

```markdown
<!-- {=block|replace:"old":"new"|indent:"  "} -->
```

Three types are supported:

- **String** — Quoted text, e.g. `"hello"` or `'hello'`
- **Number** — Integer or floating-point, e.g. `42` or `3.14`
- **Boolean** — `true` or `false`

<!-- {/mdtArgumentDocs} -->

<!-- {@mdtBadgeLinks:"crateName"} -->

[crate-image]: https://img.shields.io/crates/v/{{ crateName }}.svg [crate-link]: https://crates.io/crates/{{ crateName }} [docs-image]: https://docs.rs/{{ crateName }}/badge.svg [docs-link]: https://docs.rs/{{ crateName }}/ [ci-status-image]: https://github.com/ifiokjr/mdt/workflows/ci/badge.svg [ci-status-link]: https://github.com/ifiokjr/mdt/actions?query=workflow:ci [coverage-image]: https://codecov.io/gh/ifiokjr/mdt/branch/main/graph/badge.svg [coverage-link]: https://codecov.io/gh/ifiokjr/mdt [unlicense-image]: https://img.shields.io/badge/license-Unlicence-blue.svg [unlicense-link]: https://opensource.org/license/unlicense

<!-- {/mdtBadgeLinks} -->

<!-- {@mdtBeforeAfter} -->

## The Problem

You have the same install instructions in three places:

**readme.md:**

```markdown
## Installation

npm install my-lib
```

**src/lib.rs:**

```rust
//! ## Installation
//!
//! npm install my-lib
```

**docs/getting-started.md:**

```markdown
## Installation

npm install my-lib
```

You update one. The others drift. CI doesn't catch it.

## The Fix

Define it once in a `*.t.md` template file (the "t" stands for template):

```markdown
<!-- {@install} -->

npm install my-lib

<!-- {/install} -->
```

Use it everywhere:

```markdown
<!-- {=install} -->

(replaced automatically)

<!-- {/install} -->
```

Run `mdt update` — all three files are in sync. Run `mdt check` in CI — drift is caught before merge.

<!-- {/mdtBeforeAfter} -->

<!-- {@mdtQuickStart} -->

### 1. Initialize

```sh
mkdir my-project && cd my-project
mdt init
```

This creates `.templates/template.t.md` (your source blocks) and `mdt.toml` (config).

### 2. Define a source block

In `.templates/template.t.md`:

```markdown
<!-- {@greeting} -->

Hello from mdt!

<!-- {/greeting} -->
```

### 3. Use it in your README

In `readme.md`:

```markdown
<!-- {=greeting} -->
<!-- {/greeting} -->
```

### 4. Sync

```sh
mdt update
```

Every target block named `greeting` now has the same content. Run `mdt check` in CI to catch drift.

<!-- {/mdtQuickStart} -->
