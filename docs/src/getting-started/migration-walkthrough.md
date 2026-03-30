# Migration Walkthrough

This walkthrough shows how to adopt `mdt` in a project that already has documentation drift.

The example is intentionally realistic: the same installation instructions appear in a README, a Rust doc comment, and a docs page.

## Before: three copies to maintain

Imagine these three files already exist.

### `readme.md`

```markdown
## Installation

npm install my-lib
```

### `src/lib.rs`

```rust
//! ## Installation
//!
//! npm install my-lib
```

### `docs/src/getting-started.md`

```markdown
## Installation

npm install my-lib
```

At first this seems harmless. Then the command changes to `npm install my-lib@latest`, or the project switches to `pnpm`, or you want to add a second setup note.

Now you have three edits to make, and one of them eventually gets missed.

## After: one provider, three consumers

### 1. Initialize `mdt`

```sh
mdt init
```

This creates a starter template file at `.templates/template.t.md`.

### 2. Define one provider

Add a provider block to `.templates/template.t.md`:

```markdown
<!-- {@install} -->

## Installation

npm install my-lib@latest

<!-- {/install} -->
```

### 3. Replace the README copy with a consumer

```markdown
<!-- {=install} -->

Old copied content

<!-- {/install} -->
```

### 4. Replace the docs-page copy with a consumer

```markdown
<!-- {=install} -->

Old copied content

<!-- {/install} -->
```

### 5. Replace the Rust doc comment with a transformed consumer

```rust
//! <!-- {=install|trim|linePrefix:"//! ":true} -->
//! Old copied content
//! <!-- {/install} -->
```

If your project uses source-file consumers heavily, add padding settings in `mdt.toml` so formatters do not collapse content awkwardly:

```toml
[padding]
before = 0
after = 0
```

### 6. Sync everything

```sh
mdt update
```

After the update, all three places render from the same provider.

## What changed structurally

### Before

- each surface owned its own copy
- wording changes required repeated manual edits
- CI could not reliably detect drift

### After

- the provider in `.templates/template.t.md` becomes the source of truth
- each surface keeps only a consumer tag
- `mdt check` can fail CI when a consumer is stale

## The day-two workflow

Once the migration is done, the maintenance loop is simple:

1. edit the provider block
2. run `mdt update`
3. run `mdt check`
4. commit the synchronized result

That is the real adoption win: not just fewer edits, but a repeatable workflow that prevents drift from coming back.

## A small migration strategy that works well

Do not try to template your entire docs set in one pass.

Start with content that is:

- repeated in 2 or more places
- easy to recognize when it drifts
- expensive or embarrassing when it diverges

Good first candidates:

- installation instructions
- support policy / compatibility notes
- API overview paragraphs
- badge/link sections
- CLI usage summaries

## How to know the migration paid off

A migration is usually worth it when one of these becomes true:

- you can point to a provider that replaced three or more manual copies
- CI now catches stale docs that previously slipped through
- README, source docs, and docs pages no longer need separate wording updates

If you want to see this pattern in a real codebase, inspect the repo-backed examples in [Proof of Value](./proof-of-value.md).
