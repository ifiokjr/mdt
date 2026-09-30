# Transformers

Transformers let each consumer adapt the same provider content to its context: trim it, fence it, turn it into doc comments, or drop it when a flag is off. This page shows common tasks. The [Transformer Reference](../reference/transformers.md) lists every transformer and its exact behavior.

## Syntax

Transformers follow the block name, separated by `|`, and run left to right. Arguments follow a `:`.

```markdown
<!-- {=install|trim|codeBlock:"sh"} -->
<!-- {/install} -->
```

Quote text arguments. Numbers are converted to text, so `indent:4` prepends the character `4`. Write `indent:"    "` for four spaces.

## Start with `trim`

Providers are usually written with a blank line after the opening tag and before the closing tag, and that whitespace is part of the content. Start a chain with `trim` whenever the consumer should control the layout, and always before `codeBlock`, `code`, `wrap`, or `linePrefix`, or the blank lines end up inside the fence, the span, or the comment.

## Show content as code

Given this provider:

```markdown
<!-- {@install} -->

npm install my-lib

<!-- {/install} -->
```

`codeBlock` fences it:

````text
<!-- {=install|trim|codeBlock:"sh"} -->
```sh
npm install my-lib
```
<!-- {/install} -->
````

For an inline code span, use `code`: `{=install|trim|code}` writes `` `npm install my-lib` `` between the tags.

`codeBlock` picks a fence longer than any backtick run in the content, so providers that contain fences stay valid. It works with the default padding; no `[padding]` section is needed.

## Nest content in a list item

`indent` prepends text to every non-empty line. Indent the tags to match; the closing tag keeps its indentation.

````text
1. Install the package:

   <!-- {=install|trim|codeBlock:"sh"|indent:"   "} -->
   ```sh
   npm install my-lib
   ```
   <!-- {/install} -->

2. Import it.
````

## Write doc comments

In source files, every content line needs the comment marker, including blank lines. Use `linePrefix` with `true`:

```rust
//! <!-- {=clientDocs|trim|linePrefix:"//! ":true} -->
//! A fast HTTP client.
//!
//! Supports async and blocking modes.
//! <!-- {/clientDocs} -->
```

On empty lines, `linePrefix` trims the prefix's trailing space, so the blank line is `//!` with no trailing space. Without `true`, the blank line has no marker: rustdoc merges the paragraphs, clippy warns `empty line after doc comment`, and in Go the blank line splits the comment group so only the last paragraph stays attached.

For JSDoc, use `linePrefix:" * ":true` rather than `indent:" * ":true`. `indent` keeps the trailing space on empty lines, formatters strip it, and the consumer turns stale on every run.

For comments on nested items, put the indentation inside the prefix, for example `linePrefix:"    /// ":true` in an `impl` block. Otherwise a formatter re-indents the lines and `mdt check` reports them stale.

[Source File Support](source-files.md) has a working example for each language.

## Include content conditionally

`if` keeps the content only when a `[data]` value is truthy. It takes a dot-separated path, not an expression.

```toml
[data]
pkg = "package.json"
```

```text
<!-- {=betaNotice|trim|prefix:"> "|if:"pkg.flags.beta"} -->
> The streaming API is in beta.
<!-- {/betaNotice} -->
```

When `pkg.flags.beta` is missing, `false`, `null`, `""`, or `0`, the consumer is empty. Put `if` last: transformers after it still run on the empty content, so `if:"..."|prefix:"> "` would leave a stray `>`.

## Rewrite text

`replace` swaps every occurrence of one string for another. Use it to adapt wording or to escape text that would break the surrounding syntax:

```markdown
<!-- {=intro|trim|replace:"this crate":"this package"} -->
<!-- {/intro} -->
```

An empty replacement deletes the match. See [Source File Support](source-files.md#block-comments) for escaping `*/` inside block comments.
