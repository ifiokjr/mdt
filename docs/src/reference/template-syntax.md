# Template Syntax Reference

All mdt tags are HTML comments. They are invisible when markdown is rendered.

## Tag types

### Provider tag

Defines a named block of content in a template file (`*.t.md`).

```
<!-- {@blockName} -->
```

- Sigil: `@`
- Only recognized in `*.t.md` files.
- The content between the opening and closing tags becomes the provider's content.

### Consumer tag

Marks where provider content should be injected.

```
<!-- {=blockName} -->
<!-- {=blockName|transformer1|transformer2:"arg"} -->
```

- Sigil: `=`
- Recognized in any scanned file (markdown or source code).
- Optionally includes transformers after the block name.

### Inline tag

<!-- {=mdtInlineBlocksGuide} -->

Inline blocks are useful when you need dynamic content in-place without creating a separate provider. Typical examples include versions, toolchain values, environment metadata, and short computed strings.

Inline blocks render minijinja template content from the block's first argument:

```markdown
<!-- {~version:"{{ pkg.version }}"} -->0.0.0<!-- {/version} -->
```

During `mdt update`, mdt evaluates the template argument with your configured `[data]` context, then replaces the content between the opening and closing tags.

Because inline blocks are provider-free, they are ideal for one-off values that still need to stay synchronized.

<!-- {/mdtInlineBlocksGuide} -->

#### Current limits

<!-- {=mdtInlineBlocksLimits} -->

- Inline blocks must include a first argument that is the template string to render.
- Inline blocks do not resolve provider content; everything comes from the inline template argument and current data context.
- Inline rendering still supports transformers (`|trim`, `|code`, etc.) after template evaluation.
- Inline blocks are scanned where mdt scans HTML comment tags (markdown and supported source comments), and follow the same code-block filtering rules configured for source scanning.

<!-- {/mdtInlineBlocksLimits} -->

### Close tag

Closes provider, consumer, and inline blocks.

```
<!-- {/blockName} -->
```

- Sigil: `/`
- The name must match the opening tag.

## Block names

Block names follow identifier rules:

- Start with a letter or underscore
- Followed by letters, digits, or underscores
- Case-sensitive

Valid names: `install`, `apiDocs`, `my_block`, `block123`, `_private`

## Transformer syntax

Transformers are pipe-delimited and follow the block name:

```
{=name|transformer1|transformer2:"arg1":"arg2"}
{~name:"{{ value }}"|transformer1|transformer2:"arg1":"arg2"}
```

### Structure

```
|transformerName           — no arguments
|transformerName:"arg"     — one string argument
|transformerName:4         — one numeric argument
|transformerName:"a":"b"   — two arguments
```

### Argument types

| Type    | Syntax                    | Example                     |
| ------- | ------------------------- | --------------------------- |
| String  | Double-quoted             | `"hello"`, `"/// "`, `"\n"` |
| Number  | Unquoted integer or float | `4`, `2.5`                  |
| Boolean | `true` or `false`         | `true`                      |

String arguments support escape sequences: `\"`, `\\`, `\n`, `\t`.

## Whitespace handling

Whitespace between the comment delimiters and the tag braces is allowed:

```markdown
<!--  { @blockName }  -->
```

Newlines within the comment are also allowed:

```markdown
<!--
{/blockName}
-->
```

## Content boundaries

The content of a block is everything between the **end** of the opening tag and the **start** of the closing tag. This includes surrounding whitespace and newlines:

```markdown
<!-- {@block} -->

This content includes the newlines above and below.

<!-- {/block} -->
```

The provider content here is `\nThis content includes the newlines above and below.\n\n` — note the leading newline after the opening tag and the trailing newline before the closing tag. Use the `trim` transformer on consumers if you want to strip this whitespace.

## Template variables

Inside provider blocks, minijinja template syntax is available when data files are configured:

### Variable output

```
{{ namespace.key }}
{{ namespace.nested.value }}
```

### Control flow

```
{% if condition %}...{% endif %}
{% if condition %}...{% else %}...{% endif %}
{% for item in list %}...{% endfor %}
```

### Comments

```
{# This is a template comment and won't appear in output #}
```

Template variables are rendered before transformers are applied.

## Examples

### Minimal

```markdown
<!-- {@greeting} -->

Hello!

<!-- {/greeting} -->
```

### With transformers

```markdown
<!-- {=docs|trim|linePrefix:"/// "} -->

Old content.

<!-- {/docs} -->
```

### With template variables

```markdown
<!-- {@version} -->

Current version: {{ package.version }}

<!-- {/version} -->
```

### Complex chain

```markdown
<!-- {=apiDocs|trim|replace:"Example":"Usage"|codeBlock:"typescript"} -->
<!-- {/apiDocs} -->
```
