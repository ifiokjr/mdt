<!-- {@mdtPackageDocumentation} -->

`mdt` helps library and tool maintainers keep README sections, source-doc comments, and docs-site content in sync across a project. Define content once with comment-based template tags, then reuse it across markdown files, code documentation comments, READMEs, mdbook docs, and more, so your docs do not drift.

<!-- {/mdtPackageDocumentation} -->

<!-- {@mdtCliUsage} -->

### CLI Commands

- `mdt init` — Set up a project, adding only what is missing: an annotated `mdt.toml`, a sample provider in `.templates/template.t.md`, a synced `readme.md` when the project has no README, and a `.mdt/` entry in `.gitignore` in git repositories.
- `mdt check [--diff] [--format text|json|github] [--watch]` — Verify every consumer is linked to a provider and up to date. Exits 0 when in sync, 1 for stale or orphan consumers and render errors, 2 for validation or config errors.
- `mdt update [--dry-run] [--watch]` — Write the latest provider content into every consumer.
- `mdt list` — List providers and consumers with their locations and link status.
- `mdt info [--format text|json]` — Print a project summary, diagnostic totals, and cache metrics.
- `mdt doctor [--format text|json]` — Run health checks with fix hints. Exits 1 when a check fails.
- `mdt skill [--reference] [--install <DIR>]` — Print the agent skill for AI coding assistants, or install it into a skills directory.
- `mdt assist <generic|claude|cursor|copilot|pi> [--format text|json]` — Print MCP setup and skill guidance for an assistant.
- `mdt lsp` — Start the language server over stdin/stdout.
- `mdt mcp` — Start the MCP server over stdin/stdout.

### Global Options

- `-p, --path <DIR>` — Project root (default: the current directory). Must exist, except for `mdt init`.
- `-v, --verbose` — Print more detail, including warnings silenced by `--ignore-*` flags.
- `--no-color` — Disable colored output (`NO_COLOR` works too).
- `--ignore-unclosed-blocks`, `--ignore-unused-blocks`, `--ignore-invalid-names`, `--ignore-invalid-transformers` — Skip one class of diagnostics.

### Diagnostics Workflow

- Run `mdt info` to inspect project shape, diagnostic totals, and cache reuse.
- Run `mdt doctor` for health checks with remediation hints (config, data, layout, sync, cache).
- Set `MDT_CACHE_VERIFY_HASH=1` when troubleshooting cache consistency, and `MDT_LOG=debug` for debug logs on stderr.

<!-- {/mdtCliUsage} -->

<!-- {@mdtTemplateSyntax} -->

### Template Syntax

**Provider** (defines content; only recognized in `*.t.md` files):

```markdown
<!-- {@blockName} -->

Content to inject

<!-- {/blockName} -->
```

**Consumer** (its content is replaced by `mdt update`):

```markdown
<!-- {=blockName} -->

This content gets replaced

<!-- {/blockName} -->
```

**Inline block** (renders its template argument in place; needs `[data]` in `mdt.toml`):

```markdown
Current version: <!-- {~version:"{{ "{{" }} package.version {{ "}}" }}"} -->0.0.0<!-- {/version} -->
```

```markdown
| Artifact | Version                                                                                   |
| -------- | ----------------------------------------------------------------------------------------- |
| mdt_cli  | <!-- {~cliVersion:"{{ "{{" }} package.version {{ "}}" }}"} -->0.0.0<!-- {/cliVersion} --> |
```

**Transformers** change the content on its way into a consumer, applied left to right:

```markdown
<!-- {=blockName|trim|linePrefix:"/// ":true} -->
```

Available transformers: `trim`, `trimStart`, `trimEnd`, `indent`, `prefix`, `suffix`, `linePrefix`, `lineSuffix`, `wrap`, `codeBlock`, `code`, `replace`, `if`.

<!-- {/mdtTemplateSyntax} -->

<!-- {@blockName} -->

Content to inject

<!-- {/blockName} -->

<!-- {=blockName} -->

Content to inject

<!-- {/blockName} -->

<!-- {@mdtInlineBlocksGuide} -->

Inline blocks render a small template in place, with no provider. Use them for short values such as version numbers, toolchain versions, or other metadata from your `[data]` sources.

The block's first argument is a minijinja template:

```markdown
<!-- {~version:"{{ "{{" }} pkg.version {{ "}}" }}"} -->0.0.0<!-- {/version} -->
```

`mdt update` renders the argument with your `[data]` context and replaces the text between the opening and closing tags. `mdt check` reports the block as stale when that text is out of date.

<!-- {/mdtInlineBlocksGuide} -->

<!-- {@mdtInlineBlocksLimits} -->

- An inline block needs a first argument: the template string to render.
- Inline blocks do not read a provider; everything comes from the template argument and the `[data]` context. Without `[data]`, the argument is written as-is.
- Transformers (`|trim`, `|code`, and so on) run after the template is rendered.
- Padding never applies to inline blocks, so they stay on one line.
- In markdown, inline blocks work in paragraphs, lists, headings, and table cells. In table cells, do not add transformers: GFM splits cells on `|`, so the tag is not recognized.
- Tags inside fenced code blocks and inline code spans in markdown are examples, not live blocks.
- In source files, inline tags follow the source scanning rules, including `[exclude] markdown_codeblocks`.

<!-- {/mdtInlineBlocksLimits} -->

<!-- {@mdtInlineBlocksExamples} -->

### Inline value in prose

```markdown
Install version <!-- {~releaseVersion:"{{ "{{" }} pkg.version {{ "}}" }}"} -->0.0.0<!-- {/releaseVersion} --> today.
```

### Inline value in a table cell

```markdown
| Package | Version                                                                               |
| ------- | ------------------------------------------------------------------------------------- |
| mdt     | <!-- {~mdtVersion:"{{ "{{" }} pkg.version {{ "}}" }}"} -->0.0.0<!-- {/mdtVersion} --> |
```

### Inline value with a transformer

```markdown
CLI version: <!-- {~cliVersionCode:"{{ "{{" }} pkg.version {{ "}}" }}"|code} -->`0.0.0`<!-- {/cliVersionCode} -->
```

### Inline value from a script-backed data source

```toml
[data]
release = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

```markdown
Release: <!-- {~releaseValue:"{{ "{{" }} release {{ "}}" }}"} -->0.0.0<!-- {/releaseValue} -->
```

The text format drops the file's trailing newline, so the value stays on one line. While `VERSION` is unchanged, mdt reuses the cached output in `.mdt/cache/data-v1.json`.

<!-- {/mdtInlineBlocksExamples} -->
