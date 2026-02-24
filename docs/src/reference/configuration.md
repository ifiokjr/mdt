# Configuration Reference

mdt is configured via an `mdt.toml` file in the project root. All sections are optional.

## File location

mdt looks for `mdt.toml` in the root directory passed via `--path` (or the current directory if not specified).

## Sections

### `[data]`

Maps namespace names to data file paths. Each key becomes a namespace for template variable access.

```toml
[data]
package = "package.json"
cargo = "Cargo.toml"
config = "settings.yaml"
metadata = "data.kdl"
```

**Keys:** Any valid TOML key. Used as the namespace prefix in templates (`{{ key.field }}`).

**Values:** Relative file paths from the project root. The file extension determines the parser.

**Supported formats:**

| Extension       | Parser                              |
| --------------- | ----------------------------------- |
| `.json`         | JSON (`serde_json`)                 |
| `.toml`         | TOML (converted to JSON internally) |
| `.yaml`, `.yml` | YAML (`serde_yml`)                  |
| `.kdl`          | KDL (converted to JSON internally)  |

Other extensions produce an error:

```
error: unsupported data file format: `xml`
  help: supported formats: .json, .toml, .yaml, .yml, .kdl
```

If a referenced file doesn't exist, mdt produces an error:

```
error: failed to load data file `missing.json`: No such file or directory
```

### `[exclude]`

Glob patterns for files and directories to skip during scanning.

```toml
[exclude]
patterns = [
	"vendor/**",
	"dist/**",
	"**/*.generated.md",
]
```

**`patterns`:** Array of glob strings. Matched against file paths relative to the project root.

These patterns are applied **in addition to** the built-in exclusions:

- Hidden directories (names starting with `.`)
- `node_modules/`
- `target/`
- Directories containing their own `mdt.toml` (sub-project boundaries)

### `[include]`

Glob patterns to restrict which files are scanned.

```toml
[include]
patterns = ["docs/**/*.rs", "src/**/*.ts"]
```

**`patterns`:** Array of glob strings. When present, only files matching at least one pattern are scanned.

Markdown files (`*.md`, `*.mdx`, `*.markdown`) and template files (`*.t.md`) are always scanned regardless of include patterns.

### `[templates]`

Directories to search for template files.

```toml
[templates]
paths = ["templates", "shared/docs"]
```

**`paths`:** Array of directory paths relative to the project root. When present, only `*.t.md` files within these directories are recognized as templates.

By default (when this section is absent), mdt finds `*.t.md` files anywhere in the project tree.

### `pad_blocks`

When set to `true`, mdt ensures a newline always separates the opening tag from the content and the content from the closing tag. This prevents content from running directly into tags when transformers produce output without leading or trailing newlines.

```toml
pad_blocks = true
```

This is especially important for **source code files** (`.rs`, `.ts`, `.py`, `.go`, etc.) where consumer blocks appear inside code comments. Without padding, transformers like `trim` followed by `linePrefix` can produce content that merges with the surrounding tags, breaking the code structure.

**Example:** A Rust file with `pad_blocks = true`:

```rust
//! <!-- {=docs|trim|linePrefix:"//! ":true} -->
//! This content stays properly formatted.
//! <!-- {/docs} -->
```

Without `pad_blocks`, the same setup might produce:

```rust
//! <!-- {=docs|trim|linePrefix:"//! ":true} -->This content merges with the
//! tag.<!-- {/docs} -->
```

Default value: `false`.

### `max_file_size`

Maximum file size in bytes that mdt will scan.

```toml
max_file_size = 10485760
```

If a scanned file exceeds this value, mdt returns an error instead of reading it.

Default value: `10485760` (10 MB).

## Complete example

```toml
# mdt.toml

# Ensure newlines separate tags from content (recommended for source files)
pad_blocks = true

# Map data files to namespaces for template variables
[data]
package = "package.json"
cargo = "crates/my-lib/Cargo.toml"
config = "config.yaml"

# Skip these paths during scanning
[exclude]
patterns = [
	"vendor/**",
	"dist/**",
	"coverage/**",
]

# Only scan source files matching these patterns
[include]
patterns = ["src/**", "docs/**"]

# Only look for templates in this directory
[templates]
paths = ["templates"]

# Refuse to scan files larger than 10 MB
max_file_size = 10485760
```

## Minimal example

A minimal config for data interpolation only:

```toml
[data]
package = "package.json"
```

## No config

If `mdt.toml` doesn't exist, mdt uses defaults:

- No data interpolation (template variables pass through unchanged)
- No extra exclusions (only built-in exclusions apply)
- No include filtering (all scannable files are scanned)
- Templates found anywhere in the project tree
- `pad_blocks` defaults to `false`
- `max_file_size` defaults to 10 MB
