# Transformer Reference

Quick reference for all available transformers.

## Summary table

| Transformer  | Arguments                               | Description                            |
| ------------ | --------------------------------------- | -------------------------------------- |
| `trim`       | none                                    | Remove whitespace from both ends       |
| `trimStart`  | none                                    | Remove whitespace from the start       |
| `trimEnd`    | none                                    | Remove whitespace from the end         |
| `indent`     | `string` (optional), `bool` (optional)  | Prepend string to each line            |
| `prefix`     | `string` (optional)                     | Prepend string to entire content       |
| `suffix`     | `string` (optional)                     | Append string to entire content        |
| `linePrefix` | `string` (optional), `bool` (optional)  | Prepend string to each line            |
| `lineSuffix` | `string` (optional), `bool` (optional)  | Append string to each line             |
| `wrap`       | `string` (optional)                     | Wrap content with string on both sides |
| `code`       | none                                    | Wrap in inline code backticks          |
| `codeBlock`  | `language` (optional)                   | Wrap in fenced code block              |
| `replace`    | `search`, `replacement` (both required) | Replace all occurrences                |

## Alias table

| Primary name | Alias         |
| ------------ | ------------- |
| `trimStart`  | `trim_start`  |
| `trimEnd`    | `trim_end`    |
| `codeBlock`  | `code_block`  |
| `linePrefix` | `line_prefix` |
| `lineSuffix` | `line_suffix` |

## Detailed reference

### `trim`

```
|trim
```

Removes leading and trailing whitespace (spaces, tabs, newlines).

**Arguments:** none

**Example:**

| Input           | Output  |
| --------------- | ------- |
| `\n  hello  \n` | `hello` |

---

### `trimStart`

```
|trimStart
```

Removes leading whitespace only.

**Arguments:** none

---

### `trimEnd`

```
|trimEnd
```

Removes trailing whitespace only.

**Arguments:** none

---

### `indent`

```
|indent:"  "
|indent:"  ":true
|indent
```

Prepends the given string to each line. By default, empty lines are left empty. Pass `true` as a second argument to also indent empty lines.

**Arguments:** 0-2 (string, optional boolean)

- First argument: the indent string (defaults to empty string)
- Second argument: `true` to include empty lines, `false` or omitted to skip them

**Example:**

Input:

```
line 1

line 3
```

With `|indent:"  "` (default — skips empty lines):

```
  line 1

  line 3
```

With `|indent:"  ":true`, every line gets the indent — including empty lines (which become lines containing only the indent string).

---

### `prefix`

```
|prefix:"# "
|prefix
```

Prepends the string to the entire content (once, not per-line).

**Arguments:** 0-1 string

---

### `suffix`

```
|suffix:"\n"
|suffix
```

Appends the string to the entire content.

**Arguments:** 0-1 string

---

### `linePrefix`

```
|linePrefix:"// "
|linePrefix:"//! ":true
|line_prefix:"// "
```

Prepends the string to each line. By default, empty lines are left empty. Pass `true` as a second argument to also prefix empty lines — essential for code comment blocks.

**Arguments:** 0-2 (string, optional boolean)

- First argument: the prefix string (defaults to empty string)
- Second argument: `true` to include empty lines, `false` or omitted to skip them

**Example:**

Input:

```
A fast HTTP client.

Supports async and blocking modes.
```

With `|linePrefix:"/// ":true`:

```
/// A fast HTTP client.
///
/// Supports async and blocking modes.
```

Without `true`, the empty line would be left blank (breaking Rust doc comments).

---

### `lineSuffix`

```
|lineSuffix:" \\"
|lineSuffix:";":true
|line_suffix:" \\"
```

Appends the string to each line. By default, empty lines are left empty. Pass `true` as a second argument to also suffix empty lines.

**Arguments:** 0-2 (string, optional boolean)

- First argument: the suffix string (defaults to empty string)
- Second argument: `true` to include empty lines, `false` or omitted to skip them

---

### `wrap`

```
|wrap:"**"
```

Wraps the entire content: prepends and appends the same string.

**Arguments:** 0-1 string

**Example:**

| Input       | With `\|wrap:"**"` |
| ----------- | ------------------ |
| `bold text` | `**bold text**`    |

---

### `code`

```
|code
```

Wraps the content in inline code backticks.

**Arguments:** none

**Example:**

| Input    | Output         |
| -------- | -------------- |
| `my-lib` | `` `my-lib` `` |

---

### `codeBlock`

```
|codeBlock:"rust"
|codeBlock
|code_block:"typescript"
```

Wraps the content in a fenced code block. The optional argument specifies the language.

**Arguments:** 0-1 string (language identifier)

**Example with language:**

Input: `let x = 1;`

Output:

````
```rust
let x = 1;
```
````

---

### `replace`

```
|replace:"search":"replacement"
```

Replaces all occurrences of the search string with the replacement.

**Arguments:** exactly 2 strings (search, replacement)

**Example:**

| Input         | With `\|replace:"foo":"bar"` |
| ------------- | ---------------------------- |
| `foo and foo` | `bar and bar`                |

To delete occurrences, use an empty replacement:

```
|replace:"unwanted":""
```

## Argument validation

mdt validates transformer arguments at runtime:

| Transformer                             | Expected args |
| --------------------------------------- | ------------- |
| `trim`, `trimStart`, `trimEnd`, `code`  | 0             |
| `prefix`, `suffix`, `wrap`, `codeBlock` | 0-1           |
| `indent`, `linePrefix`, `lineSuffix`    | 0-2           |
| `replace`                               | exactly 2     |

Passing the wrong number of arguments produces an error:

```
error: transformer `replace` expects 2 argument(s), got 1
```
