# Sources and Targets

mdt's template system has two roles: **sources** define content, and **targets** receive it.

## Sources

A source block defines a named piece of content. Providers live in **template files** (`*.t.md`).

```
<!-- {@installGuide} -->

Install the package:

  npm install my-lib

<!-- {/installGuide} -->
```

The `@` sigil marks this as a source. The name `installGuide` is how consumers reference it.

### Rules for sources

- Providers can **only** appear in `*.t.md` files. A `{@name}` tag in `readme.md` is ignored.
- Each source name must be unique across the entire project. Two template files defining `{@installGuide}` produces an error.
- The content between the opening and closing tags is the source's content — including the surrounding whitespace.

## Targets

A target block marks a location where source content should be injected. Consumers can appear in any scanned file.

```
<!-- {=installGuide} -->

Old content here (will be replaced).

<!-- {/installGuide} -->
```

The `=` sigil marks this as a target. The name `installGuide` tells mdt which provider to use.

### Rules for targets

- Consumers can appear in any markdown file or source code file.
- Multiple targets can reference the same source. Each gets the same content.
- If a target references a non-existent source, mdt warns but doesn't fail.
- Consumers can include [transformers](../guide/transformers.md) to modify the content for their specific context.

## Close tags

Both sources and targets share the same close tag syntax:

```
<!-- {/blockName} -->
```

The `/` sigil closes the block. The name must match the opening tag.

## How content flows

```
.templates/*.t.md         readme.md                     src/lib.rs
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

The same source content feeds multiple targets. Each consumer can apply its own transformers to adapt the content for its context.

## A complete example

**`.templates/*.t.md`** — grouped sources of truth:

```
<!-- {@projectDescription} -->

A fast, type-safe HTTP client for Rust.

<!-- {/projectDescription} -->

<!-- {@usage} -->

    let response = client.get("https://example.com").send().await?;

<!-- {/usage} -->
```

**`readme.md`** — targets reference sources by name:

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
