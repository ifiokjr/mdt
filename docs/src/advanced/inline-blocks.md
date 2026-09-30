# Inline Blocks

Inline blocks render a small template in place, with no provider. They need a `[data]` section in `mdt.toml`; without one, the template argument is written as-is.

## Why this exists

<!-- {=mdtInlineBlocksGuide} -->

Inline blocks render a small template in place, with no provider. Use them for short values such as version numbers, toolchain versions, or other metadata from your `[data]` sources.

The block's first argument is a minijinja template:

```markdown
<!-- {~version:"{{ pkg.version }}"} -->0.0.0<!-- {/version} -->
```

`mdt update` renders the argument with your `[data]` context and replaces the text between the opening and closing tags. `mdt check` reports the block as stale when that text is out of date.

<!-- {/mdtInlineBlocksGuide} -->

## Limits and behavior

<!-- {=mdtInlineBlocksLimits} -->

- An inline block needs a first argument: the template string to render.
- Inline blocks do not read a provider; everything comes from the template argument and the `[data]` context. Without `[data]`, the argument is written as-is.
- Transformers (`|trim`, `|code`, and so on) run after the template is rendered.
- Padding never applies to inline blocks, so they stay on one line.
- In markdown, inline blocks work in paragraphs, lists, headings, and table cells. In table cells, do not add transformers: GFM splits cells on `|`, so the tag is not recognized.
- Tags inside fenced code blocks and inline code spans in markdown are examples, not live blocks.
- In source files, inline tags follow the source scanning rules, including `[exclude] markdown_codeblocks`.

<!-- {/mdtInlineBlocksLimits} -->

## Practical examples

<!-- {=mdtInlineBlocksExamples} -->

### Inline value in prose

```markdown
Install version <!-- {~releaseVersion:"{{ pkg.version }}"} -->0.0.0<!-- {/releaseVersion} --> today.
```

### Inline value in a table cell

```markdown
| Package | Version                                                               |
| ------- | --------------------------------------------------------------------- |
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

The text format drops the file's trailing newline, so the value stays on one line. While `VERSION` is unchanged, mdt reuses the cached output in `.mdt/cache/data-v1.json`.

<!-- {/mdtInlineBlocksExamples} -->

## Table cells

GFM splits table cells on `|`, so an inline tag with transformers inside a cell is cut apart and not recognized, and its closing tag becomes a `mdt::unmatched_closing_tag` error. In table cells, use inline blocks without transformers, as in the table example above.

## Inline blocks or providers?

- Use a provider (`{@name}` in a `*.t.md` file) and consumers (`{=name}`) when the same content appears in several places.
- Use an inline block (`{~name:"..."}`) for a one-off value computed from `[data]`.

An inline block without a template argument is a render error: `mdt update` skips it and exits with code 1.
