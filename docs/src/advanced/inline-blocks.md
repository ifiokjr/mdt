# Inline Blocks

Inline blocks add provider-free interpolation for small dynamic values that still need to stay synchronized.

## Why this exists

<!-- {=mdtInlineBlocksGuide} -->

Inline blocks are useful when you need dynamic content in-place without creating a separate provider. Typical examples include versions, toolchain values, environment metadata, and short computed strings.

Inline blocks render minijinja template content from the block's first argument:

```markdown
<!-- {~version:"{{ pkg.version }}"} -->0.0.0<!-- {/version} -->
```

During `mdt update`, mdt evaluates the template argument with your configured `[data]` context, then replaces the content between the opening and closing tags.

Because inline blocks are provider-free, they are ideal for one-off values that still need to stay synchronized.

<!-- {/mdtInlineBlocksGuide} -->

## Limits and behavior

<!-- {=mdtInlineBlocksLimits} -->

- Inline blocks must include a first argument that is the template string to render.
- Inline blocks do not resolve provider content; everything comes from the inline template argument and current data context.
- Inline rendering still supports transformers (`|trim`, `|code`, etc.) after template evaluation.
- In markdown, inline blocks work in normal content (paragraphs, lists, headings, tables) where HTML comments are parsed.
- Tags shown inside fenced markdown code blocks are treated as examples and are not interpreted as live blocks.
- In source files, inline tags follow source scanning rules and respect `[exclude] markdown_codeblocks` filtering.

<!-- {/mdtInlineBlocksLimits} -->

## Practical examples

<!-- {=mdtInlineBlocksExamples} -->

### Inline value in prose

```markdown
Install version <!-- {~releaseVersion:"{{ pkg.version }}"} -->0.0.0<!-- {/releaseVersion} --> today.
```

### Inline value in a table cell

```markdown
| Package | Version |
| ------- | ------- |
| mdt     | <!-- {~mdtVersion:"{{ pkg.version }}"} -->0.0.0<!-- {/mdtVersion} --> |
```

### Inline value with a transformer

```markdown
CLI version: <!-- {~cliVersionCode:"{{ pkg.version }}"|code} -->`0.0.0`<!-- {/cliVersionCode} -->
```

### Inline value from a script-backed data source

```toml
[data]
release = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

```markdown
Release: <!-- {~releaseValue:"{{ release }}"} -->0.0.0<!-- {/releaseValue} -->
```

When `VERSION` is unchanged, mdt reuses cached script output from `.mdt/cache/data-v1.json`.

<!-- {/mdtInlineBlocksExamples} -->

## Comparison to providers

- Use `{@name} ... {/name}` when the same content should be reused in many places.
- Use `{~name:"..."} ... {/name}` when you need localized dynamic output without a dedicated provider block.
