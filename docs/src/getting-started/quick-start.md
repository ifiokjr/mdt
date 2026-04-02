# Quick Start

This walkthrough creates a small project that uses mdt to keep a README section and a Rust doc comment in sync from one source.

## 1. Initialize a project

Create a new directory and generate the starter files:

```sh
mkdir my-project && cd my-project
mdt init
```

This creates:

- `.templates/template.t.md` — your starter provider file
- `mdt.toml` — a starter config with commented examples

The starter template contains:

```markdown
<!-- {@greeting} -->

Hello from mdt! This is a source block.

<!-- {/greeting} -->
```

## 2. Add a README target

Create a `readme.md` that references the source:

```markdown
# My Project

Welcome to my project.

<!-- {=greeting} -->

This will be replaced by mdt.

<!-- {/greeting} -->
```

The `{=greeting}` tag marks this as a **target** of the `greeting` provider.

## 3. Add a source-doc consumer

Create `src/lib.rs` with a doc comment consumer that reuses the same source:

```rust
//! <!-- {=greeting|trim|linePrefix:"//! "} -->
//!
//! This will be replaced by mdt.
//!
//! <!-- {/greeting} -->

pub fn hello() {}
```

The `linePrefix:"//! "` transformer adapts the source content so it becomes valid Rust doc comments.

> Not using Rust? The same pattern works in other source files too — use a comment style and transformers that match your language.

## 4. Update

Run the update command:

```sh
mdt update
```

Output:

```
Updated 2 block(s) in 2 file(s).
```

Now both files are synchronized from the same source.

`readme.md` contains:

```markdown
# My Project

Welcome to my project.

<!-- {=greeting} -->

Hello from mdt! This is a source block.

<!-- {/greeting} -->
```

And `src/lib.rs` contains:

```rust
//! <!-- {=greeting|trim|linePrefix:"//! "} -->
//!
//! Hello from mdt! This is a source block.
//!
//! <!-- {/greeting} -->

pub fn hello() {}
```

## 5. Check for staleness

Edit the source in `.templates/template.t.md`:

```markdown
<!-- {@greeting} -->

Hello from mdt! This content has been updated.

<!-- {/greeting} -->
```

Now run the check command:

```sh
mdt check
```

Output:

```
Check failed.
  render errors: 0
  stale targets: 2

Stale targets:
  block `greeting` at readme.md:5:1
  block `greeting` at src/lib.rs:1:5

2 target block(s) are out of date. Run `mdt update` to fix.
```

The check command exits with a non-zero status code when blocks are stale, making it useful in CI pipelines.

## 6. See what changed

Use the `--diff` flag to see exactly what's different:

```sh
mdt check --diff
```

This shows a colorized unified diff between the current target content and what the source would produce.

## 7. List all blocks

See all sources and targets in the project:

```sh
mdt list
```

Output:

```
Sources:
  @greeting .templates/template.t.md (2 target(s))

Targets:
  =greeting readme.md [linked]
  =greeting src/lib.rs |trim|linePrefix [linked]

1 source(s), 2 target(s)
```

## Next steps

- Read [Proof of Value](./proof-of-value.md) to see how this repository uses mdt across READMEs, Rust source docs, and mdBook pages
- Follow the [Migration Walkthrough](./migration-walkthrough.md) to convert repeated docs into a source-plus-consumer workflow
- Learn about [sources and targets](../concepts/providers-and-consumers.md) in depth
- Add [data interpolation](../guide/data-interpolation.md) to pull values from project files
- Use [transformers](../guide/transformers.md) to adapt content for different contexts
- Set up [CI integration](../guide/ci-integration.md) to catch stale docs automatically
