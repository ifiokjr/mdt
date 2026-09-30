# Block Arguments

Block arguments make a provider reusable with different values. Instead of one provider per variation, define one provider with parameters and pass values from each consumer.

Arguments work with or without a `[data]` section.

## Syntax

### Provider: declare parameters

Add `:"param_name"` after the block name:

```markdown
<!-- {@badges:"crate_name"} -->

[![crates.io](https://img.shields.io/crates/v/{{ crate_name }})](https://crates.io/crates/{{ crate_name }}) [![docs.rs](https://docs.rs/{{ crate_name }}/badge.svg)](https://docs.rs/{{ crate_name }}/)

<!-- {/badges} -->
```

Each parameter becomes a template variable in the provider content: `{{ crate_name }}`.

### Consumer: pass values

Consumers pass string values in the same positions:

```markdown
<!-- {=badges:"mdt_core"} -->
<!-- {/badges} -->
```

When mdt renders this consumer, `{{ crate_name }}` becomes `mdt_core`.

## Multiple arguments

```markdown
<!-- {@installCmd:"pkg_manager":"pkg_name":"version"} -->

{{ pkg_manager }} add {{ pkg_name }}@{{ version }}

<!-- {/installCmd} -->
```

Consumers pass values in the same order:

```markdown
<!-- {=installCmd:"npm":"my-lib":"1.2.3"} -->
<!-- {/installCmd} -->

<!-- {=installCmd:"yarn":"my-lib":"2.0.0"} -->
<!-- {/installCmd} -->
```

After `mdt update`, the first consumer contains `npm add my-lib@1.2.3` and the second `yarn add my-lib@2.0.0`.

## Combining arguments with other features

### Transformers

Transformers come after the arguments, separated by `|`:

```markdown
<!-- {=badges:"mdt_core"|trim} -->
<!-- {/badges} -->
```

### Data interpolation

Arguments and `[data]` variables work together in the same provider:

```toml
# mdt.toml
[data]
cargo = "Cargo.toml"
```

```markdown
<!-- {@crateInfo:"crate_name"} -->

**{{ crate_name }}** v{{ cargo.workspace.package.version }}

<!-- {/crateInfo} -->
```

`{{ crate_name }}` comes from the consumer's argument, and `{{ cargo.workspace.package.version }}` from `Cargo.toml`.

### Quotes

Single and double quotes both work. Double-quoted values decode escapes such as `\t`; single-quoted values are taken literally:

```markdown
<!-- {=badges:'mdt_core'} -->
<!-- {/badges} -->
```

## Use cases

### Badge links for multiple crates

Each crate needs the same badge markup with a different name:

```text
<!-- {@badgeLinks:"crateName"} -->

[crate-image]: https://img.shields.io/crates/v/{{ crateName }}.svg
[crate-link]: https://crates.io/crates/{{ crateName }}
[docs-image]: https://docs.rs/{{ crateName }}/badge.svg
[docs-link]: https://docs.rs/{{ crateName }}/

<!-- {/badgeLinks} -->
```

Each crate's README passes its own name:

```markdown
<!-- {=badgeLinks:"mdt_core"} -->
<!-- {/badgeLinks} -->
```

### Versioned install snippets

The crate name comes from an argument and the version from a data file:

```markdown
<!-- {@addDep:"dep_name"} -->

Install with cargo: `cargo add {{ dep_name }}`

Or add to Cargo.toml: `{{ dep_name }} = "{{ cargo.workspace.package.version }}"`

<!-- {/addDep} -->
```

## Argument count mismatch

A consumer must pass exactly as many arguments as the provider declares. Passing too many, too few, or any arguments to a provider without parameters is a render error:

```text
Render errors:
  block `badges` at readme.md:6:1: argument count mismatch: provider `badges` declares 1 parameter(s), but consumer passes 2
  block `badges` at readme.md:9:1: argument count mismatch: provider `badges` declares 1 parameter(s), but consumer passes 0
  block `simpleBlock` at readme.md:12:1: argument count mismatch: provider `simpleBlock` declares 0 parameter(s), but consumer passes 1
```

How each command reports it:

- **`mdt check`** lists the consumer under `Render errors:` and exits 1. With `--format json` it is an entry in `errors` (`file`, `block`, `line`, `column`, `message`); with `--format github` it is an `::error` annotation starting `Template render failed for block`.
- **`mdt update`** skips that consumer, updates every other one, and exits 1:

  ```text
  error: block `badges` at readme.md:6:1 was not updated: argument count mismatch: provider `badges` declares 1 parameter(s), but consumer passes 2
  Updated 1 block(s) in 1 file(s).
  ```

- **`mdt doctor`** fails its `Consumer Sync` check and exits 1.
- **`mdt list`** does not check arguments: it shows the consumer as `[linked]` and exits 0.

Fix it by matching the consumer's `:"value"` segments to the provider's parameters.
