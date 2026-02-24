---
mdt_core: docs
mdt_cli: docs
---

Add comprehensive doc comments to `mdt_core` public API types and enrich CLI help text for all `mdt_cli` commands.

**mdt_core:**

- Expand crate-level documentation with processing pipeline diagram, module overview, key types reference, data interpolation guide, and quick start code example.
- Add struct/enum-level doc comments for `Block`, `Transformer`, and `Argument` explaining their role in the template system.
- Add field-level doc comments for `Block`, `Transformer`, `Argument`, and `StaleEntry` fields.
- Add doc comments for internal types `TokenGroup`, `DynamicRange`, and `GetDynamicRange` in the tokens module.
- Add provider blocks to `template.t.md` for `mdtCoreOverview`, `mdtBlockDocs`, `mdtTransformerDocs`, and `mdtArgumentDocs` for potential use by markdown consumers.

**mdt_cli:**

- Expand `Init` help with details about what file is created and no-op behavior.
- Expand `Check` help with CI usage guidance, `--diff` and `--format` tips.
- Expand `Update` help with template rendering flow, `--dry-run` and `--watch` details.
- Expand `List` help with output format description.
- Expand `Lsp` help with diagnostics and auto-completion features.
- Expand `Mcp` help with available tools description.
- Enrich `OutputFormat` variant docs and field-level docs for `Check` and `Update` args.
