# Quick Start

This walkthrough creates a small project where one provider keeps a README section and a Rust doc comment in sync.

## 1. Initialize a project

```sh
mkdir my-project && cd my-project
git init
mdt init
```

Output:

```text
Created mdt.toml
Created .templates/template.t.md with a sample `greeting` provider
Created readme.md with a synced `greeting` consumer
Added the `.mdt/` cache directory to .gitignore

Next steps:
  1. Open readme.md to see the synced sample block
  2. Edit .templates/template.t.md, then run `mdt update`
  3. Run `mdt check` in CI to fail builds on stale docs
```

`mdt init` only adds what is missing:

- `mdt.toml`: an annotated config with every option commented out (skipped if a config already exists)
- `.templates/template.t.md`: a sample `greeting` provider (skipped if the project already has providers)
- `readme.md`: a README with the sample consumer already synced (only when the project has no README; an existing README is never modified)
- `.gitignore`: a `.mdt/` entry for mdt's cache (only in a git repository)

After `init`, you can run mdt commands from any subdirectory: like `git` and `cargo`, mdt walks up to the nearest directory containing `mdt.toml` and uses it as the project root. Pass `--path <dir>` to choose the root explicitly.

The provider in `.templates/template.t.md`:

```markdown
<!-- {@greeting} -->

Hello from mdt! This is a provider block.

<!-- {/greeting} -->
```

The consumer in `readme.md`:

```markdown
# My Project

Welcome to my project.

<!-- {=greeting} -->

Hello from mdt! This is a provider block.

<!-- {/greeting} -->
```

`{@greeting}` defines the content once. `{=greeting}` marks a place that receives it.

## 2. Add a source-file consumer

Consumers also work inside code comments. Create `src/lib.rs`:

```sh
mkdir src
```

```rust
//! <!-- {=greeting|trim|linePrefix:"//! ":true} -->
//! <!-- {/greeting} -->

pub fn hello() {}
```

The transformers adapt the provider content for Rust: `trim` removes the surrounding blank lines, and `linePrefix:"//! ":true` puts `//!` in front of every line. The `true` argument also prefixes blank lines (as `//!`), so a multi-paragraph provider stays one unbroken doc comment.

> Not using Rust? Use your language's comment prefix instead, for example `linePrefix:"// ":true` for Go or `linePrefix:"# ":true` for Python. See [Source Files](../guide/source-files.md).

## 3. Update

```sh
mdt update
```

Output:

```text
Updated 1 block(s) in 1 file(s).
```

`src/lib.rs` now contains valid Rust doc comments:

```rust
//! <!-- {=greeting|trim|linePrefix:"//! ":true} -->
//! Hello from mdt! This is a provider block.
//! <!-- {/greeting} -->

pub fn hello() {}
```

`readme.md` was already in sync, so only one block changed.

## 4. Check for staleness

Change the provider in `.templates/template.t.md`:

```markdown
<!-- {@greeting} -->

Hello from mdt! This content has been updated.

<!-- {/greeting} -->
```

Then run:

```sh
mdt check
```

Output:

```text
Check failed.
  stale consumers: 2

Stale consumers:
  block `greeting` at readme.md:5:1
  block `greeting` at src/lib.rs:1:5

2 consumer block(s) are out of date. Run `mdt update`.
```

`mdt check` exits with status 1 when a consumer is stale, which makes it a CI gate. It also fails on consumers whose name matches no provider.

## 5. See what changed

```sh
mdt check --diff
```

This prints a unified diff under each stale consumer, comparing its current content with what the provider would produce.

## 6. Sync again

```sh
mdt update
```

Output:

```text
Updated 2 block(s) in 2 file(s).
```

Running `mdt check` now prints `Check passed: all consumer blocks are up to date.`

## 7. List all blocks

```sh
mdt list
```

Output:

```text
Providers:
  @greeting .templates/template.t.md:1 (2 consumer(s))

Consumers:
  =greeting readme.md:5 [linked]
  =greeting src/lib.rs:1 |trim|linePrefix:"//! ":true [linked]

1 provider(s), 2 consumer(s)
```

## Next steps

- Read [Proof of Value](./proof-of-value.md) to see how this repository uses mdt across READMEs, Rust source docs, and mdBook pages
- Follow the [Migration Walkthrough](./migration-walkthrough.md) to convert repeated docs into providers and consumers
- Learn about [providers and consumers](../concepts/providers-and-consumers.md) in depth
- Add [data interpolation](../guide/data-interpolation.md) to pull values from project files
- Use [transformers](../guide/transformers.md) to adapt content for different contexts
- Set up [CI integration](../guide/ci-integration.md) to catch stale docs automatically
