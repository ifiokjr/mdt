# Template Syntax Reference

Every mdt tag is an HTML comment, so tags are invisible in rendered markdown. A block is an opening tag, some content, and a closing tag with the same name.

## Tag types

### Provider tag

Defines a named block of content. Sigil `@`.

```markdown
<!-- {@blockName} -->

Content to share.

<!-- {/blockName} -->
```

Providers are read only from `*.t.md` template files. A provider tag anywhere else is ignored and reported as a `mdt::provider_outside_template` warning.

### Consumer tag

Marks where a provider's content is written. Sigil `=`. Optional [transformers](#transformer-syntax) follow the name.

```markdown
<!-- {=blockName} -->
<!-- {/blockName} -->

<!-- {=blockName|trim|codeBlock:"sh"} -->
<!-- {/blockName} -->
```

Consumers work in any scanned file: markdown and [source files](../guide/source-files.md). A consumer whose name matches no provider is an orphan: `mdt check` fails and `mdt update` warns.

### Inline tag

<!-- {=mdtInlineBlocksGuide} -->

Inline blocks render a small template in place, with no provider. Use them for short values such as version numbers, toolchain versions, or other metadata from your `[data]` sources.

The block's first argument is a minijinja template:

```markdown
<!-- {~version:"{{ pkg.version }}"} -->0.0.0<!-- {/version} -->
```

`mdt update` renders the argument with your `[data]` context and replaces the text between the opening and closing tags. `mdt check` reports the block as stale when that text is out of date.

<!-- {/mdtInlineBlocksGuide} -->

#### Current limits

<!-- {=mdtInlineBlocksLimits} -->

- An inline block needs a first argument: the template string to render.
- Inline blocks do not read a provider; everything comes from the template argument and the `[data]` context. Without `[data]`, the argument is written as-is.
- Transformers (`|trim`, `|code`, and so on) run after the template is rendered.
- Padding never applies to inline blocks, so they stay on one line.
- In markdown, inline blocks work in paragraphs, lists, headings, and table cells. In table cells, do not add transformers: GFM splits cells on `|`, so the tag is not recognized.
- Tags inside fenced code blocks and inline code spans in markdown are examples, not live blocks.
- In source files, inline tags follow the source scanning rules, including `[exclude] markdown_codeblocks`.

<!-- {/mdtInlineBlocksLimits} -->

#### Practical examples

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

### Close tag

Closes provider, consumer, and inline blocks. Sigil `/`. The name must match the opening tag.

```markdown
<!-- {/blockName} -->
```

## Block names

Names match `[A-Za-z_][A-Za-z0-9_-]*`: a letter or underscore, then letters, digits, underscores, or hyphens. Names are case-sensitive, so `apiDocs` and `ApiDocs` are different blocks.

Valid: `install`, `apiDocs`, `install-command`, `my_block`, `block123`, `_private`.

Invalid: `my.block`, `docs/intro`, `1starts`, names with spaces.

## Tag syntax

The sigil must come directly after `{`. Whitespace is allowed everywhere else: inside the comment delimiters, after the sigil, and around `|` and `:`. A tag may also span lines.

```markdown
<!-- {@ blockName } -->
<!-- {=blockName | trim | replace: "Hi" : "Hello"} -->
<!--
{/blockName}
-->
```

`<!-- { @blockName } -->` is not a tag. In markdown, a comment that starts like a tag (`{` followed by `@`, `=`, `~`, or `/`) but does not parse is a `mdt::invalid_tag` error that names the file and line. In source files it is ignored without a diagnostic.

## Transformer syntax

Transformers follow the block name, separated by `|`, and run left to right. Arguments follow the transformer name, separated by `:`.

```text
{=name|transformer}
{=name|transformer:"arg"}
{=name|transformer:"arg1":"arg2"}
{~name:"{{ value }}"|transformer:"arg"}
```

See the [transformer reference](transformers.md) for each transformer.

### Argument types

| Type          | Syntax          | Value passed to the transformer                                                           |
| ------------- | --------------- | ----------------------------------------------------------------------------------------- |
| Double-quoted | `"text"`        | Text with escapes decoded                                                                 |
| Single-quoted | `'text'`        | Text exactly as written; backslashes stay literal                                         |
| Number        | `4`, `2.5`      | Converted to text: `indent:4` prepends the character `4`, not spaces                      |
| Boolean       | `true`, `false` | A flag as the second argument of `indent`, `linePrefix`, `lineSuffix`; text anywhere else |

Double-quoted strings decode `\"`, `\\`, `\n`, `\t`, and `\u{...}` (for example `\u{2f}` for `/`). If a string contains an escape mdt does not recognize, such as `"\q"`, the whole argument is kept literally and none of its escapes are decoded.

## Content boundaries

A block's content is everything between the end of the opening tag and the start of the closing tag, including newlines.

```markdown
<!-- {@block} -->

This content includes the newlines above and below.

<!-- {/block} -->
```

The provider content here is `\n\nThis content includes the newlines above and below.\n\n`. Consumers receive those newlines too. Add `|trim` to a consumer to drop them, and see [`[padding]`](configuration.md#padding) for how mdt places content between the tags.

## Where tags are recognized

**Markdown files** (`.md`, `.mdx`, `.markdown`):

- Tags inside fenced code blocks and inline code spans are inert. That is how documentation shows tag examples, including this page.
- GFM tables split cells on `|`, so a tag with transformers inside a table cell is cut apart and not recognized; its closing tag is then a `mdt::unmatched_closing_tag` error. In table cells, use an inline block without transformers.

**Source files**: tags are found in any comment, and also inside string literals. See [Source File Support](../guide/source-files.md).

## Nesting

A consumer or inline block cannot contain another block, because `mdt update` replaces everything between its tags. A block opened inside one is a `mdt::nested_block` error.

Provider bodies are not checked for nesting, but a provider's content is copied into its consumers with any tags it contains. A block inside a provider therefore becomes a nested block in every consumer of that provider, and `mdt check` fails there. Keep blocks flat: put consumers one after another instead of inside each other.

## Diagnostics

| Code                             | Severity | Cause                                                                  | Silence with                    |
| -------------------------------- | -------- | ---------------------------------------------------------------------- | ------------------------------- |
| `mdt::unclosed_block`            | error    | An opening tag has no matching closing tag (markdown and source files) | `--ignore-unclosed-blocks`      |
| `mdt::nested_block`              | error    | A block opens inside a consumer or inline block                        | none                            |
| `mdt::invalid_tag`               | error    | Markdown only: a comment looks like a tag but does not parse           | `--ignore-invalid-names`        |
| `mdt::unknown_transformer`       | error    | A transformer name does not exist                                      | `--ignore-invalid-transformers` |
| `mdt::invalid_transformer_args`  | error    | A transformer got the wrong number of arguments                        | `--ignore-invalid-transformers` |
| `mdt::duplicate_provider`        | error    | Two providers share a name                                             | none                            |
| `mdt::unmatched_closing_tag`     | error    | A closing tag has no opening tag, usually a misspelled opening tag     | `--ignore-unclosed-blocks`      |
| `mdt::unused_provider`           | warning  | A provider has no consumers                                            | `--ignore-unused-blocks`        |
| `mdt::provider_outside_template` | warning  | A provider tag is outside a `*.t.md` file                              | none                            |

Errors stop `mdt check` and `mdt update` with exit code 2; `mdt list` still prints the listing, then exits 2. Warnings are printed but do not change the exit code. `--verbose` also shows diagnostics silenced by an `--ignore-*` flag. Names listed in `[exclude] blocks` are skipped entirely: their consumers are never filled or checked, and they produce no diagnostics.

Most diagnostics name the file and position, with a hint:

```text
mdt::unclosed_block

  x [readme.md:9:1] missing closing tag for block `dup`
  help: add `<!-- {/dup} -->` to close this block, or, if a nearby closing tag
        has a different name, fix the misspelled tag so the names match
```

## Template variables

When `[data]` is configured in `mdt.toml`, provider content is rendered with [minijinja](https://docs.rs/minijinja) before transformers run:

```text
{{ package.version }}
{% if package.private %}...{% endif %}
{% for item in list %}...{% endfor %}
{# a comment that is not rendered #}
```

Without `[data]`, provider content is copied as written, and `check` and `update` warn when a used provider contains a variable such as `{{ pkg.version }}`. See [Data Interpolation](../guide/data-interpolation.md).
