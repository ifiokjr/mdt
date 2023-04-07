# mdt

> update markdown content anywhere using comments as template tags

<br />

[![Crate][crate-image]][crate-link] [![Docs][docs-image]][docs-link]
[![Status][ci-status-image]][ci-status-link] [![Unlicense][unlicense-image]][unlicense-link]

<br />

## Why?

Often, while developing a project, you will want to automatically update sections of markdown
content in your files. For example:

- The `version` has been updated and should be reflected in multiple files.
- The API has changed and you want to automatically generate documentation that is placed within the
  `readme.md` file.

## Solution

This project allows you to wrap the content you want to keep updated in a markdown comment block
`<!-- ={exampleBlock} --><!-- {/exampleBlock}-->`. A build command is then run which injects values
into the comment tag while preserving the tags. This means that blocks can be updated multiple times
with new data.

Blocks can also be checked during continuous integration to ensure that they are up-to-date.

## Deeper Explanation

Here is a deeper explanation of the `mdt` flow.

### Step 1: Define templates

Define replacement blocks within the markdown content. A replacement block is defined with the
following syntax:

#### Opening tag

```markdown
<!-- ={exampleBlock} -->
```

#### Closing Tag

```markdown
<!-- {/exampleBlock}-->
```

This markdown content can be anywhere that takes markdown content. For example you might want to
keep you `readme.md` up to date with the latest API documentation. You can do this by adding the
following to your `readme.md` file:

```markdown
# my_precious

> does precious things

## API documentation

<!-- ={api} -->

This is automatically replaced with the API documentation.

<!-- {/api}-->
```

You may also want to reuse code examples in multiple places without having to redefine the multiple
times. Below is an example of using the templates in rust files. The same flow can be used for
`TypeScript`, `Dart` and other languages that support markdown in their documentation comments.

````rust
//! This is the API documentation for my_precious
//!
//! # Examples
//!
//! The block below will automatically be replaced when running the `mdt`
//! command.
//!
//! <!-- ={codeExample} -->
//! ```rust
//! use my_precious::be_precious;
//!
//! be_precious();
//! ```
//! <!-- {/codeExample} -->

pub fn be_precious() {
  println!("I am precious");
}
````

### Step 2: Define definition files

The tags in the previous step are pulling in their content from somewhere. In mdt this is from the
`definition` files. A definition file is a markdown file with the following naming convention
`*.t.md`.

These file are comprised of template blocks of content where the blocks used in the previous section
are defined.

#### Defining a template block

````markdown
<!-- @{exampleBlock} -->

This content will be injected into any markdown content which has the following tag

```markdown
<!-- ={exampleBlock} -->

...

<!-- {/exampleBlock} -->`
```

<!-- {/exampleBlock} -->
````

#### Defining a template block with template values

In the following example the `{{example.version}}` is a template value. This value will be replaced
with the value defined in the configuration when running the `mdt` command.

````markdown
<!-- @{codeExample} -->

```ts
import { bePrecious } from "https://deno.land/x/my_precious@{{example.version}}/mod.ts";

bePrecious();
```

<!-- {/codeExample} -->
````

### Step 3: Define the template values

The template values can be defined in the following ways.

- `mdt.json`
- `mdt.yaml`
- `mdt.kdl`
- `mdt.toml`
- `mdt.ts` - recommended (see below)

## Installation

```toml
[dependencies]
mdt = "0.0.0"
```

[crate-image]: https://img.shields.io/crates/v/mdt.svg
[crate-link]: https://crates.io/crates/mdt
[docs-image]: https://docs.rs/mdt/badge.svg
[docs-link]: https://docs.rs/mdt/
[ci-status-image]: https://github.com/ifiokjr/monochange/workflows/ci/badge.svg
[ci-status-link]: https://github.com/ifiokjr/monochange/actions?query=workflow:ci
[unlicense-image]: https://img.shields.io/badge/license-Unlicence-blue.svg
[unlicense-link]: https://opensource.org/license/unlicense
