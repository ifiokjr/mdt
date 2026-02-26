# Block Arguments

Block arguments let you create parameterized provider blocks. Instead of defining a separate provider for each variation, you define one provider with parameters and pass different values from each consumer.

## Syntax

### Provider: declare parameters

Add `:"param_name"` after the block name to declare parameters:

```
<!-- {@badges:"crate_name"} -->

[![crates.io](https://img.shields.io/crates/v/{{ crate_name }})](https://crates.io/crates/{{ crate_name }})
[![docs.rs](https://docs.rs/{{ crate_name }}/badge.svg)](https://docs.rs/{{ crate_name }}/)

<!-- {/badges} -->
```

The parameter name `crate_name` becomes a template variable available in the provider content via `{{ crate_name }}`.

### Consumer: pass values

Consumers pass string values in the same position:

```
<!-- {=badges:"mdt_core"} -->
<!-- {/badges} -->
```

When mdt renders this consumer, `{{ crate_name }}` in the provider content is replaced with `mdt_core`.

## Multiple arguments

Providers can declare multiple parameters:

```
<!-- {@installCmd:"pkg_manager":"pkg_name":"version"} -->

{{ pkg_manager }} install {{ pkg_name }}@{{ version }}

<!-- {/installCmd} -->
```

Consumers pass values in the same order:

```
<!-- {=installCmd:"npm":"my-lib":"1.2.3"} -->
<!-- {/installCmd} -->

<!-- {=installCmd:"yarn":"my-lib":"2.0.0"} -->
<!-- {/installCmd} -->
```

After `mdt update`, the first consumer contains `npm install my-lib@1.2.3` and the second contains `yarn install my-lib@2.0.0`.

## Combining arguments with other features

### With transformers

Arguments and transformers work together. Transformers come after the arguments, separated by `|`:

```
<!-- {=badges:"mdt_core"|trim} -->
<!-- {/badges} -->
```

### With data interpolation

Block arguments and data interpolation variables coexist in the same provider content. Arguments are resolved alongside the data context:

```toml
# mdt.toml
[data]
cargo = "Cargo.toml"
```

```
<!-- {@crateInfo:"crate_name"} -->

**{{ crate_name }}** v{{ cargo.workspace.package.version }}

<!-- {/crateInfo} -->
```

Here `{{ crate_name }}` comes from the consumer's argument, while `{{ cargo.workspace.package.version }}` comes from the data file.

### With single quotes

Both single and double quotes work for argument values:

```
<!-- {@tmpl:'param'} -->
<!-- {=tmpl:'value'} -->
```

## Use cases

### Badge links for multiple crates

A common monorepo pattern where each crate needs the same badge markup with different crate names:

```
<!-- {@badgeLinks:"crateName"} -->

[crate-image]: https://img.shields.io/crates/v/{{ crateName }}.svg
[crate-link]: https://crates.io/crates/{{ crateName }}
[docs-image]: https://docs.rs/{{ crateName }}/badge.svg
[docs-link]: https://docs.rs/{{ crateName }}/

<!-- {/badgeLinks} -->
```

Each crate's README passes its own name:

```
<!-- {=badgeLinks:"mdt_core"} -->
<!-- {/badgeLinks} -->
```

```
<!-- {=badgeLinks:"mdt_cli"} -->
<!-- {/badgeLinks} -->
```

### Versioned install snippets

Generate install instructions that pull the crate name from an argument and the version from a data file:

```
<!-- {@addDep:"dep_name"} -->

Install via cargo: `cargo add {{ dep_name }}`

Or add to Cargo.toml: `{{ dep_name }} = "{{ cargo.workspace.package.version }}"`

<!-- {/addDep} -->
```

### Platform-specific instructions

```
<!-- {@buildCmd:"platform":"toolchain"} -->

To build on {{ platform }}, install {{ toolchain }} first,
then run: {{ toolchain }} build --release

<!-- {/buildCmd} -->
```

## Argument count mismatch

The number of consumer arguments must match the number of provider parameters. If they don't match, mdt reports a render error:

```
error: argument count mismatch: provider `badges` declares 1 parameter(s),
       but consumer passes 2 argument(s)
```

- `mdt check` reports the mismatch as an error.
- `mdt update` skips the mismatched consumer and continues with the rest.

### Zero arguments on consumer

A consumer referencing a parameterized provider without arguments also triggers a mismatch. If the provider declares parameters, every consumer must supply values:

```
<!-- Provider expects 1 argument -->
<!-- {@greeting:"name"} -->
Hello, {{ name }}!
<!-- {/greeting} -->

<!-- This consumer is missing the argument — mdt reports an error -->
<!-- {=greeting} -->
<!-- {/greeting} -->
```

### Zero parameters on provider

If a provider has no parameters, consumers should not pass arguments. Passing arguments to a parameter-less provider is a mismatch:

```
<!-- Provider has no parameters -->
<!-- {@simpleBlock} -->
Static content.
<!-- {/simpleBlock} -->

<!-- This consumer has an unexpected argument — mdt reports an error -->
<!-- {=simpleBlock:"unused"} -->
<!-- {/simpleBlock} -->
```
