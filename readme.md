# mdt

> manage **m**ark**d**own **t**emplates across your project

<br />

[![Status][ci-status-image]][ci-status-link] [![Coverage][coverage-image]][coverage-link] [![Unlicense][unlicense-image]][unlicense-link]

<br />

<!-- {=mdtPackageDocumentation} -->

`mdt` helps library and tool maintainers keep README sections, source-doc comments, and docs-site content synchronized across a project. Define content once with comment-based template tags, then reuse it across markdown files, code documentation comments, READMEs, mdbook docs, and more so your docs do not drift.

<!-- {/mdtPackageDocumentation} -->

## Installation

<!-- {=mdtCliInstall} -->

- Install with npm:

```sh
npm install -g @ifi/mdt
```

- Or run it without installing:

```sh
npx @ifi/mdt --help
```

- Or download a prebuilt binary from the [latest GitHub release](https://github.com/ifiokjr/mdt/releases/latest)
- Or install with Cargo:

```sh
cargo install mdt_cli
```

<!-- {/mdtCliInstall} -->

## See It in Practice

- [Proof of Value](./docs/src/getting-started/proof-of-value.md) shows how this repository already syncs README content, Rust source docs, and mdBook pages from shared providers.
- [Migration Walkthrough](./docs/src/getting-started/migration-walkthrough.md) shows a before/after adoption path for moving repeated docs onto `mdt`.

<!-- {=mdtTemplateSyntax} -->

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

**Inline tag** (provider-free interpolation using configured data):

```markdown
Current version: <!-- {~version:"{{ package.version }}"} -->0.0.0<!-- {/version} -->
```

```markdown
| Artifact | Version                                                                   |
| -------- | ------------------------------------------------------------------------- |
| mdt_cli  | <!-- {~cliVersion:"{{ package.version }}"} -->0.0.0<!-- {/cliVersion} --> |
```

**Filters and pipes:** Template values support pipe-delimited transformers:

```markdown
<!-- {=block|prefix:"\n"|indent:"  "} -->
```

Available transformers: `trim`, `trimStart`, `trimEnd`, `indent`, `prefix`, `suffix`, `linePrefix`, `lineSuffix`, `wrap`, `codeBlock`, `code`, `replace`, `if`.

<!-- {/mdtTemplateSyntax} -->

<!-- {=mdtCliUsage} -->

### CLI Commands

- `mdt init [--path <dir>]` — Create a sample `.templates/template.t.md` file and starter `mdt.toml`.
- `mdt check [--path <dir>] [--verbose]` — Verify all consumer blocks are up-to-date. Exits non-zero if any are stale.
- `mdt update [--path <dir>] [--verbose] [--dry-run]` — Update all consumer blocks with latest provider content.
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

## Agent Skill Package

If you use a coding agent that supports the [Agent Skills standard](https://agentskills.io) (like [Pi](https://github.com/badlogic/pi)), install the official mdt skill:

```sh
pi install npm:@ifi/mdt-skills
```

The [`@ifi/mdt-skills`](https://www.npmjs.com/package/@ifi/mdt-skills) package teaches your agent template syntax, MCP tools, CLI workflows, transformer patterns, and configuration — so it can fully manage your project's documentation templates.

## Crates

| Crate                    | Description                                                                    |
| ------------------------ | ------------------------------------------------------------------------------ |
| [`mdt_core`](./mdt_core) | Core library — lexer, parser, scanner, and template engine                     |
| [`mdt_cli`](./mdt_cli)   | CLI tool — `mdt` binary for managing templates                                 |
| [`mdt_lsp`](./mdt_lsp)   | LSP server — editor integration with diagnostics, completions, hover, and more |
| [`mdt_mcp`](./mdt_mcp)   | MCP server — AI assistant integration via the Model Context Protocol           |

## Contributing

<!-- {=mdtContributing} -->

[`devenv`](https://devenv.sh/) is used to provide a reproducible development environment for this project. Follow the [getting started instructions](https://devenv.sh/getting-started/).

To automatically load the environment you should [install direnv](https://devenv.sh/automatic-shell-activation/) and then load the `direnv`.

```bash
# The security mechanism didn't allow to load the `.envrc`.
# Since we trust it, let's allow it execution.
direnv allow .
```

At this point you should see the `nix` commands available in your terminal. Run `install:all` to install all tooling and dependencies.

<!-- {/mdtContributing} -->

[ci-status-image]: https://github.com/ifiokjr/mdt/workflows/ci/badge.svg
[ci-status-link]: https://github.com/ifiokjr/mdt/actions?query=workflow:ci
[coverage-image]: https://codecov.io/gh/ifiokjr/mdt/branch/main/graph/badge.svg
[coverage-link]: https://codecov.io/gh/ifiokjr/mdt
[unlicense-image]: https://img.shields.io/badge/license-Unlicence-blue.svg
[unlicense-link]: https://opensource.org/license/unlicense
