# Providers and Consumers

mdt blocks have two roles. A **provider** defines content once; a **consumer** marks where that content is written.

## Providers

A provider is a named block in a template file (`*.t.md`), opened with the `@` sigil:

```markdown
<!-- {@installGuide} -->

Install the package:

    npm install my-lib

<!-- {/installGuide} -->
```

- Providers are read only from `*.t.md` files. A `{@name}` tag anywhere else is ignored and reported as a `mdt::provider_outside_template` warning.
- Names are unique across the project. A second `{@installGuide}` is a `mdt::duplicate_provider` error.
- The content is everything between the tags, including the blank lines after the opening tag and before the closing tag.
- A provider with no consumers is a `mdt::unused_provider` warning.

## Consumers

A consumer is a named block opened with the `=` sigil. `mdt update` replaces everything between its tags with the provider's content.

```markdown
<!-- {=installGuide} -->

Old content here (will be replaced).

<!-- {/installGuide} -->
```

- Consumers work in markdown files and in [source file](../guide/source-files.md) comments.
- Any number of consumers can use the same provider.
- Each consumer can add [transformers](../guide/transformers.md) to adapt the content, such as `|trim|linePrefix:"//! ":true` for Rust doc comments.
- A consumer whose name matches no provider is an orphan. `mdt check` fails on it and suggests a close match; `mdt update` warns and leaves it unchanged.
- A consumer cannot contain another block; `mdt update` would overwrite it (`mdt::nested_block`).

## Closing tags

Providers, consumers, and inline blocks share one closing tag. The name must match the opening tag:

```markdown
<!-- {/installGuide} -->
```

## A complete example

`.templates/docs.t.md` holds the providers:

````markdown
<!-- {@projectDescription} -->

A fast, type-safe HTTP client for Rust.

<!-- {/projectDescription} -->

<!-- {@usage} -->

```rust
let response = client.get("https://example.com").send().await?;
```

<!-- {/usage} -->
````

`readme.md` uses both as they are:

```markdown
# my-http-client

<!-- {=projectDescription} -->
<!-- {/projectDescription} -->

## Quick start

<!-- {=usage} -->
<!-- {/usage} -->
```

`src/lib.rs` turns the description into crate docs:

```rust
//! <!-- {=projectDescription|trim|linePrefix:"//! ":true} -->
//! <!-- {/projectDescription} -->
```

After `mdt update`, `src/lib.rs` contains:

```rust
//! <!-- {=projectDescription|trim|linePrefix:"//! ":true} -->
//! A fast, type-safe HTTP client for Rust.
//! <!-- {/projectDescription} -->
```

and `readme.md` contains the description and the fenced usage example between the tags. Edit a provider, run `mdt update` again, and every consumer follows.
