# Migration Walkthrough

This walkthrough adopts mdt in a project that already has documentation drift: the same installation instructions appear in a README, a Rust doc comment, and a docs page. Every step below can be run as written.

## Before: three copies to maintain

`readme.md`:

```markdown
# my-lib

## Installation

npm install my-lib
```

`src/lib.rs`:

```rust
//! ## Installation
//!
//! npm install my-lib
```

`docs/src/getting-started.md`:

```markdown
# Getting started

## Installation

npm install my-lib
```

That seems harmless until the command changes to `npm install my-lib@latest`, the project switches to `pnpm`, or you add a second setup note. Now you have three edits to make, and one of them eventually gets missed.

## After: one provider, three consumers

### 1. Initialize mdt

From the project root:

```sh
mdt init
```

Output:

```text
Created mdt.toml
Created .templates/template.t.md with a sample `greeting` provider
Left the existing readme.md unchanged
Added the `.mdt/` cache directory to .gitignore

Next steps:
  1. Add a consumer to readme.md: <!-- {=greeting} --> <!-- {/greeting} -->
  2. Run `mdt update` to fill it in (until then `mdt check` warns that `greeting` has no consumers)
  3. Replace the sample in .templates/template.t.md with your own providers
```

The `.gitignore` line only appears in a git repository. Your existing README is never modified.

### 2. Replace the sample provider

Replace the whole contents of `.templates/template.t.md` with your own provider:

```markdown
<!-- {@install} -->

## Installation

npm install my-lib@latest

<!-- {/install} -->
```

Delete the sample `greeting` provider rather than keeping it next to yours. A provider with no consumers still works, but every `mdt check` and `mdt update` prints an `mdt::unused_provider` warning for it.

### 3. Wrap each copy in consumer tags

Put consumer tags around each existing copy. You do not need to delete the old text: `mdt update` replaces everything between the tags.

`readme.md`:

```markdown
# my-lib

<!-- {=install} -->

## Installation

npm install my-lib

<!-- {/install} -->
```

`docs/src/getting-started.md`:

```markdown
# Getting started

<!-- {=install} -->

## Installation

npm install my-lib

<!-- {/install} -->
```

`src/lib.rs` needs transformers so the markdown becomes Rust doc comments:

```rust
//! <!-- {=install|trim|linePrefix:"//! ":true} -->
//! ## Installation
//!
//! npm install my-lib
//! <!-- {/install} -->
```

### 4. See what is out of date

```sh
mdt check
```

Output:

```text
Check failed.
  stale consumers: 3

Stale consumers:
  block `install` at docs/src/getting-started.md:3:1
  block `install` at readme.md:3:1
  block `install` at src/lib.rs:1:5

3 consumer block(s) are out of date. Run `mdt update`.
```

Add `--diff` to see the exact changes.

### 5. Sync everything

```sh
mdt update
```

Output:

```text
Updated 3 block(s) in 3 file(s).
```

All three files now render from the one provider. `src/lib.rs` becomes:

```rust
//! <!-- {=install|trim|linePrefix:"//! ":true} -->
//! ## Installation
//!
//! npm install my-lib@latest
//! <!-- {/install} -->
```

Running `mdt check` again prints `Check passed: all consumer blocks are up to date.`

## What changed structurally

Before:

- each surface owned its own copy
- wording changes required repeated manual edits
- CI could not detect drift

After:

- the provider in `.templates/template.t.md` is the single source of truth
- each surface keeps only a consumer tag pair
- `mdt check` fails CI when a consumer is stale

## The day-two workflow

1. Edit the provider.
2. Run `mdt update`.
3. Run `mdt check`.
4. Commit the synchronized result.

That is the real adoption win: a repeatable workflow that keeps drift from coming back, not a pile of one-off edits.

## A migration strategy that works

Do not template your entire docs set in one pass. Start with content that is:

- repeated in two or more places
- easy to recognize when it drifts
- expensive or embarrassing when it diverges

Good first candidates:

- installation instructions
- support policy and compatibility notes
- API overview paragraphs
- badge and link sections
- CLI usage summaries

## How to know the migration paid off

- one provider replaced three or more manual copies
- CI catches stale docs that previously slipped through
- README, source docs, and docs pages no longer need separate wording updates

To see this pattern in a real codebase, read [Proof of Value](./proof-of-value.md).
