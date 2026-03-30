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

Example:

```md
<!-- {=block|prefix:"\n"|indent:"//! "} -->
```

## File conventions

- Use `*.t.md` for template definition files.
- Providers are only recognized in `*.t.md` files.
- Other `.md`, `.mdx`, and `.markdown` files may contain consumer blocks.
- Supported source files may contain consumer blocks inside comments.

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

## Block padding for source files

When using consumer blocks in source files, prefer explicit padding in `mdt.toml`:

```toml
[padding]
before = 0
after = 0
```

Use this to avoid formatter-induced mangling when line-based transformers are involved.

## Cache diagnostics

- `mdt info` reports cache diagnostics and observability data.
- `mdt doctor` reports cache health checks and troubleshooting hints.
- Set `MDT_CACHE_VERIFY_HASH=1` when troubleshooting cache consistency.
