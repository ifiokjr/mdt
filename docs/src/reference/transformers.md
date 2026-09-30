# Transformer Reference

Transformers adapt provider content for one consumer. They follow the block name, separated by `|`, and run left to right: `{=name|trim|codeBlock:"sh"}`. For task-oriented examples, see the [Transformers guide](../guide/transformers.md).

## Summary

| Transformer  | Arguments                | Effect                                              |
| ------------ | ------------------------ | --------------------------------------------------- |
| `trim`       | none                     | Remove whitespace from both ends                    |
| `trimStart`  | none                     | Remove whitespace from the start                    |
| `trimEnd`    | none                     | Remove whitespace from the end                      |
| `prefix`     | `text?`                  | Prepend text to the whole content                   |
| `suffix`     | `text?`                  | Append text to the whole content                    |
| `wrap`       | `text?`                  | Add text to both ends of the content                |
| `indent`     | `text?`, `includeEmpty?` | Prepend text to each line                           |
| `linePrefix` | `text?`, `includeEmpty?` | Prepend text to each line; trims it on empty lines  |
| `lineSuffix` | `text?`, `includeEmpty?` | Append text to each line; trims it on empty lines   |
| `code`       | none                     | Wrap in an inline code span                         |
| `codeBlock`  | `language?`              | Wrap in a fenced code block                         |
| `replace`    | `search`, `replacement`  | Replace every occurrence of `search`                |
| `if`         | `dataPath`               | Keep the content only when the data value is truthy |

Arguments are separated by `:`. Numbers are converted to text, so `indent:4` prepends the character `4`; write `indent:"    "` for four spaces. See [argument types](template-syntax.md#argument-types) for quoting and escapes.

## Names and aliases

Names are case-sensitive. These snake_case aliases are accepted:

| Name         | Aliases                   |
| ------------ | ------------------------- |
| `trimStart`  | `trim_start`              |
| `trimEnd`    | `trim_end`                |
| `codeBlock`  | `code_block`, `codeblock` |
| `linePrefix` | `line_prefix`             |
| `lineSuffix` | `line_suffix`             |

## `trim`, `trimStart`, `trimEnd`

Remove spaces, tabs, and newlines from both ends, the start, or the end. `trim` turns `\n  hello  \n` into `hello`.

Provider content usually starts and ends with a newline, so most consumers begin with `|trim`.

## `prefix`, `suffix`, `wrap`

Add text once to the whole content, not to each line.

| Transformer   | Input   | Output      |
| ------------- | ------- | ----------- |
| `prefix:"> "` | `hello` | `> hello`   |
| `suffix:"\n"` | `hello` | `hello\n`   |
| `wrap:"**"`   | `hello` | `**hello**` |

With no argument they add nothing.

## `indent`, `linePrefix`, `lineSuffix`

Add text to each line. The optional second argument `true` also applies it to empty lines.

- Without `true`, empty lines stay empty.
- With `true`, `indent` adds the text unchanged. `linePrefix` trims the prefix's trailing whitespace on empty lines, and `lineSuffix` trims the suffix's leading whitespace, so no line ends in stray spaces.
- Lines that contain only whitespace are not empty and always get the text.
- A final trailing newline in the content is not kept.

With the input `A fast HTTP client.`, an empty line, and `Supports async and blocking modes.`:

```text
|linePrefix:"/// "          |linePrefix:"/// ":true
/// A fast HTTP client.     /// A fast HTTP client.
                            ///
/// Supports async ...      /// Supports async ...
```

On empty lines, `linePrefix:" * ":true` drops the trailing space of the prefix, while `indent:" * ":true` keeps it. Prefer `linePrefix` for comment markers: formatters strip that trailing space, and the consumer is then stale again. `lineSuffix:" \\":true` writes a bare `\` on empty lines.

## `code`

Wraps the content in an inline code span. The delimiter is the shortest run of backticks that does not appear in the content, and a space is added inside both ends when the content starts or ends with a backtick (CommonMark).

````text
Input:  my-lib
Output: `my-lib`

Input:  `a` and ``b``
Output: ``` `a` and ``b`` ```
````

## `codeBlock`

Wraps the content in a fenced code block. The optional argument is the info string (language). The fence is one backtick longer than the longest backtick run in the content, with a minimum of three, so content that already contains fences stays intact: content with a four-backtick fence is wrapped in a five-backtick fence.

`codeBlock` does not trim. Put `trim` first, or the provider's blank lines end up inside the fence.

````text
<!-- {=snippet|trim|codeBlock:"typescript"} -->
```typescript
const x = 1;
```
<!-- {/snippet} -->
````

This works with the default padding; no `[padding]` section is needed.

## `replace`

Replaces every occurrence of the first argument with the second. Both arguments are required. An empty replacement deletes matches; an empty search string leaves the content unchanged.

| Transformer           | Input         | Output        |
| --------------------- | ------------- | ------------- |
| `replace:"foo":"bar"` | `foo and foo` | `bar and bar` |
| `replace:"my-":""`    | `my-lib`      | `lib`         |

## `if`

Keeps the content when the value at a dot-separated `[data]` path is truthy, and empties the consumer otherwise. The argument is a path, not an expression: `if:"pkg.flags.beta"` works, `if:"pkg.version == 1"` is looked up as a key and is always false.

| Value at the path                      | Result       |
| -------------------------------------- | ------------ |
| missing, `false`, `null`, `""`, `0`    | empty        |
| anything else, including `[]` and `{}` | content kept |

Without a `[data]` section every path is missing, so the consumer is always empty.

## Argument validation

| Transformers                            | Arguments |
| --------------------------------------- | --------- |
| `trim`, `trimStart`, `trimEnd`, `code`  | 0         |
| `prefix`, `suffix`, `wrap`, `codeBlock` | 0-1       |
| `indent`, `linePrefix`, `lineSuffix`    | 0-2       |
| `replace`                               | 2         |
| `if`                                    | 1         |

A wrong argument count or an unknown name is a validation error (exit code 2):

```text
mdt::invalid_transformer_args

  x [readme.md:6:1] transformer `replace` expects 2 argument(s), got 1
  help: check the transformer documentation for the correct number of
        arguments
```

```text
mdt::unknown_transformer

  x [readme.md:3:1] unknown transformer `shout`
  help: available transformers: trim, trimStart, trimEnd, indent, prefix,
        suffix, linePrefix, lineSuffix, wrap, codeBlock, code, replace, if
```

`--ignore-invalid-transformers` silences both.
