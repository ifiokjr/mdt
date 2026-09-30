# Proof of Value

This repository is the best example of mdt solving a real problem. Providers in [`.templates/*.t.md`](https://github.com/ifiokjr/mdt/tree/main/.templates) keep repeated content in sync across:

- the root and crate READMEs
- crate-level and item-level Rust docs
- mdBook pages, including this book

Write shared content once, then fan it out wherever people actually read it.

## 1. README synchronization

The `mdtCliInstall` provider lives in [`.templates/api-and-install.t.md`](https://github.com/ifiokjr/mdt/blob/main/.templates/api-and-install.t.md). Two READMEs consume it:

- [`readme.md`](https://github.com/ifiokjr/mdt/blob/main/readme.md)
- [`mdt_cli/readme.md`](https://github.com/ifiokjr/mdt/blob/main/mdt_cli/readme.md)

The install instructions stay identical in both without copying edits by hand. Likewise, `mdtBeforeAfter` fills both the root README and the introduction of this book.

## 2. Source-doc synchronization

The `mdtLspOverview` provider, in the same template file, fans out into both a README and Rust crate docs:

- [`mdt_lsp/readme.md`](https://github.com/ifiokjr/mdt/blob/main/mdt_lsp/readme.md)
- [`mdt_lsp/src/lib.rs`](https://github.com/ifiokjr/mdt/blob/main/mdt_lsp/src/lib.rs)

The Rust consumer uses a transformer chain that turns the markdown into crate documentation comments:

```rust
//! <!-- {=mdtLspOverview|trim|linePrefix:"//! ":true} -->
//! <!-- {/mdtLspOverview} -->
```

The same pattern keeps these files in sync with their crate READMEs:

- [`mdt_core/src/lib.rs`](https://github.com/ifiokjr/mdt/blob/main/mdt_core/src/lib.rs) (`mdtCoreOverview`)
- [`mdt_mcp/src/lib.rs`](https://github.com/ifiokjr/mdt/blob/main/mdt_mcp/src/lib.rs) (`mdtMcpOverview`)

[`mdt_core/src/parser.rs`](https://github.com/ifiokjr/mdt/blob/main/mdt_core/src/parser.rs) uses `linePrefix:"/// ":true` for item-level docs on its public types.

The payoff: you do not maintain one explanation for README readers and a second for API docs readers.

## 3. Docs-site synchronization

The mdBook pages consume shared providers too. For example, `mdtInlineBlocksGuide` (in [`.templates/overview.t.md`](https://github.com/ifiokjr/mdt/blob/main/.templates/overview.t.md)) appears in both:

- [Template Syntax](../reference/template-syntax.md)
- [Inline Blocks](../advanced/inline-blocks.md)

The explanation of inline blocks stays consistent across a reference page and a guide page.

## Why this matters

Without mdt, these copies drift in predictable ways:

- the README gets the newest wording
- the source-doc comment keeps an older explanation
- the docs site uses slightly different examples
- command lists diverge across pages

With mdt, one provider edit refreshes every consumer, and CI proves it:

```sh
mdt update
mdt check
```

## The pitch

Describe mdt to a teammate like this:

> We keep a few pieces of documentation repeated across our README, crate docs, and docs site. mdt lets us define those pieces once, reuse them everywhere, and verify in CI that they never drift apart.

If that description matches your project, the tool is worth trying. The [Migration Walkthrough](./migration-walkthrough.md) shows how to start.
