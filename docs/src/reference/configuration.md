# Configuration Reference

mdt is configured via a TOML file in the project root. All sections are optional.

## File location

mdt resolves config in the root directory passed via `--path` (or the current directory if not specified) using this precedence:

1. `mdt.toml`
2. `.mdt.toml`
3. `.config/mdt.toml`

## Sections

### `[data]`

Maps namespace names to data sources. Each key becomes a namespace for template variable access.

```toml
[data]
package = "package.json"
cargo = "Cargo.toml"
config = "settings.yaml"
metadata = "data.kdl"
release = { path = "release-info", format = "json" }
version = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

**Keys:** Any valid TOML key. Used as the namespace prefix in templates (`{{ key.field }}`).

**Values:**

- String path (backward-compatible): `pkg = "package.json"`
- Typed entry with explicit format: `release = { path = "release-info", format = "json" }`
- Script entry: `version = { command = "cat VERSION", format = "text", watch = ["VERSION"] }`

String paths infer format from file extension. Typed entries use `format` and are useful for files without extensions.

### Script-backed data sources

<!-- {=mdtScriptDataSourcesGuide} -->

`[data]` entries can run shell commands and use stdout as template data. This is useful for values that come from tooling (for example Nix, git metadata, or generated version files).

```toml
[data]
release = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

- `command`: shell command executed from the project root.
- `format`: parser for stdout (`text`, `json`, `toml`, `yaml`, `yml`, `kdl`, `ini`).
- `watch`: files that control cache invalidation.

When `watch` files are unchanged, mdt reuses cached script output from `.mdt/cache/data-v1.json` instead of re-running the command.

<!-- {/mdtScriptDataSourcesGuide} -->

<!-- {=mdtScriptDataSourcesNotes} -->

- Script outputs are cached per namespace, command, format, and watch list.
- If `watch` is empty, mdt re-runs the script every load (no cache hit).
- A non-zero script exit status fails data loading with an explicit error.

<!-- {/mdtScriptDataSourcesNotes} -->

**Supported formats:**

| Format / Extension | Parser                              |
| ------------------ | ----------------------------------- |
| `json`, `.json`    | JSON (`serde_json`)                 |
| `toml`, `.toml`    | TOML (converted to JSON internally) |
| `yaml`, `.yaml`    | YAML (`serde_yaml_ng`)              |
| `yml`, `.yml`      | YAML (`serde_yaml_ng`)              |
| `kdl`, `.kdl`      | KDL (converted to JSON internally)  |
| `ini`, `.ini`      | INI (`serde_ini`)                   |

Other formats produce an error:

```
error: unsupported data file format: `xml`
  help: supported formats: text, json, toml, yaml, yml, kdl, ini
```

If a referenced file doesn't exist, mdt produces an error:

```
error: failed to load data file `missing.json`: No such file or directory
```

### `[exclude]`

Patterns for files and directories to skip during scanning. Uses **gitignore-style syntax** — the same pattern format as `.gitignore` files, including negation (`!`), directory markers (`/`), wildcards (`*`, `**`), and character classes.

```toml
[exclude]
patterns = [
	"vendor/",
	"dist/",
	"**/*.generated.md",
	"!dist/important.md",
]
```

**`patterns`:** Array of gitignore-style pattern strings. Matched against file paths relative to the project root.

These patterns are applied **in addition to** the built-in exclusions:

- Hidden directories (names starting with `.`)
- `node_modules/`
- `target/`
- Directories containing their own mdt config file (`mdt.toml`, `.mdt.toml`, `.config/mdt.toml`) (sub-project boundaries)

**`markdown_codeblocks`:** Controls whether mdt tags inside fenced code blocks in source files are processed.

| Value                      | Behavior                                                                      |
| -------------------------- | ----------------------------------------------------------------------------- |
| `false` (default)          | Tags in code blocks are processed normally                                    |
| `true`                     | Tags in ALL fenced code blocks are skipped                                    |
| A string (e.g. `"ignore"`) | Tags in code blocks whose info string contains the string are skipped         |
| An array of strings        | Tags in code blocks whose info string contains ANY of the strings are skipped |

```toml
[exclude]
# Skip tags inside all fenced code blocks
markdown_codeblocks = true

# Or skip only code blocks with specific info strings
markdown_codeblocks = "ignore"

# Or skip code blocks matching any of several info strings
markdown_codeblocks = ["ignore", "example", "no-sync"]
```

Markdown fenced code blocks are not treated as live tags by markdown parsing, so this option is specifically for source-file comment scanning.

**`blocks`:** Array of block names to exclude. Any block (provider or consumer) whose name is in this list is completely ignored during scanning and updating.

```toml
[exclude]
blocks = ["draft-section", "deprecated-api"]
```

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
paths = [".templates", "templates", "shared/docs"]
```

**`paths`:** Array of directory paths relative to the project root.

Canonical recommendation: place provider templates in `.templates/`. Compatibility: `templates/` is also supported.

By default (when this section is absent), mdt finds `*.t.md` files in the project tree, including `.templates/`.

### `[padding]`

Controls blank lines between block tags and their content. When absent, no padding is applied. When present, `before` and `after` control how many blank lines separate tags from content.

```toml
[padding]
before = 0
after = 0
```

**`before`:** Controls blank lines between the opening tag and the content.

**`after`:** Controls blank lines between the content and the closing tag.

Both accept the same values:

| Value   | Behavior                                                           |
| ------- | ------------------------------------------------------------------ |
| `false` | Content appears inline with the tag (no newline separator)         |
| `0`     | Content starts on the very next line (one newline, no blank lines) |
| `1`     | One blank line between the tag and content                         |
| `2`     | Two blank lines, and so on                                         |

When `[padding]` is present but `before`/`after` are omitted, they default to `1`.

In source code files with comment prefixes (e.g., `//!`, `///`, `*`), blank lines include the comment prefix to maintain valid syntax.

This is especially important for **source code files** (`.rs`, `.ts`, `.py`, `.go`, etc.) where consumer blocks appear inside code comments. Without padding, transformers like `trim` followed by `linePrefix` can produce content that merges with the surrounding tags, breaking the code structure.

**Example:** `before = 0, after = 0` — content directly on the next line:

```rust
//! <!-- {=docs|trim|linePrefix:"//! ":true} -->
//! This content stays properly formatted.
//! <!-- {/docs} -->
```

**Example:** `before = 1, after = 1` (default when `[padding]` is present) — one blank line:

```rust
//! <!-- {=docs|trim|linePrefix:"//! ":true} -->
//!
//! This content stays properly formatted.
//!
//! <!-- {/docs} -->
```

Without `[padding]`, the same setup might produce:

```rust
//! <!-- {=docs|trim|linePrefix:"//! ":true} -->This content merges with the
//! tag.<!-- {/docs} -->
```

**Recommended setting for projects with formatters:** Use `before = 0, after = 0` to minimize whitespace that formatters might alter, ensuring `mdt check` stays clean after formatting.

### `max_file_size`

Maximum file size in bytes that mdt will scan.

```toml
max_file_size = 10485760
```

If a scanned file exceeds this value, mdt returns an error instead of reading it.

Default value: `10485760` (10 MB).

### `disable_gitignore`

Disables `.gitignore` integration when scanning for files.

```toml
disable_gitignore = true
```

| Value             | Behavior                                                                 |
| ----------------- | ------------------------------------------------------------------------ |
| `false` (default) | mdt respects `.gitignore` patterns and skips files that git would ignore |
| `true`            | mdt ignores `.gitignore` rules and scans all files                       |

When set to `true`, file filtering is controlled entirely by the `[exclude]` and `[include]` sections. The built-in exclusions (hidden directories, `node_modules/`, `target/`, sub-project boundaries) still apply.

Use this when scanning generated files or build output that contains mdt consumer blocks, when working outside a git repository, or when you want full control over file scanning via `[exclude]` and `[include]` patterns.

**Type:** `bool`

**Default:** `false`

## Complete example

```toml
# mdt.toml

# Refuse to scan files larger than 10 MB
max_file_size = 10485760

# Respect .gitignore rules (default behavior)
disable_gitignore = false

# Ensure content is properly separated from tags (recommended for source files)
[padding]
before = 0
after = 0

# Map data files to namespaces for template variables
[data]
package = "package.json"
cargo = "my-lib/Cargo.toml"
config = "config.yaml"

# Skip these paths during scanning (gitignore-style patterns)
[exclude]
patterns = [
	"vendor/",
	"dist/",
	"coverage/",
]
blocks = ["draft-section"]
markdown_codeblocks = true

# Only scan source files matching these patterns
[include]
patterns = ["src/**", "docs/**"]

# Only look for templates in this directory
[templates]
paths = ["templates"]
```

## Minimal example

A minimal config for data interpolation only:

```toml
[data]
package = "package.json"
```

## No config

If no config file (`mdt.toml`, `.mdt.toml`, or `.config/mdt.toml`) exists, mdt uses defaults:

- No data interpolation (template variables pass through unchanged)
- No extra exclusions (only built-in exclusions apply, no block or code block filtering)
- No include filtering (all scannable files are scanned)
- Templates found anywhere in the project tree
- No padding (content is not adjusted between tags)
- `max_file_size` defaults to 10 MB
- `.gitignore` rules are respected (`disable_gitignore` defaults to `false`)
