# mdt Reference

## Tag syntax

All mdt tags live inside HTML comments, so they are invisible in rendered markdown.

```
<!-- {sigil name | transformers} -->
       │      │    │
       │      │    └── Optional: pipe-delimited content filters
       │      └─────── The block name (globally unique for providers)
       └────────────── @ provider, = consumer, ~ inline, / close
```

**Block names** may contain ASCII letters, digits, `_`, and `-` (e.g. `installCommand`, `install-command`). Other punctuation (`.`, `/`, spaces) makes the tag unparseable and it is **silently ignored** — if a block does not show up in `mdt list`, check the name first, then run `mdt doctor`.

### Provider (define content in `*.t.md` files only)

```markdown
<!-- {@greeting} -->

Hello from mdt!

<!-- {/greeting} -->
```

### Consumer (reference content — markdown or source files)

```markdown
<!-- {=greeting} -->

Replaced on `mdt update`.

<!-- {/greeting} -->
```

### Inline (provider-free interpolation using data context)

```markdown
Version: <!-- {~ver:"{{ pkg.version }}"} -->0.0.0<!-- {/ver} -->
```

### Close tag (shared by all block types)

```markdown
<!-- {/blockName} -->
```

## Transformers

Transformers are pipe-delimited filters applied left-to-right on the consumer tag. They modify provider content before injection.

| Transformer  | Arguments               | Description                                                               |
| ------------ | ----------------------- | ------------------------------------------------------------------------- |
| `trim`       | none                    | Strip whitespace from both ends                                           |
| `trimStart`  | none                    | Strip leading whitespace                                                  |
| `trimEnd`    | none                    | Strip trailing whitespace                                                 |
| `indent`     | `string` [, `bool`]     | Prepend string to each non-empty line. Pass `true` to include empty lines |
| `prefix`     | `string`                | Prepend string to entire content                                          |
| `suffix`     | `string`                | Append string to entire content                                           |
| `linePrefix` | `string` [, `bool`]     | Prepend string per line. Pass `true` to include empty lines               |
| `lineSuffix` | `string` [, `bool`]     | Append string per line. Pass `true` to include empty lines                |
| `wrap`       | `string`                | Wrap content on both sides with the string                                |
| `code`       | none                    | Wrap in inline backticks                                                  |
| `codeBlock`  | [`string`]              | Wrap in fenced code block with optional language                          |
| `replace`    | `search`, `replacement` | Replace all occurrences                                                   |
| `if`         | `condition`             | Include content only when condition is truthy                             |

All transformers accept both camelCase and snake_case: `linePrefix` / `line_prefix`, `trimStart` / `trim_start`, etc.

### Common patterns

**Rust `//!` module docs:**

```markdown
<!-- {=docs|trim|linePrefix:"//! ":true} -->
<!-- {/docs} -->
```

**Rust `///` item docs:**

```markdown
<!-- {=docs|trim|linePrefix:"/// ":true} -->
<!-- {/docs} -->
```

**JSDoc / TypeScript:**

```markdown
<!-- {=docs|trim|indent:" * ":true} -->
<!-- {/docs} -->
```

**Go comments:**

```markdown
<!-- {=docs|trim|linePrefix:"// ":true} -->
<!-- {/docs} -->
```

**Dart `///` doc comments:**

```markdown
<!-- {=docs|trim|linePrefix:"/// ":true} -->
<!-- {/docs} -->
```

**Python `#` comments:**

```markdown
<!-- {=docs|trim|linePrefix:"# "} -->
<!-- {/docs} -->
```

**Fenced code block (e.g. install command in a README):**

```markdown
<!-- {=install|trim|codeBlock:"sh"} -->
<!-- {/install} -->
```

The `codeBlock` transformer generates the fence itself — consumer tags sit on ordinary lines, and any static `` ```sh `` fence that was there before should be deleted (tags inside an existing fence are inert).

## Data interpolation

Provider content supports [minijinja](https://docs.rs/minijinja) template variables populated from project files.

### Configuration (`mdt.toml`)

```toml
[data]
pkg = "package.json"
cargo = "Cargo.toml"
config = "config.yaml"
release = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
typed = { path = "release-info", format = "json" }
```

The namespace name (`pkg`, `cargo`, ...) is arbitrary — pick one per source file. Typed sources (`path` + `format`) force a parser when the file extension is missing or unusual.

### Usage in providers

````markdown
<!-- {@install} -->

Install `{{ pkg.name }}` version {{ pkg.version }}:

```sh
npm install {{ pkg.name }}@{{ pkg.version }}
```

<!-- {/install} -->
````

### Supported formats

| Format | Extensions          |
| ------ | ------------------- |
| JSON   | `.json`             |
| TOML   | `.toml`             |
| YAML   | `.yaml`, `.yml`     |
| KDL    | `.kdl`              |
| INI    | `.ini`              |
| Text   | `.txt` (raw string) |

### Template features (minijinja)

- `{{ namespace.key }}` — variable
- `{{ namespace.key | upper }}` — built-in filter
- `{% if pkg.private %}...{% endif %}` — conditional
- `{% for f in config.features %}...{% endfor %}` — loop

Undefined variables render as empty strings. Template rendering happens **before** transformers are applied.

### Script data sources

```toml
[data]
release = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

- `command` runs from the project root.
- `watch` files control cache invalidation.
- Cached in `.mdt/cache/data-v1.json` when watch files are unchanged.

## Inline blocks

Inline blocks render a template expression without a separate provider. Useful for single values like versions.

```markdown
Install version <!-- {~v:"{{ pkg.version }}"} -->0.0.0<!-- {/v} --> today.
```

In tables:

```markdown
| Package | Version                                                 |
| ------- | ------------------------------------------------------- |
| mdt     | <!-- {~ver:"{{ pkg.version }}"} -->0.0.0<!-- {/ver} --> |
```

With transformers:

```markdown
Version: <!-- {~ver:"{{ pkg.version }}"|code} -->`0.0.0`<!-- {/ver} -->
```

## Configuration (`mdt.toml`)

```toml
# Maximum file size for scanning (default: 10MB)
max_file_size = 10485760

# Disable .gitignore integration (default: false)
disable_gitignore = false

[data]
package = "package.json"

[padding]
before = 0 # false = inline, 0 = next line, 1 = one blank line, 2+ = more
after = 0

[check]
comparison = "lenient" # or "strict" (default)

[exclude]
patterns = ["vendor/", "dist/"]
blocks = ["draft-section"]
markdown_codeblocks = true # or "ignore" or ["ignore", "example"]

[include]
patterns = ["src/**", "docs/**"]

[templates]
paths = [".templates"]

[[formatters]]
command = "dprint fmt --stdin \"{{ filePath }}\""
patterns = ["**/*.md"]
ignore = ["**/*.t.md"]
```

Unknown keys and sections are **silently ignored** — there is no validation error, so double-check spelling against this reference when a config change appears to do nothing.

### `[padding]` for source-file consumers

Controls blank lines between tags and content.

- With **no `[padding]` section**: content starts on the line after the opening tag, and the closing tag is written **inline** with the content — in source files this glues `-->` onto the last content line. Always set the section for source-file consumers.
- With the section present but values omitted: `before`/`after` default to `1`.
- `false` — content inline with tag; `0` — content on next line (recommended with formatters); `1` — one blank line; `2+` — more.

`[padding]` does not add blank lines around markdown consumers, but it **does** move content authored on the same line as the opening tag onto its own line. For same-line values in markdown (version numbers, badges), use an **inline** (`~`) block instead of a consumer — inline blocks are unaffected by `[padding]`. In source files, blank lines inherit the surrounding comment prefix (e.g. `//!`, `///`, `*`).

### `[check]` — comparison strictness

```toml
[check]
comparison = "lenient"
```

- `"strict"` (default) — byte-for-byte comparison. Any formatter rewrite of a synced file shows up as stale.
- `"lenient"` — whitespace-normalized comparison: ignores blank-line count, trailing whitespace, and table/JSON formatting differences, so external formatters do not cause false staleness.

`mdt update` always writes exact bytes regardless of this setting.

### `[[formatters]]` — formatter-aware update/check pipeline

The built-in fix for the `mdt update → formatter → mdt check` loop. Each matching entry runs the named formatter over the **full candidate file** (stdin → stdout, from the project root):

- after block injection during `mdt update`
- before expected-output comparison during `mdt check`
- in declaration order when multiple entries match the same file

```toml
[[formatters]]
command = "dprint fmt --stdin \"{{ filePath }}\""
patterns = ["**/*.md"]
ignore = ["**/*.t.md"]

[[formatters]]
command = "prettier --stdin-filepath \"{{ filePath }}\""
patterns = ["**/*.ts", "**/*.tsx"]
```

- `command` is rendered with minijinja: `{{ filePath }}` (absolute), `{{ relativeFilePath }}`, `{{ rootDirectory }}` are available.
- `patterns` and `ignore` are ordered gitignore-style rule lists; a leading `!` re-includes paths.
- A failing formatter command is an explicit error — mdt never silently falls back to unformatted output.
- Keep `*.t.md` provider files out of formatter scope: formatters that treat them as markdown can rewrite `#`-prefixed example lines and `**` glob examples (dprint markdown turns `**/*.ts` into `**/_.ts`).
- Repos without `[[formatters]]` keep the fast legacy path; the feature is opt-in.

### Sub-project boundaries

A directory with its own `mdt.toml` is treated as a separate mdt project. The parent project's scan skips it.

## Source file support

Consumer tags work inside code comments — wrap the HTML-comment tags in the language's comment syntax and re-apply the prefix with a transformer:

```ts
/**
 * <!-- {=apiDocs|trim|indent:" * ":true} -->
 * Old JSDoc content.
 * <!-- {/apiDocs} -->
 */
export function createClient() {
	return {};
}
```

`mdt update` re-emits tag lines with the comment prefix they were authored with; the transformer's prefix applies to content lines only.

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
| Dart       | `.dart`            |

**Important:**

- Source files can only contain **consumer** and **inline** blocks, never providers.
- Only the extensions above (plus markdown) are scanned. Anything else — `.rb`, `.php`, `.vue`, ... — is skipped **silently**: `mdt list` will not show the consumer and `mdt check` passes vacuously. Opt unlisted extensions in with `[include]`:

  ```toml
  [include]
  patterns = ["**/*.md", "**/*.rb"]
  ```

  (Keep `**/*.md` in the list or markdown files stop being scanned.) Included files are parsed like markdown: HTML comments are found anywhere, so keep the file valid by wrapping tags in real comments.
- Parsing is lenient: unclosed tags are silently ignored.
- Use `[padding]` (`before = 0, after = 0`) to prevent content merging with tags.

### Dart example

```dart
// <!-- {=pkgDescription|trim|linePrefix:"/// ":true} -->
// <!-- {/pkgDescription} -->
library;
```

After `mdt update`:

```dart
// <!-- {=pkgDescription|trim|linePrefix:"/// ":true} -->
/// A collection of fancy widgets for Flutter apps.
// <!-- {/pkgDescription} -->
library;
```

Tag lines keep the comment prefix you author them with — the transformer prefix applies to content lines only. If the tag lines themselves must be doc comments (e.g. so `dart analyze` attaches the docs), author them with `///`.

`.dart` files are scanned by default in recent versions. If `mdt list` does not show the consumer, upgrade the CLI or use the `[include]` opt-in above.

## CLI commands

| Command                            | Purpose                                                                        |
| ---------------------------------- | ------------------------------------------------------------------------------ |
| `mdt init`                         | Create starter `.templates/template.t.md` and `mdt.toml` (project stays green) |
| `mdt check [--diff] [--watch]`     | Verify consumers are current. Non-zero exit on stale.                          |
| `mdt update [--dry-run] [--watch]` | Sync all consumers with provider content                                       |
| `mdt list`                         | List all providers and consumers with status                                   |
| `mdt info [--format json]`         | Project diagnostics and cache telemetry                                        |
| `mdt doctor [--format json]`       | Health checks with actionable hints — run this first when something is off     |
| `mdt assist <assistant>`           | Print MCP config and setup guidance                                            |
| `mdt lsp`                          | Start the Language Server Protocol server                                      |
| `mdt mcp`                          | Start the Model Context Protocol server                                        |

Notes:

- There is no `mdt preview` subcommand; rendered previews are MCP-only (`mdt_preview`).
- `mdt check` reports orphan consumers (no matching provider) as warnings but still exits 0; use `mdt doctor` for a hard orphan check.

### Common flags

- `--path <dir>` — Project root (default: current directory)
- `--verbose` — Show detailed output
- `--no-color` — Disable colored output

## MCP server tools

The MCP server (`mdt mcp`) exposes these tools to AI assistants:

| Tool             | Description                                                         |
| ---------------- | ------------------------------------------------------------------- |
| `mdt_init`       | Initialize a new mdt project                                        |
| `mdt_check`      | Verify all consumer blocks are up-to-date (returns structured JSON) |
| `mdt_update`     | Update all consumer blocks (supports `dry_run`)                     |
| `mdt_list`       | List all providers and consumers with file locations                |
| `mdt_find_reuse` | Find similar providers and reuse opportunities                      |
| `mdt_get_block`  | Get a specific block's content by name                              |
| `mdt_preview`    | Preview rendered provider + consumer output with transformers       |

### Agent best practices

1. **Reuse first** — Always call `mdt_find_reuse` before creating a new provider block.
2. **Preview before sync** — Use `mdt_preview` to inspect rendered output before running `mdt_update`.
3. **Check after edits** — Call `mdt_check` after any documentation change.
4. **JSON responses** — All MCP tool responses are structured JSON. Parse them directly.
5. **Unique names** — Provider names must be globally unique across all `*.t.md` files.
6. **Canonical layout** — Use `.templates/` as the template directory.

## File conventions

| Pattern                                       | Role                                                              |
| --------------------------------------------- | ----------------------------------------------------------------- |
| `*.t.md`                                      | Template files — only these contain provider blocks               |
| `*.md`, `*.mdx`, `*.markdown`                 | Markdown files — scanned for consumer and inline blocks           |
| `*.rs`, `*.ts`, `*.dart`, `*.py`, etc.        | Source files — scanned for consumer and inline blocks in comments |
| `mdt.toml` / `.mdt.toml` / `.config/mdt.toml` | Configuration file                                                |
| `.mdt/cache/`                                 | Cache directory (auto-managed; gitignore it)                      |
| `.templates/`                                 | Canonical template directory                                      |

Additional tips:

- Tags inside properly closed fenced code blocks in markdown files are **not** scanned. Watch fence nesting: an example that contains its own `` ``` `` fence closes the outer fence early, and tags after that point become live consumers. Use 4-backtick outer fences for such examples, or exclude the file via `[exclude] patterns`.
- `[exclude] markdown_codeblocks` only affects fenced code blocks inside source-file comments.
- Do not format `*.t.md` files with markdown formatters; exclude them from formatter configs (see `[[formatters]]`).

## Skipped by default

- Hidden directories (`.git`, `.vscode`, etc.)
- `node_modules/`
- `target/` (Rust build output)
- Directories with their own `mdt.toml` (sub-projects)
- Files matching `.gitignore` rules (unless `disable_gitignore = true`)
- Source-file extensions outside the supported list (opt in via `[include] patterns`)
