# mdt

**Write it once, sync it everywhere. Doc drift is dead.**

Markdown templates that keep your READMEs, doc comments, and docs sites in lockstep — with data interpolation, transformers, and CI verification.

<br />

[![Status][ci-status-image]][ci-status-link] [![Coverage][coverage-image]][coverage-link] [![Unlicense][unlicense-image]][unlicense-link]

<br />

<!-- {=mdtPackageDocumentation} -->

`mdt` helps library and tool maintainers keep README sections, source-doc comments, and docs-site content synchronized across a project. Define content once with comment-based template tags, then reuse it across markdown files, code documentation comments, READMEs, mdbook docs, and more so your docs do not drift.

<!-- {/mdtPackageDocumentation} -->

<!-- {=mdtBeforeAfter} -->

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

## Quick Start

<!-- {=mdtQuickStart} -->

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

## Learn More

- [Template Syntax](./docs/src/reference/template-syntax.md)
- [CLI Reference](./docs/src/reference/cli.md)
- [Data Interpolation](./docs/src/guide/data-interpolation.md)
- [Transformers](./docs/src/reference/transformers.md)
- [CI Integration](./docs/src/guide/ci-integration.md)
- [Source File Support](./docs/src/guide/source-files.md)
- [Proof of Value](./docs/src/getting-started/proof-of-value.md)
- [Migration Walkthrough](./docs/src/getting-started/migration-walkthrough.md)

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
