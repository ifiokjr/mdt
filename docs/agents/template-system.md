# Template system

## Provider and consumer tags

### Provider block

```md
<!-- {@blockName} -->

Content to inject

<!-- {/blockName} -->
```

### Consumer block

```md
<!-- {=blockName} -->

This content gets replaced

<!-- {/blockName} -->
```

### Shared close tag

```md
<!-- {/blockName} -->
```

## Transformers

Supported transformers:

- `trim`
- `trimStart`
- `trimEnd`
- `indent`
- `prefix`
- `suffix`
- `linePrefix`
- `lineSuffix`
- `wrap`
- `codeBlock`
- `code`
- `replace`
- `if`

Example:

```md
<!-- {=block|prefix:"\n"|indent:"//! "} -->
```

## File conventions

- Use `*.t.md` for template definition files.
- Providers are only recognized in `*.t.md` files.
- Other `.md`, `.mdx`, and `.markdown` files may contain consumer and inline blocks.
- Supported source files may contain consumer and inline blocks inside comments.

## Data interpolation

Use `mdt.toml` to map data files into template namespaces:

```toml
[data]
pkg = "package.json"
cargo = "Cargo.toml"
```

Then reference values like:

- `{{ pkg.version }}`
- `{{ cargo.package.edition }}`

Supported data formats:

- JSON
- TOML
- YAML
- KDL
- INI

## Padding

Without a `[padding]` section, content starts on the line after the opening tag and the closing tag starts on its own line (`before = 0`, `after = 0`). Set `[padding]` only to add blank lines; values add to the content's own newlines, so use `|trim` for exact control.

## Agent skill

The user-facing agent skill lives in `packages/m-d-t__skills/skills/mdt/` and ships inside the CLI (`mdt skill`, `mdt skill --reference`, `mdt skill --install <dir>`) from the copy in `mdt_cli/skill/`. Run `fix:skill` after editing it.

## Cache diagnostics

- `mdt info` reports cache diagnostics and observability data.
- `mdt doctor` reports cache health checks and troubleshooting hints.
- Set `MDT_CACHE_VERIFY_HASH=1` when troubleshooting cache consistency.
