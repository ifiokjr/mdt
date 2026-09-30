# mdt_cli

> the CLI for mdt (manage markdown templates)

<br />

[![Crate][crate-image]][crate-link] [![Docs][docs-image]][docs-link] [![Status][ci-status-image]][ci-status-link] [![Coverage][coverage-image]][coverage-link] [![Unlicense][unlicense-image]][unlicense-link]

<br />

<!-- {=mdtPackageDocumentation} -->

`mdt` helps library and tool maintainers keep README sections, source-doc comments, and docs-site content in sync across a project. Define content once with comment-based template tags, then reuse it across markdown files, code documentation comments, READMEs, mdbook docs, and more, so your docs do not drift.

<!-- {/mdtPackageDocumentation} -->

## Installation

<!-- {=mdtCliInstall} -->

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

<!-- {=mdtCliUsage} -->

### CLI Commands

- `mdt init` — Set up a project, adding only what is missing: an annotated `mdt.toml`, a sample provider in `.templates/template.t.md`, a synced `readme.md` when the project has no README, and a `.mdt/` entry in `.gitignore` in git repositories.
- `mdt check [--diff] [--format text|json|github] [--watch]` — Verify every consumer is linked to a provider and up to date. Exits 0 when in sync, 1 for stale or orphan consumers and render errors, 2 for validation or config errors.
- `mdt update [--dry-run] [--watch]` — Write the latest provider content into every consumer.
- `mdt list` — List providers and consumers with their locations and link status.
- `mdt info [--format text|json]` — Print a project summary, diagnostic totals, and cache metrics.
- `mdt doctor [--format text|json]` — Run health checks with fix hints. Exits 1 when a check fails.
- `mdt skill [--reference] [--install <DIR>]` — Print the agent skill for AI coding assistants, or install it into a skills directory.
- `mdt assist <generic|claude|cursor|copilot|pi> [--format text|json]` — Print MCP setup and skill guidance for an assistant.
- `mdt lsp` — Start the language server over stdin/stdout.
- `mdt mcp` — Start the MCP server over stdin/stdout.

### Global Options

- `-p, --path <DIR>` — Project root (default: the current directory). Must exist, except for `mdt init`.
- `-v, --verbose` — Print more detail, including warnings silenced by `--ignore-*` flags.
- `--no-color` — Disable colored output (`NO_COLOR` works too).
- `--ignore-unclosed-blocks`, `--ignore-unused-blocks`, `--ignore-invalid-names`, `--ignore-invalid-transformers` — Skip one class of diagnostics.

### Diagnostics Workflow

- Run `mdt info` to inspect project shape, diagnostic totals, and cache reuse.
- Run `mdt doctor` for health checks with remediation hints (config, data, layout, sync, cache).
- Set `MDT_CACHE_VERIFY_HASH=1` when troubleshooting cache consistency, and `MDT_LOG=debug` for debug logs on stderr.

<!-- {/mdtCliUsage} -->

<!-- {=mdtTemplateSyntax} -->

### Template Syntax

**Provider** (defines content; only recognized in `*.t.md` files):

```markdown
<!-- {@blockName} -->

Content to inject

<!-- {/blockName} -->
```

**Consumer** (its content is replaced by `mdt update`):

```markdown
<!-- {=blockName} -->

This content gets replaced

<!-- {/blockName} -->
```

**Inline block** (renders its template argument in place; needs `[data]` in `mdt.toml`):

```markdown
Current version: <!-- {~version:"{{ package.version }}"} -->0.0.0<!-- {/version} -->
```

```markdown
| Artifact | Version                                                                   |
| -------- | ------------------------------------------------------------------------- |
| mdt_cli  | <!-- {~cliVersion:"{{ package.version }}"} -->0.0.0<!-- {/cliVersion} --> |
```

**Transformers** change the content on its way into a consumer, applied left to right:

```markdown
<!-- {=blockName|trim|linePrefix:"/// ":true} -->
```

Available transformers: `trim`, `trimStart`, `trimEnd`, `indent`, `prefix`, `suffix`, `linePrefix`, `lineSuffix`, `wrap`, `codeBlock`, `code`, `replace`, `if`.

<!-- {/mdtTemplateSyntax} -->

<!-- {=mdtBadgeLinks:"mdt_cli"} -->

[crate-image]: https://img.shields.io/crates/v/mdt_cli.svg
[crate-link]: https://crates.io/crates/mdt_cli
[docs-image]: https://docs.rs/mdt_cli/badge.svg
[docs-link]: https://docs.rs/mdt_cli/
[ci-status-image]: https://github.com/ifiokjr/mdt/workflows/ci/badge.svg
[ci-status-link]: https://github.com/ifiokjr/mdt/actions?query=workflow:ci
[coverage-image]: https://codecov.io/gh/ifiokjr/mdt/branch/main/graph/badge.svg
[coverage-link]: https://codecov.io/gh/ifiokjr/mdt
[unlicense-image]: https://img.shields.io/badge/license-Unlicense-blue.svg
[unlicense-link]: https://opensource.org/license/unlicense

<!-- {/mdtBadgeLinks} -->
