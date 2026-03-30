# Architecture

## Workspace crates

- `mdt_core` — lexer, parser, pattern matcher, project scanner, source-file scanner, config loader, and template engine
- `mdt_cli` — CLI entrypoints such as `init`, `check`, `update`, `info`, `doctor`, `lsp`, and `mcp`
- `mdt_lsp` — language server support
- `mdt_mcp` — MCP server support for AI integrations
- `docs/` — mdBook documentation

## Core concepts

1. Provider blocks define reusable content in `*.t.md` files.
2. Consumer blocks reference provider content and are updated by `mdt update`.
3. Provider content can interpolate project data with `minijinja`.
4. Consumers can appear in markdown files and source-code comments.
5. Transformers modify injected content during rendering.

## Internal pipeline

```text
Markdown source
  → markdown AST
  → lexer
  → pattern matcher
  → parser
  → project scanner
  → engine
```

## Project scanning rules

- Providers are recognized only in `*.t.md` files.
- Consumer blocks are scanned in markdown files and supported source files.
- Hidden directories, `node_modules`, and `target` are skipped.
- Subdirectories with their own `mdt.toml` become separate project scopes.
