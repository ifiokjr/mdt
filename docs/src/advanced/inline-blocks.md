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
- Inline blocks are scanned where mdt scans HTML comment tags (markdown and supported source comments), and follow the same code-block filtering rules configured for source scanning.

<!-- {/mdtInlineBlocksLimits} -->

## Comparison to providers

- Use `{@name} ... {/name}` when the same content should be reused in many places.
- Use `{~name:"..."} ... {/name}` when you need localized dynamic output without a dedicated provider block.
