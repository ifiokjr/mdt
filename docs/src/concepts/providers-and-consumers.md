# Providers and Consumers

mdt's template system has two roles: **providers** define content, and **consumers** receive it.

## Providers

A provider block defines a named piece of content. Providers live in **template files** (`*.t.md`).

```
<!-- {@installGuide} -->

Install the package:

  npm install my-lib

<!-- {/installGuide} -->
```

The `@` sigil marks this as a provider. The name `installGuide` is how consumers reference it.

### Rules for providers

- Providers can **only** appear in `*.t.md` files. A `{@name}` tag in `readme.md` is ignored.
- Each provider name must be unique across the entire project. Two template files defining `{@installGuide}` produces an error.
- The content between the opening and closing tags is the provider's content — including the surrounding whitespace.

## Consumers

A consumer block marks a location where provider content should be injected. Consumers can appear in any scanned file.

```
<!-- {=installGuide} -->

Old content here (will be replaced).

<!-- {/installGuide} -->
```

The `=` sigil marks this as a consumer. The name `installGuide` tells mdt which provider to use.

### Rules for consumers

- Consumers can appear in any markdown file or source code file.
- Multiple consumers can reference the same provider. Each gets the same content.
- If a consumer references a non-existent provider, mdt warns but doesn't fail.
- Consumers can include [transformers](../guide/transformers.md) to modify the content for their specific context.

## Close tags

Both providers and consumers share the same close tag syntax:

```
<!-- {/blockName} -->
```

The `/` sigil closes the block. The name must match the opening tag.

## How content flows

```
template.t.md                    readme.md                     src/lib.rs
┌─────────────────┐             ┌──────────────────┐          ┌──────────────────┐
│ <!-- {@docs} -->│             │ <!-- {=docs} --> │          │ // <!-- {=docs|  │
│                 │──────┬─────→│                  │          │ //  trim|indent: │
│ API reference.  │      │      │ API reference.   │          │ //  "/// "} -->  │
│                 │      │      │                  │          │ /// API reference│
│ <!-- {/docs} -->│      └─────→│ <!-- {/docs} --> │          │ // <!-- {/docs}  │
└─────────────────┘             └──────────────────┘          │ //  -->          │
                                                              └──────────────────┘
     Provider                     Consumer (plain)              Consumer (with
                                                                transformers)
```

The same provider content feeds multiple consumers. Each consumer can apply its own transformers to adapt the content for its context.

## A complete example

**`template.t.md`** — the single source of truth:

```
<!-- {@projectDescription} -->

A fast, type-safe HTTP client for Rust.

<!-- {/projectDescription} -->

<!-- {@usage} -->

    let response = client.get("https://example.com").send().await?;

<!-- {/usage} -->
```

**`readme.md`** — consumers reference providers by name:

```
# my-http-client

<!-- {=projectDescription} -->
<!-- {/projectDescription} -->

## Quick start

<!-- {=usage} -->
<!-- {/usage} -->
```

**`my-http-client/src/lib.rs`** — even works in source comments:

```rust
//! <!-- {=projectDescription|trim} -->
//! <!-- {/projectDescription} -->
```

After `mdt update`, all three files contain the same project description and usage example, each adapted for its context.
