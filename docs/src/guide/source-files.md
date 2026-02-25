# Source File Support

mdt isn't limited to markdown files. Consumer tags work inside code comments in any language that supports `<!-- -->` HTML comments within its comment syntax.

## How it works

mdt scans source files for HTML comment patterns (`<!-- ... -->`) embedded within code comments. The same `{=name}` / `{/name}` consumer syntax works regardless of the surrounding comment style.

## Supported languages

mdt recognizes these source file extensions:

| Language   | Extensions         |
| ---------- | ------------------ |
| Rust       | `.rs`              |
| TypeScript | `.ts`, `.tsx`      |
| JavaScript | `.js`, `.jsx`      |
| Python     | `.py`              |
| Go         | `.go`              |
| Java       | `.java`            |
| Kotlin     | `.kt`              |
| Swift      | `.swift`           |
| C/C++      | `.c`, `.cpp`, `.h` |
| C#         | `.cs`              |

## Examples by language

### Rust doc comments

Keep crate-level documentation in sync with your README:

```rust
//! <!-- {=packageDescription|trim} -->
//! A fast, type-safe HTTP client for Rust.
//! <!-- {/packageDescription} -->

pub fn main() {}
```

For `///` doc comments on items, use `linePrefix` to add the prefix:

```rust
/// <!-- {=apiDocs|trim|linePrefix:"/// "} -->
/// API documentation here.
/// <!-- {/apiDocs} -->
pub fn create_client() {}
```

### TypeScript / JavaScript JSDoc

Keep JSDoc in sync with your docs:

```typescript
/**
 * <!-- {=apiDocs|trim|indent:" * "} -->
 * Old JSDoc content.
 * <!-- {/apiDocs} -->
 */
export function createClient() {
	return {};
}
```

### Python docstrings

```python
# <!-- {=moduleDoc|trim} -->
# Module documentation here.
# <!-- {/moduleDoc} -->

def main():
    pass
```

### Go comments

```go
// <!-- {=packageDoc|trim|linePrefix:"// "} -->
// Package documentation.
// <!-- {/packageDoc} -->
package mylib
```

## Recommended: Enable `[padding]`

When using consumer blocks in source files, add a `[padding]` section to your `mdt.toml`:

```toml
[padding]
before = 0
after = 0
```

This ensures content is properly separated from the surrounding tags. The `before` and `after` values control how many blank lines appear between tags and content:

- `false` — Content inline with tag (no newline)
- `0` — Content on the very next line (recommended for projects using formatters)
- `1` — One blank line between tag and content
- `2` — Two blank lines, etc.

Without `[padding]`, a consumer with `trim|linePrefix:"//! ":true` could produce:

```rust
//! <!-- {=docs|trim|linePrefix:"//! ":true} -->//! Content here.<!-- {/docs}
//! -->
```

With `before = 0, after = 0`, the output is properly structured:

```rust
//! <!-- {=docs|trim|linePrefix:"//! ":true} -->
//! Content here.
//! <!-- {/docs} -->
```

With `before = 1, after = 1`, blank lines are added between tags and content:

```rust
//! <!-- {=docs|trim|linePrefix:"//! ":true} -->
//!
//! Content here.
//!
//! <!-- {/docs} -->
```

## Key differences from markdown

### Lenient parsing

Source file parsing is **lenient**. If an opening tag has no matching close tag, it's silently ignored rather than producing an error. This prevents false positives when HTML comments appear in strings or other non-tag contexts.

### Provider blocks in source files

Source files can only contain **consumer** blocks. Even if you write `{@name}` in a source file, it won't be recognized as a provider. Providers must be in `*.t.md` template files.

## Real-world example

Consider a TypeScript library where you want the README, JSDoc, and mdbook docs to stay in sync.

**`template.t.md`** defines the content:

```
<!-- {@apiDocs} -->

A sample TypeScript library.

## Usage

    import { createClient } from "my-lib";
    const client = createClient();

<!-- {/apiDocs} -->
```

**`readme.md`** consumes it as-is:

```
## API

<!-- {=apiDocs} -->
<!-- {/apiDocs} -->
```

**`src/index.ts`** consumes it with transformers for JSDoc formatting:

```typescript
/**
 * <!-- {=apiDocs|trim|indent:" * "} -->
 * <!-- {/apiDocs} -->
 */
export function createClient() {
	return {};
}
```

Running `mdt update` fills both consumers. The readme gets the content as-is. The TypeScript file gets the content trimmed and indented with `*` for JSDoc formatting.
