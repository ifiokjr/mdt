# mdt Reference

## Tag Syntax

All mdt tags live inside HTML comments so they are invisible in rendered markdown.

```
<!-- {sigil name | transformers} -->
       │      │    │
       │      │    └── Optional: pipe-delimited content filters
       │      └─────── The block name (globally unique for providers)
       └────────────── @ provider, = consumer, ~ inline, / close
```

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

| Transformer | Arguments | Description |
|-------------|-----------|-------------|
| `trim` | none | Strip whitespace from both ends |
| `trimStart` | none | Strip leading whitespace |
| `trimEnd` | none | Strip trailing whitespace |
| `indent` | `string` [, `bool`] | Prepend string to each non-empty line. Pass `true` to include empty lines |
| `prefix` | `string` | Prepend string to entire content |
| `suffix` | `string` | Append string to entire content |
| `linePrefix` | `string` [, `bool`] | Prepend string per line. Pass `true` to include empty lines |
| `lineSuffix` | `string` [, `bool`] | Append string per line. Pass `true` to include empty lines |
| `wrap` | `string` | Wrap content on both sides with the string |
| `code` | none | Wrap in inline backticks |
| `codeBlock` | [`string`] | Wrap in fenced code block with optional language |
| `replace` | `search`, `replacement` | Replace all occurrences |
| `if` | `condition` | Include content only when condition is truthy |

All transformers accept both camelCase and snake_case: `linePrefix` / `line_prefix`, `trimStart` / `trim_start`, etc.

### Common Patterns

**Rust `//!` doc comments:**

```markdown
<!-- {=docs|trim|linePrefix:"//! ":true} -->
<!-- {/docs} -->
```

**Rust `///` doc comments:**

```markdown
<!-- {=docs|trim|linePrefix:"/// ":true} -->
<!-- {/docs} -->
```

**JSDoc:**

```markdown
<!-- {=docs|trim|indent:" * ":true} -->
<!-- {/docs} -->
```

**Go comments:**

```markdown
<!-- {=docs|trim|linePrefix:"// ":true} -->
<!-- {/docs} -->
```

**Python `#` comments:**

```markdown
<!-- {=docs|trim|linePrefix:"# "} -->
<!-- {/docs} -->
```

**Fenced code block:**

```markdown
<!-- {=example|trim|codeBlock:"typescript"} -->
<!-- {/example} -->
```

## Data Interpolation

Provider content supports [minijinja](https://docs.rs/minijinja) template variables populated from project files.

### Configuration (`mdt.toml`)

```toml
[data]
pkg = "package.json"
cargo = "Cargo.toml"
config = "config.yaml"
release = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

### Usage in providers

```markdown
<!-- {@install} -->
Install `{{ pkg.name }}` version {{ pkg.version }}:
```sh
npm install {{ pkg.name }}@{{ pkg.version }}
```
<!-- {/install} -->
```

### Supported formats

| Format | Extensions |
|--------|-----------|
| JSON | `.json` |
| TOML | `.toml` |
| YAML | `.yaml`, `.yml` |
| KDL | `.kdl` |
| INI | `.ini` |
| Text | `.txt` (raw string) |

### Template features (minijinja)

```
{{ namespace.key }}                    — Variable
{{ namespace.key | upper }}            — Built-in filter
{% if pkg.private %}...{% endif %}     — Conditional
{% for f in config.features %}...{% endfor %}  — Loop
```

Undefined variables render as empty strings. Template rendering happens **before** transformers are applied.

### Script data sources

```toml
[data]
release = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

- `command` runs from the project root.
- `watch` files control cache invalidation.
- Cached in `.mdt/cache/data-v1.json` when watch files are unchanged.

## Inline Blocks

Inline blocks render a template expression without a separate provider. Useful for single values like versions.

```markdown
Install version <!-- {~v:"{{ pkg.version }}"} -->0.0.0<!-- {/v} --> today.
```

In tables:

```markdown
| Package | Version |
|---------|---------|
| mdt | <!-- {~ver:"{{ pkg.version }}"} -->0.0.0<!-- {/ver} --> |
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
before = 0   # 0 = next line, 1 = one blank line, false = inline
after = 0

[exclude]
patterns = ["vendor/", "dist/"]
blocks = ["draft-section"]
markdown_codeblocks = true   # or "ignore" or ["ignore", "example"]

[include]
patterns = ["src/**", "docs/**"]

[templates]
paths = [".templates"]
```

### `[padding]` — Required for source file consumers

Controls blank lines between tags and content. **Always set this when using consumers in source files** to prevent content from merging with tags.

- `false` — Content inline with tag
- `0` — Content on next line (recommended with formatters)
- `1` — One blank line (default when section present but values omitted)

### Sub-project boundaries

A directory with its own `mdt.toml` is treated as a separate mdt project. The parent project's scan skips it.

## Source File Support

Consumer tags work inside code comments in any supported language.

| Language | Extensions |
|----------|-----------|
| Rust | `.rs` |
| TypeScript | `.ts`, `.tsx` |
| JavaScript | `.js`, `.jsx` |
| Python | `.py` |
| Go | `.go` |
| Java | `.java` |
| Kotlin | `.kt` |
| Swift | `.swift` |
| C/C++ | `.c`, `.cpp`, `.h` |
| C# | `.cs` |

**Important:**

- Source files can only contain **consumer** and **inline** blocks, never providers.
- Parsing is lenient: unclosed tags are silently ignored.
- Use `[padding]` to prevent content merging with tags.

## CLI Commands

| Command | Purpose |
|---------|---------|
| `mdt init` | Create starter `.templates/template.t.md` and `mdt.toml` |
| `mdt check [--diff] [--watch]` | Verify consumers are current. Non-zero exit on stale. |
| `mdt update [--dry-run] [--watch]` | Sync all consumers with provider content |
| `mdt list` | List all providers and consumers with status |
| `mdt info [--format json]` | Project diagnostics and cache telemetry |
| `mdt doctor [--format json]` | Health checks with actionable hints |
| `mdt assist <assistant>` | Print MCP config and setup guidance |
| `mdt lsp` | Start the Language Server Protocol server |
| `mdt mcp` | Start the Model Context Protocol server |

### Common flags

- `--path <dir>` — Project root (default: current directory)
- `--verbose` — Show detailed output
- `--no-color` — Disable colored output

## MCP Server Tools

The MCP server (`mdt mcp`) exposes these tools to AI assistants:

| Tool | Description |
|------|-------------|
| `mdt_init` | Initialize a new mdt project |
| `mdt_check` | Verify all consumer blocks are up-to-date (returns structured JSON) |
| `mdt_update` | Update all consumer blocks (supports `dry_run`) |
| `mdt_list` | List all providers and consumers with file locations |
| `mdt_find_reuse` | Find similar providers and reuse opportunities |
| `mdt_get_block` | Get a specific block's content by name |
| `mdt_preview` | Preview rendered provider + consumer output with transformers |

### Agent best practices

1. **Reuse first** — Always call `mdt_find_reuse` before creating a new provider block.
2. **Preview before sync** — Use `mdt_preview` to inspect rendered output before running `mdt_update`.
3. **Check after edits** — Call `mdt_check` after any documentation change.
4. **JSON responses** — All MCP tool responses are structured JSON. Parse them directly.
5. **Unique names** — Provider names must be globally unique across all `*.t.md` files.
6. **Canonical layout** — Use `.templates/` as the template directory.

## File Conventions

| Pattern | Role |
|---------|------|
| `*.t.md` | Template files — only these contain provider blocks |
| `*.md`, `*.mdx`, `*.markdown` | Markdown files — scanned for consumer and inline blocks |
| `*.rs`, `*.ts`, `*.py`, etc. | Source files — scanned for consumer and inline blocks in comments |
| `mdt.toml` / `.mdt.toml` / `.config/mdt.toml` | Configuration file |
| `.mdt/cache/` | Cache directory (auto-managed) |
| `.templates/` | Canonical template directory |

## Skipped by default

- Hidden directories (`.git`, `.vscode`, etc.)
- `node_modules/`
- `target/` (Rust build output)
- Directories with their own `mdt.toml` (sub-projects)
- Files matching `.gitignore` rules (unless `disable_gitignore = true`)
