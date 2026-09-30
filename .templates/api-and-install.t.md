<!-- {@mdtLspOverview} -->

`mdt_lsp` is a [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) server for [mdt](https://github.com/ifiokjr/mdt). It shows mdt diagnostics in your editor and lets you navigate, rename, and update blocks without leaving it.

### Capabilities

- **Diagnostics**: stale consumers and inline blocks, provider render failures, orphan consumers (with a did-you-mean suggestion), unclosed blocks, unmatched closing tags, comments that look like tags but do not parse, blocks nested inside a consumer, unknown transformers and wrong argument counts, duplicate providers, unused providers, and providers outside `*.t.md` files.
- **Quick fix**: "Update block" rewrites a stale consumer exactly as `mdt update` would, padding and comment prefixes included.
- **Completions**: block names after `{=`, `{~`, `{@`, and `{/`, and transformer names after `|`.
- **Hover**: a block's provider, rendered content, transformer chain, and consumer count.
- **Go to definition**: from a consumer to its provider, or from a provider to its consumers.
- **References**: every provider, consumer, and inline block with the same name.
- **Rename**: a block name in every opening and closing tag in the workspace.
- **Document symbols**: providers, consumers, and inline blocks in the outline view.

The server reads `mdt.toml` and honors `[padding]`, `[check] comparison`, and `[exclude] markdown_codeblocks`. It does not run `[[formatters]]`, so with formatters configured it can report a block as stale that `mdt check` accepts.

### Usage

Start the language server through the CLI:

```sh
mdt lsp
```

It communicates over stdin/stdout and uses the editor's workspace folder as the project root.

<!-- {/mdtLspOverview} -->

<!-- {@mdtMcpOverview} -->

`mdt_mcp` is a [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) server for [mdt](https://github.com/ifiokjr/mdt). It gives AI assistants structured, JSON-first access to a project's providers and consumers.

### Tools

- **`mdt_list`**: providers and consumers with locations, transformers, arguments, and status (`current`, `stale`, `render_error`, or `orphan`). Pass `include_content: true` to include block content.
- **`mdt_check`**: stale consumers, stale files, orphans with suggestions, render errors, and diagnostics.
- **`mdt_update`**: sync every consumer; `dry_run: true` previews. It refuses to write while validation errors exist.
- **`mdt_find_reuse`**: rank existing providers by name (`block_name`) or content (`content_query`) before you create a new one.
- **`mdt_preview`**: a provider and each consumer's rendered and current content.
- **`mdt_get_block`**: a provider (or `null`) and its consumers, by name.
- **`mdt_init`**: set up a project the same way `mdt init` does.

Every tool accepts an optional project `path`. `mdt_list`, `mdt_check`, and `mdt_update` also accept `ignore_unclosed_blocks`, `ignore_unused_blocks`, `ignore_invalid_names`, and `ignore_invalid_transformers`.

### Responses

Every response is a JSON object with `ok`, `action`, and `summary`. A failure is a normal tool result with `ok: false` and `error: { code, message, help }`, for example `mdt::config_parse` or `mdt::path_outside_root`, rather than a protocol error.

### Agent Workflow

- Call `mdt_find_reuse` (or `mdt_list`) before creating a provider, and reuse one that fits.
- Use `mdt_preview` to see what each consumer will contain before syncing.
- Keep provider names unique across the project.
- After editing providers, run `mdt_update`, then `mdt_check`.
- Run `mdt skill` for the full agent guide.

### Usage

Start the MCP server through the CLI. Its project root is the directory given with `--path`; without it, the server walks up from its working directory to the nearest directory with an mdt config file, like the CLI. Tool `path` arguments must resolve inside that root.

```sh
mdt mcp
```

Add it to your MCP client configuration:

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

`mdt assist <generic|claude|cursor|copilot|pi>` prints the exact setup for each client.

<!-- {/mdtMcpOverview} -->

<!-- {@mdtContributing} -->

[`devenv`](https://devenv.sh/) provides a reproducible development environment for this project. Follow its [getting started instructions](https://devenv.sh/getting-started/), then enter the environment from the repository root:

```bash
devenv shell
```

Run `install:all` to install the remaining tooling. Repository commands such as `build:all`, `test:all`, `lint:all`, and `fix:all` are available inside the shell.

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

- Install the prebuilt binary with npm:

```sh
npm install -g @m-d-t/cli
```

- Or run it without installing:

```sh
npx -y @m-d-t/cli --help
```

- Or download a prebuilt binary from the [latest GitHub release](https://github.com/ifiokjr/mdt/releases/latest).
- Or build from source with Cargo (slower):

```sh
cargo install mdt_cli
```

<!-- {/mdtCliInstall} -->

<!-- {@mdtCoreOverview} -->

`mdt_core` is the core library for [mdt](https://github.com/ifiokjr/mdt). It provides the lexer, parser, project scanner, and template engine behind the `mdt` CLI, language server, and MCP server. Content defined once in a provider block is distributed to consumer blocks across markdown files, code documentation comments, READMEs, and more.

## Processing Pipeline

```text
Markdown / source file
  → Lexer (tokenizes HTML comments into TokenGroups)
  → Pattern matcher (validates token sequences)
  → Parser (classifies groups, extracts names + transformers, matches open/close into Blocks)
  → Project scanner (walks the project, collects providers from *.t.md files and consumers everywhere)
  → Engine (renders providers, applies transformers, replaces consumer content)
```

## Modules

- [`config`]: loads `mdt.toml`, including data sources, scan patterns, padding, comparison mode, and formatters.
- [`project`]: walks the project and discovers provider and consumer blocks.
- [`source_scanner`]: finds tags inside code comments (Rust, TypeScript, Python, Go, Java, Dart, and more).
- [`init`]: the project setup shared by `mdt init` and the MCP `mdt_init` tool.

## Key Types

- [`Block`]: a parsed provider, consumer, or inline block with its name, type, position, and transformers.
- [`Transformer`]: a pipe-delimited content filter (such as `trim`, `indent`, or `linePrefix`) applied during injection.
- [`ProjectContext`]: a scanned project with its loaded template data, ready for checking or updating.
- [`MdtConfig`]: configuration loaded from `mdt.toml`.
- [`CheckResult`]: stale consumers, stale files, orphans, and render errors found by a check.
- [`UpdateResult`]: the file contents to write, plus consumers skipped because their provider failed to render.

## Data Interpolation

When `mdt.toml` has a `[data]` section, provider content is rendered with [`minijinja`](https://docs.rs/minijinja) using values from project files and commands:

```toml
[data]
pkg = "package.json"
cargo = "Cargo.toml"
```

Providers can then use `{{ "{{" }} pkg.version {{ "}}" }}` or `{{ "{{" }} cargo.package.edition {{ "}}" }}`.

Supported sources: files and commands. Supported formats: text, JSON, TOML, YAML, KDL, and INI.

## Quick Start

```rust,no_run
use mdt_core::project::scan_project_with_config;
use mdt_core::{check_project, compute_updates, write_updates};
use std::path::Path;

let ctx = scan_project_with_config(Path::new(".")).unwrap();

// Check that every consumer is linked and current
let result = check_project(&ctx).unwrap();
if !result.is_ok() {
    eprintln!("{} stale consumer(s) found", result.stale.len());
}

// Update all consumer blocks
let updates = compute_updates(&ctx).unwrap();
write_updates(&updates).unwrap();
```

<!-- {/mdtCoreOverview} -->

<!-- {@mdtBlockDocs} -->

A parsed template block: a provider, a consumer, or an inline block.

Providers are defined in `*.t.md` template files with `{@name}...{/name}` tags (wrapped in HTML comments). They supply content to every consumer with the same name.

Consumers appear in any scanned file with `{=name}...{/name}` tags (wrapped in HTML comments). `mdt update` replaces their content with the matching provider's content, after applying any transformers.

Each block tracks its [`name`](Block::name) for provider-consumer matching, its [`BlockType`], the [`Position`] of its opening and closing tags, and any [`Transformer`]s to apply during content injection.

<!-- {/mdtBlockDocs} -->

<!-- {@mdtTransformerDocs} -->

A content transformer applied to provider content as it is injected into a consumer.

Transformers are written as pipe-delimited filters after the block name in a consumer tag:

```markdown
<!-- {=blockName|trim|indent:"  "|linePrefix:"/// ":true} -->
```

Transformers apply left to right. Each has a [`TransformerType`] and zero or more [`Argument`]s passed with colon-delimited syntax (for example `indent:"  "`).

Available transformers: `trim`, `trimStart`, `trimEnd`, `indent`, `prefix`, `suffix`, `linePrefix`, `lineSuffix`, `wrap`, `codeBlock`, `code`, `replace`, `if`.

<!-- {/mdtTransformerDocs} -->

<!-- {@mdtArgumentDocs} -->

An argument value passed to a [`Transformer`].

Arguments follow the transformer name with colon-delimited syntax:

```markdown
<!-- {=block|replace:"old":"new"|indent:"  "} -->
```

Three types are supported:

- **String**: quoted text, such as `"hello"` or `'hello'`. Only double-quoted strings decode escapes like `\n`.
- **Number**: an integer or float, such as `42` or `3.14`. Transformers that expect text use the number's text, so `indent:4` prepends `4`, not four spaces.
- **Boolean**: `true` or `false`.

<!-- {/mdtArgumentDocs} -->

<!-- {@mdtBadgeLinks:"crateName"} -->

[crate-image]: https://img.shields.io/crates/v/{{ crateName }}.svg
[crate-link]: https://crates.io/crates/{{ crateName }}
[docs-image]: https://docs.rs/{{ crateName }}/badge.svg
[docs-link]: https://docs.rs/{{ crateName }}/
[ci-status-image]: https://github.com/ifiokjr/mdt/workflows/ci/badge.svg
[ci-status-link]: https://github.com/ifiokjr/mdt/actions?query=workflow:ci
[coverage-image]: https://codecov.io/gh/ifiokjr/mdt/branch/main/graph/badge.svg
[coverage-link]: https://codecov.io/gh/ifiokjr/mdt
[unlicense-image]: https://img.shields.io/badge/license-Unlicense-blue.svg
[unlicense-link]: https://opensource.org/license/unlicense

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

Run `mdt update` and all three files are in sync. Run `mdt check` in CI and drift is caught before merge.

<!-- {/mdtBeforeAfter} -->

<!-- {@mdtQuickStart} -->

### 1. Initialize

```sh
mkdir my-project && cd my-project
mdt init
```

`mdt init` creates `mdt.toml`, a sample `greeting` provider in `.templates/template.t.md`, and, because the project has no README yet, a `readme.md` whose `greeting` consumer is already in sync.

### 2. Edit the provider

In `.templates/template.t.md`:

```markdown
<!-- {@greeting} -->

Hello from mdt! Edit me once, update everywhere.

<!-- {/greeting} -->
```

### 3. Reuse it

Add a consumer to any other markdown file, for example `docs/intro.md`:

```markdown
<!-- {=greeting} -->
<!-- {/greeting} -->
```

### 4. Sync and verify

```sh
mdt update
mdt check
```

`mdt update` writes the provider's content into every `greeting` consumer. `mdt check` exits non-zero when a consumer is out of date or names no provider, so run it in CI.

<!-- {/mdtQuickStart} -->

<!-- {@greeting} -->

Hello from mdt!

<!-- {/greeting} -->

<!-- {=greeting} -->

Hello from mdt!

<!-- {/greeting} -->
