<!-- {@mdtPackageDocumentation} -->

`mdt` is a data-driven template engine for keeping documentation synchronized across your project. It uses comment-based template tags to define content once and distribute it to multiple locations — markdown files, code documentation comments (in any language), READMEs, mdbook docs, and more.

<!-- {/mdtPackageDocumentation} -->

<!-- {@mdtCliUsage} -->

### CLI Commands

- `mdt init [--path <dir>]` — Create a sample `template.t.md` file with getting-started instructions.
- `mdt check [--path <dir>] [--verbose]` — Verify all consumer blocks are up-to-date. Exits non-zero if any are stale.
- `mdt update [--path <dir>] [--verbose] [--dry-run]` — Update all consumer blocks with latest provider content.
- `mdt lsp` — Start the mdt language server (LSP) for editor integration. Communicates over stdin/stdout.
- `mdt mcp` — Start the mdt MCP server for AI assistants. Communicates over stdin/stdout.

<!-- {/mdtCliUsage} -->

<!-- {@mdtTemplateSyntax} -->

### Template Syntax

**Provider tag** (defines a template block in `*.t.md` definition files):

```markdown
<!-- {@blockName} -->

Content to inject

<!-- {/blockName} -->
```

**Consumer tag** (marks where content should be injected):

```markdown
<!-- {=blockName} -->

This content gets replaced

<!-- {/blockName} -->
```

**Filters and pipes:** Template values support pipe-delimited transformers:

```markdown
<!-- {=block|prefix:"\n"|indent:"  "} -->
```

Available transformers: `trim`, `trimStart`, `trimEnd`, `indent`, `prefix`, `wrap`, `codeBlock`, `code`, `replace`.

<!-- {/mdtTemplateSyntax} -->

<!-- {@mdtCoreInstall} -->

```toml
[dependencies]
mdt_core = "{{ cargo_mdt_core.package.version }}"
```

<!-- {/mdtCoreInstall} -->

<!-- {@mdtLspInstall} -->

```toml
[dependencies]
mdt_lsp = "{{ cargo_mdt_cli.package.version }}"
```

<!-- {/mdtLspInstall} -->

<!-- {@mdtMcpInstall} -->

```toml
[dependencies]
mdt_mcp = "{{ cargo_mdt_mcp.package.version }}"
```

<!-- {/mdtMcpInstall} -->

<!-- {@mdtCoreOverview} -->

`mdt_core` is the core library for the [mdt](https://github.com/ifiokjr/mdt) template engine. It provides the lexer, parser, project scanner, and template engine for processing markdown template tags. Content defined once in provider blocks can be distributed to consumer blocks across markdown files, code documentation comments, READMEs, and more.

## Processing Pipeline

```text
Markdown / source file
  → Lexer (tokenizes HTML comments into TokenGroups)
  → Pattern matcher (validates token sequences)
  → Parser (classifies groups, extracts names + transformers, matches open/close into Blocks)
  → Project scanner (walks directory tree, builds provider→content map + consumer list)
  → Engine (matches consumers to providers, applies transformers, replaces content)
```

## Modules

- [`config`] — Configuration loading from `mdt.toml`, including data source mappings, exclude/include patterns, and template paths.
- [`project`] — Project scanning and directory walking. Discovers provider and consumer blocks across all files in a project.
- [`source_scanner`] — Source file scanning for consumer tags inside code comments (Rust, TypeScript, Python, Go, Java, etc.).

## Key Types

- [`Block`] — A parsed template block (provider or consumer) with its name, type, position, and transformers.
- [`Transformer`] — A pipe-delimited content filter (e.g., `trim`, `indent`, `linePrefix`) applied during injection.
- [`ProjectContext`] — A scanned project together with its loaded template data, ready for checking or updating.
- [`MdtConfig`] — Configuration loaded from `mdt.toml`.
- [`CheckResult`] — Result of checking a project for stale consumers.
- [`UpdateResult`] — Result of computing updates for consumer blocks.

## Data Interpolation

Provider content supports [`minijinja`](https://docs.rs/minijinja) template variables populated from project files. The `mdt.toml` config maps source files to namespaces:

```toml
[data]
pkg = "package.json"
cargo = "Cargo.toml"
```

Then in provider blocks: `{{ "{{" }} pkg.version {{ "}}" }}` or `{{ "{{" }} cargo.package.edition {{ "}}" }}`.

Supported formats: JSON, TOML, YAML, and KDL.

## Quick Start

```rust,no_run
use mdt_core::project::scan_project_with_config;
use mdt_core::{check_project, compute_updates, write_updates};
use std::path::Path;

let ctx = scan_project_with_config(Path::new(".")).unwrap();

// Check for stale consumers
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

A parsed template block representing either a provider or consumer.

Provider blocks are defined in `*.t.md` template files using `{@name}...{/name}` tag syntax (wrapped in HTML comments). They supply content that gets distributed to matching consumers.

Consumer blocks appear in any scanned file using `{=name}...{/name}` tag syntax (wrapped in HTML comments). Their content is replaced with the matching provider's content (after applying any transformers) when `mdt update` is run.

Each block tracks its [`name`](Block::name) for provider-consumer matching, its [`BlockType`], the [`Position`] of its opening and closing tags, and any [`Transformer`]s to apply during content injection.

<!-- {/mdtBlockDocs} -->

<!-- {@mdtTransformerDocs} -->

A content transformer applied to provider content during injection into a consumer block.

Transformers are specified using pipe-delimited syntax after the block name in a consumer tag:

```markdown
<!-- {=blockName|trim|indent:"  "|linePrefix:"/// "} -->
```

Transformers are applied in left-to-right order. Each transformer has a [`TransformerType`] and zero or more [`Argument`]s passed via colon-delimited syntax (e.g., `indent:"  "`).

Available transformers: `trim`, `trimStart`, `trimEnd`, `indent`, `prefix`, `suffix`, `linePrefix`, `lineSuffix`, `wrap`, `codeBlock`, `code`, `replace`.

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

[coverage-image]: https://codecov.io/gh/ifiokjr/mdt/branch/main/graph/badge.svg
[coverage-link]: https://codecov.io/gh/ifiokjr/mdt
[crate-image]: https://img.shields.io/crates/v/{{ crateName }}.svg
[crate-link]: https://crates.io/crates/{{ crateName }}
[docs-image]: https://docs.rs/{{ crateName }}/badge.svg
[docs-link]: https://docs.rs/{{ crateName }}/
[ci-status-image]: https://github.com/ifiokjr/mdt/workflows/ci/badge.svg
[ci-status-link]: https://github.com/ifiokjr/mdt/actions?query=workflow:ci
[unlicense-image]: https://img.shields.io/badge/license-Unlicence-blue.svg
[unlicense-link]: https://opensource.org/license/unlicense

<!-- {/mdtBadgeLinks} -->
