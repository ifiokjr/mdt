<!-- {@mdtPackageDocumentation} -->

`mdt` helps library and tool maintainers keep README sections, source-doc comments, and docs-site content synchronized across a project. Define content once with comment-based template tags, then reuse it across markdown files, code documentation comments, READMEs, mdbook docs, and more so your docs do not drift.

<!-- {/mdtPackageDocumentation} -->

<!-- {@mdtCliUsage} -->

### CLI Commands

- `mdt init [--path <dir>]` — Create a sample `.templates/template.t.md` file and starter `mdt.toml`.
- `mdt check [--path <dir>] [--verbose]` — Verify all target blocks are up-to-date. Exits non-zero if any are stale.
- `mdt update [--path <dir>] [--verbose] [--dry-run]` — Update all target blocks with latest source content.
- `mdt info [--path <dir>]` — Print project diagnostics and cache observability metrics.
- `mdt doctor [--path <dir>] [--format text|json]` — Run health checks with actionable hints, including cache validity and efficiency.
- `mdt assist <assistant> [--format text|json]` — Print an official assistant setup profile with MCP config and repo-local guidance.
- `mdt lsp` — Start the mdt language server (LSP) for editor integration. Communicates over stdin/stdout.
- `mdt mcp` — Start the mdt MCP server for AI assistants. Communicates over stdin/stdout.

### Diagnostics Workflow

- Run `mdt info` first to inspect project shape, diagnostics totals, and cache reuse telemetry.
- Run `mdt doctor` when you need actionable health checks and remediation hints (config/data/layout/cache).
- Use `MDT_CACHE_VERIFY_HASH=1` when troubleshooting cache consistency issues and comparing reuse behavior.

<!-- {/mdtCliUsage} -->

<!-- {@mdtTemplateSyntax} -->

### Template Syntax

**Source tag** (defines a template block in `*.t.md` definition files):

```markdown
<!-- {@blockName} -->

Content to inject

<!-- {/blockName} -->
```

**Target tag** (marks where content should be injected):

```markdown
<!-- {=blockName} -->

This content gets replaced

<!-- {/blockName} -->
```

**Inline tag** (source-free interpolation using configured data):

```markdown
Current version: <!-- {~version:"{{ "{{" }} package.version {{ "}}" }}"} -->0.0.0<!-- {/version} -->
```

```markdown
| Artifact | Version                                                                                   |
| -------- | ----------------------------------------------------------------------------------------- |
| mdt_cli  | <!-- {~cliVersion:"{{ "{{" }} package.version {{ "}}" }}"} -->0.0.0<!-- {/cliVersion} --> |
```

**Filters and pipes:** Template values support pipe-delimited transformers:

```markdown
<!-- {=block|prefix:"\n"|indent:"  "} -->
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

Inline blocks are useful when you need dynamic content in-place without creating a separate source. Typical examples include versions, toolchain values, environment metadata, and short computed strings.

Inline blocks render minijinja template content from the block's first argument:

```markdown
<!-- {~version:"{{ "{{" }} pkg.version {{ "}}" }}"} -->0.0.0<!-- {/version} -->
```

During `mdt update`, mdt evaluates the template argument with your configured `[data]` context, then replaces the content between the opening and closing tags.

Because inline blocks are source-free, they are ideal for one-off values that still need to stay synchronized.

<!-- {/mdtInlineBlocksGuide} -->

<!-- {@mdtInlineBlocksLimits} -->

- Inline blocks must include a first argument that is the template string to render.
- Inline blocks do not resolve source content; everything comes from the inline template argument and current data context.
- Inline rendering still supports transformers (`|trim`, `|code`, etc.) after template evaluation.
- In markdown, inline blocks work in normal content (paragraphs, lists, headings, tables) where HTML comments are parsed.
- Tags shown inside fenced markdown code blocks are treated as examples and are not interpreted as live blocks.
- In source files, inline tags follow source scanning rules and respect `[exclude] markdown_codeblocks` filtering.

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

When `VERSION` is unchanged, mdt reuses cached script output from `.mdt/cache/data-v1.json`.

<!-- {/mdtInlineBlocksExamples} -->
