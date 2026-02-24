# Transformers

Transformers modify provider content before it's injected into a consumer. They're specified as pipe-delimited filters on the consumer tag, letting each consumer adapt the same content for its specific context.

## Syntax

Transformers appear after the block name, separated by `|`:

```markdown
<!-- {=blockName|trim|indent:"  "} -->
<!-- {/blockName} -->
```

Multiple transformers are applied left to right. Each receives the output of the previous one.

### Arguments

Some transformers take arguments, specified after a `:` delimiter:

```markdown
<!-- {=block|indent:">>> "} -->
<!-- {=block|codeBlock:"typescript"} -->
<!-- {=block|replace:"old":"new"} -->
```

String arguments are quoted. Numeric arguments are unquoted:

```markdown
<!-- {=block|indent:4} -->
```

## Available transformers

### `trim`

Removes whitespace from both ends of the content.

```markdown
<!-- {=block|trim} -->
```

Before: `\n  Hello world!  \n` After: `Hello world!`

### `trimStart`

Removes whitespace from the start of the content.

```markdown
<!-- {=block|trimStart} -->
```

Aliases: `trim_start`

### `trimEnd`

Removes whitespace from the end of the content.

```markdown
<!-- {=block|trimEnd} -->
```

Aliases: `trim_end`

### `indent`

Prepends a string to each non-empty line. Empty lines are preserved as-is by default.

```markdown
<!-- {=block|indent:"  "} -->
```

Before:

```
line one
line two

line four
```

After:

```
  line one
  line two

  line four
```

To include empty lines (indent them too), pass `true` as a second argument:

```markdown
<!-- {=block|indent:"  ":true} -->
```

With `true`, every line gets the indent — including empty lines. Without it (default), empty lines stay completely empty.

### `prefix`

Prepends a string to the entire content (not per-line).

```markdown
<!-- {=block|prefix:"\n"} -->
```

Before: `Hello` After: `\nHello`

### `suffix`

Appends a string to the entire content.

```markdown
<!-- {=block|suffix:"\n"} -->
```

Before: `Hello` After: `Hello\n`

### `linePrefix`

Prepends a string to each non-empty line. Similar to `indent` but with a clearer name for the intent.

```markdown
<!-- {=block|linePrefix:"// "} -->
```

Before:

```
line one
line two
```

After:

```
// line one
// line two
```

To also prefix empty lines, pass `true` as a second argument. This is essential for code comment blocks where every line needs a comment marker:

```markdown
<!-- {=block|linePrefix:"//! ":true} -->
```

Before:

```
A fast HTTP client.

Supports async and blocking modes.
```

After:

```
//! A fast HTTP client.
//!
//! Supports async and blocking modes.
```

Note: when `true` is set, the prefix is applied to empty lines too, so `"//! "` on an empty line produces `//!` (with trailing space). If you need a shorter prefix on empty lines (e.g., `//!` without the space), use `linePrefix:"//! "` (without `true`) and then handle the empty lines separately, or use `replace` to clean up trailing spaces.

Aliases: `line_prefix`

### `lineSuffix`

Appends a string to each non-empty line. Empty lines are left empty by default.

```markdown
<!-- {=block|lineSuffix:" \\"} -->
```

Before:

```
line one
line two
```

After:

```
line one \
line two \
```

To also suffix empty lines, pass `true` as a second argument:

```markdown
<!-- {=block|lineSuffix:";":true} -->
```

Aliases: `line_suffix`

### `wrap`

Wraps the entire content with a string on both sides.

```markdown
<!-- {=block|wrap:"**"} -->
```

Before: `important text` After: `**important text**`

### `code`

Wraps the content in inline code backticks.

```markdown
<!-- {=block|code} -->
```

Before: `my-lib` After: `` `my-lib` ``

### `codeBlock`

Wraps the content in a fenced code block. Optionally specify a language.

```markdown
<!-- {=block|codeBlock:"typescript"} -->
```

Before: `const x = 1;` After:

````
```typescript
const x = 1;
```
````

Without a language argument:

```markdown
<!-- {=block|codeBlock} -->
```

### `replace`

Replaces all occurrences of a search string with a replacement. Takes exactly two arguments.

```markdown
<!-- {=block|replace:"foo":"bar"} -->
```

Before: `foo is great, foo forever` After: `bar is great, bar forever`

## Chaining transformers

Transformers compose left to right. This is powerful for adapting content to different contexts.

### Example: Rust doc comments

Provider content as plain text, transformed into `///` doc comments. Use `true` to ensure empty lines also get the comment prefix:

```markdown
<!-- {=docs|trim|linePrefix:"/// ":true} -->
<!-- {/docs} -->
```

If the provider contains:

```
A fast HTTP client.

Supports async and blocking modes.
```

The consumer receives:

```
/// A fast HTTP client.
///
/// Supports async and blocking modes.
```

Without `true`, the empty line would be left completely blank, which breaks the doc comment block in Rust.

### Example: JSDoc comments

```markdown
<!-- {=docs|trim|linePrefix:" * ":true} -->
<!-- {/docs} -->
```

Each line (including empty lines) gets the `*` prefix, producing valid JSDoc content.

### Example: Code block with trimming

```markdown
<!-- {=example|trim|codeBlock:"rust"} -->
<!-- {/example} -->
```

Trims the whitespace first, then wraps in a fenced code block.

## Naming conventions

All transformers support both camelCase and snake_case names:

| camelCase    | snake_case    |
| ------------ | ------------- |
| `trimStart`  | `trim_start`  |
| `trimEnd`    | `trim_end`    |
| `codeBlock`  | `code_block`  |
| `linePrefix` | `line_prefix` |
| `lineSuffix` | `line_suffix` |
