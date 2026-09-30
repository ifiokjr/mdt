# Source File Support

Consumer and inline blocks also work inside source code comments, so doc comments can share content with your README. The tags are the same HTML comments used in markdown, written inside the language's comment syntax. Providers still live only in `*.t.md` files; a provider tag in a source file is ignored with a `mdt::provider_outside_template` warning.

## Scanned files

| Language   | Extensions                                       |
| ---------- | ------------------------------------------------ |
| Markdown   | `.md`, `.mdx`, `.markdown`                       |
| Rust       | `.rs`                                            |
| TypeScript | `.ts`, `.tsx`, `.mts`, `.cts`                    |
| JavaScript | `.js`, `.jsx`, `.mjs`, `.cjs`                    |
| Python     | `.py`                                            |
| Go         | `.go`                                            |
| Java       | `.java`                                          |
| Kotlin     | `.kt`                                            |
| Swift      | `.swift`                                         |
| C/C++      | `.c`, `.cc`, `.cpp`, `.cxx`, `.h`, `.hh`, `.hpp` |
| C#         | `.cs`                                            |
| Dart       | `.dart`                                          |

Files with any other extension are skipped without a message: a consumer in an unscanned file never appears in `mdt list`. To scan another extension, add it with `[include]`:

```toml
[include]
patterns = ["**/*.rb"]
```

`[include]` adds files to the default scan; it never narrows it, so markdown and the extensions above are still scanned. Included files respect `.gitignore` and `[exclude]`. Included non-markdown files are scanned like source files, with tags found in any comment. Avoid broad globs such as `src/**`: a matched binary file fails the scan with `mdt::read_file`.

## Always re-apply the comment marker

`mdt update` replaces every line between the opening and closing tags. The tag lines keep the comment marker you wrote, but content lines only get what the transformers add. A consumer without a prefix transformer writes bare text into the code:

```rust
//! <!-- {=clientDocs|trim} -->
A fast HTTP client.

Supports async and blocking modes.
//! <!-- {/clientDocs} -->
```

```text
error: expected one of `!` or `::`, found `fast`
```

Add `linePrefix:"<marker>":true` to every source-file consumer. With `true`, empty lines get the marker too, with its trailing space trimmed.

## Examples by language

Each example below is the file after `mdt update`, using this provider:

```markdown
<!-- {@clientDocs} -->

A fast HTTP client.

Supports async and blocking modes.

<!-- {/clientDocs} -->
```

### Rust

```rust
//! <!-- {=clientDocs|trim|linePrefix:"//! ":true} -->
//! A fast HTTP client.
//!
//! Supports async and blocking modes.
//! <!-- {/clientDocs} -->

/// <!-- {=clientDocs|trim|linePrefix:"/// ":true} -->
/// A fast HTTP client.
///
/// Supports async and blocking modes.
/// <!-- {/clientDocs} -->
pub fn create_client() {}
```

Inside an `impl` block or other nested item:

```text
impl Client {
    /// <!-- {=clientDocs|trim|linePrefix:"    /// ":true} -->
    /// A fast HTTP client.
    ///
    /// Supports async and blocking modes.
    /// <!-- {/clientDocs} -->
    pub fn new() -> Self {
        Client
    }
}
```

For nested items, put the indentation inside the prefix (`"    /// "`). Indented tag lines keep their indentation, but content lines start at column zero plus the prefix; without the indentation, `rustfmt` re-indents them and `mdt check` reports the consumer stale again.

### TypeScript and JavaScript (JSDoc)

```typescript
/**
 * <!-- {=clientDocs|trim|linePrefix:" * ":true} -->
 * A fast HTTP client.
 *
 * Supports async and blocking modes.
 * <!-- {/clientDocs} -->
 */
export function createClient() {
	return {};
}
```

Use `linePrefix:" * ":true`, not `indent:" * ":true`: `indent` keeps the trailing space of the prefix on empty lines, which formatters strip.

### Go

```go
// <!-- {=clientDocs|trim|linePrefix:"// ":true} -->
// A fast HTTP client.
//
// Supports async and blocking modes.
// <!-- {/clientDocs} -->
package client
```

Without `true`, the blank line splits the comment group and `go doc` shows only the last paragraph.

### Python

Comments:

```python
# <!-- {=clientDocs|trim|linePrefix:"# ":true} -->
# A fast HTTP client.
#
# Supports async and blocking modes.
# <!-- {/clientDocs} -->


def create_client():
    pass
```

Docstrings are string literals, so no prefix is needed:

```python
"""
<!-- {=clientDocs|trim} -->
A fast HTTP client.

Supports async and blocking modes.
<!-- {/clientDocs} -->
"""
```

### Dart

```dart
/// <!-- {=clientDocs|trim|linePrefix:"/// ":true} -->
/// A fast HTTP client.
///
/// Supports async and blocking modes.
/// <!-- {/clientDocs} -->
library;
```

The default padding works for every language above; no `[padding]` section is needed. See [`[padding]`](../reference/configuration.md#padding) to add blank lines between the tags and the content.

## Things to know

### Unclosed tags are errors

An opening tag without a matching closing tag in a source file is a `mdt::unclosed_block` error, as in markdown. `mdt check` and `mdt update` stop with exit code 2 until you close or remove it.

### Block comments

Everything between the tags is replaced, including a `*/` after the opening tag or a `/*` before the closing tag. Per-line block comments such as `/* <!-- {=name} --> */` therefore do not round-trip: the lines between them merge into one comment. Use line comments, or put both tags inside one `/** ... */` block.

Provider text containing `*/` (for example the glob `src/**/*.ts`) ends the surrounding `/* */` comment early. Escape it with `replace`. Write the `/` in the search string as `\u{2f}`, or the tag itself would contain `*/` and end the comment:

```typescript
/**
 * <!-- {=globDocs|trim|replace:"*\u{2f}":"*\\/"|linePrefix:" * ":true} -->
 * Scans `src/**\/*.ts` by default.
 * <!-- {/globDocs} -->
 */
```

### Tags in string literals are live

mdt does not parse the language, so a tag inside a string literal is a real block and `mdt update` rewrites it. Exclude test fixtures that contain example tags:

```toml
[exclude]
patterns = ["tests/fixtures/"]
```

### Tag examples inside doc comments

A fenced code block inside a source comment that shows an mdt tag is scanned like any other comment text. Set `[exclude] markdown_codeblocks = true` to ignore tags inside fenced code blocks in source comments. In markdown files, tags inside fenced code blocks are always inert.

### Visible tags in some doc tools

rustdoc, dartdoc, and TSDoc hide HTML comments. `go doc` and Python's `help()` show them verbatim:

```text
package client // import "."

<!-- {=clientDocs|trim|linePrefix:"// ":true} --> A fast HTTP client.

Supports async and blocking modes. <!-- {/clientDocs} -->
```
