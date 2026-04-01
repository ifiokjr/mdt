# Proof of Value

If you want to know whether `mdt` is solving a real problem, this repository is the best example.

The project already uses source blocks from `template.t.md` to keep repeated content synchronized across multiple surfaces:

- root and crate READMEs
- crate-level Rust docs
- mdBook pages

That is the core value proposition in one repo: write shared content once, then fan it out wherever people actually read it.

## 1. README synchronization

The source block `mdtCliUsage` lives in [`template.t.md`](../../../template.t.md).

It is consumed in multiple README-style surfaces:

- [`readme.md`](../../../readme.md)
- [`mdt_cli/readme.md`](../../../mdt_cli/readme.md)

That means the command list and diagnostics workflow stay aligned without copying edits by hand.

## 2. Source-doc synchronization

The source block `mdtLspOverview` also lives in [`template.t.md`](../../../template.t.md), but it fans out into both markdown and Rust source docs:

- [`mdt_lsp/readme.md`](../../../mdt_lsp/readme.md)
- [`mdt_lsp/src/lib.rs`](../../../mdt_lsp/src/lib.rs)

The source file uses a transformer chain so markdown content becomes Rust crate documentation comments:

```rust
//! <!-- {=mdtLspOverview|trim|linePrefix:"//! ":true} -->
//! <!-- {/mdtLspOverview} -->
```

The same pattern is used for:

- [`mdt_core/src/lib.rs`](../../../mdt_core/src/lib.rs)
- [`mdt_mcp/src/lib.rs`](../../../mdt_mcp/src/lib.rs)
- [`mdt_core/src/parser.rs`](../../../mdt_core/src/parser.rs)

This is the practical payoff: you do not maintain one explanation for README readers and a second explanation for API docs readers.

## 3. Docs-site synchronization

The mdBook docs also consume shared source blocks.

For example, `mdtInlineBlocksGuide` is reused in more than one docs page:

- [`docs/src/reference/template-syntax.md`](../reference/template-syntax.md)
- [`docs/src/advanced/inline-blocks.md`](../advanced/inline-blocks.md)

This keeps the conceptual explanation of inline blocks consistent across both a reference page and a guide page.

## Why this matters

Without `mdt`, these edits drift in predictable ways:

- the README gets the newest wording
- the source-doc comment keeps an older explanation
- the docs site uses slightly different examples
- command lists diverge across pages

With `mdt`, one source update can refresh all of those targets in one run:

```sh
mdt update
mdt check
```

## What to look at in this repo

If you are evaluating adoption, inspect these files together:

### Shared sources

- [`template.t.md`](../../../template.t.md)

### README targets

- [`readme.md`](../../../readme.md)
- [`mdt_cli/readme.md`](../../../mdt_cli/readme.md)
- [`mdt_core/readme.md`](../../../mdt_core/readme.md)
- [`mdt_lsp/readme.md`](../../../mdt_lsp/readme.md)
- [`mdt_mcp/readme.md`](../../../mdt_mcp/readme.md)

### Source-doc consumers

- [`mdt_core/src/lib.rs`](../../../mdt_core/src/lib.rs)
- [`mdt_core/src/parser.rs`](../../../mdt_core/src/parser.rs)
- [`mdt_lsp/src/lib.rs`](../../../mdt_lsp/src/lib.rs)
- [`mdt_mcp/src/lib.rs`](../../../mdt_mcp/src/lib.rs)

### Docs-site targets

- [`docs/src/reference/template-syntax.md`](../reference/template-syntax.md)
- [`docs/src/advanced/inline-blocks.md`](../advanced/inline-blocks.md)

## The shortest convincing story

A good way to describe `mdt` to a teammate is:

> We keep a few pieces of documentation repeated across our README, crate docs, and docs site. `mdt` lets us define those pieces once, reuse them everywhere, and verify in CI that they never drift apart.

If that story matches your project, the tool is probably worth trying.
