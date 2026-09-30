<p align="center"><img src="https://raw.githubusercontent.com/ifiokjr/mdt/main/assets/mdt-mark.svg" alt="mdt logo" width="110"></p>

# Introduction

**mdt** (manage **m**ark**d**own **t**emplates) keeps README sections, source-doc comments, and docs-site content in sync. Define content once in a template file, reference it from anywhere, and mdt updates every copy for you, in READMEs, code comments, or mdBook docs.

Library and tool maintainers duplicate documentation constantly:

- install instructions repeated in a root README, crate README, and package docs
- usage snippets duplicated between source-doc comments and a docs site
- version numbers, package names, and commands scattered across multiple files

When something changes, one copy gets updated and the others drift, so users read conflicting information.

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

Run `mdt update` and all three files are in sync. Run `mdt check` in CI and drift is caught before merge.

<!-- {/mdtBeforeAfter} -->

## How It Works

mdt tags are HTML comments, so they are invisible in rendered markdown. A **provider** block in a template file (`*.t.md`) defines the content once. **Consumer** blocks mark every place that content should appear, in markdown files or in source-code comments. `mdt update` replaces each consumer's content with its provider's, and `mdt check` fails when any consumer is out of date.

## See It in Practice

- [Quick Start](./getting-started/quick-start.md) builds a small project step by step
- [Proof of Value](./getting-started/proof-of-value.md) shows how this repository keeps README content, Rust source docs, and mdBook pages synchronized
- [Migration Walkthrough](./getting-started/migration-walkthrough.md) is a before/after adoption path you can copy into your own project

## Key Features

- **Comment-based tags**: HTML comments are invisible in rendered markdown, so your docs look clean
- **Source file support**: consumer tags work inside code comments too (Rust, TypeScript, Python, Go, Dart, and more)
- **Data interpolation**: pull values from `package.json`, `Cargo.toml`, other data files, or command output into providers with `{{ variable }}` syntax
- **Transformers**: pipe content through `trim`, `indent`, `linePrefix`, `codeBlock`, and more to adapt shared content for each context
- **CI-friendly**: `mdt check` exits non-zero when docs are stale or tags are broken, with JSON and GitHub Actions output formats
- **Project diagnostics**: `mdt info` and `mdt doctor` report project health, cache status, and actionable remediation hints
- **Watch mode**: `mdt update --watch` re-syncs on file changes during development
- **Editor support**: `mdt lsp` adds diagnostics, completions, hover, go-to-definition, and code actions in your editor
- **Agent support**: `mdt skill` teaches any coding agent how to use mdt, and `mdt mcp` exposes the same documentation graph to AI assistants through the Model Context Protocol
