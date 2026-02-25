---
mdt_core: major
mdt_lsp: minor
mdt_mcp: minor
---

Add positional block arguments to provider and consumer tags.

Provider blocks can now declare named parameters using `:"param_name"` syntax after the block name. Consumer blocks pass string values as positional arguments in the same position. The provider's parameter names become template variables that are interpolated with the consumer's argument values during rendering.

**Syntax:**

```markdown
<!-- Provider declares a parameter -->
<!-- {@badges:"crate_name"} -->

[![crates.io](https://img.shields.io/crates/v/{{ crate_name }})]

<!-- {/badges} -->

<!-- Consumer passes a value -->
<!-- {=badges:"mdt_core"} -->
<!-- {/badges} -->

<!-- Another consumer with different value -->
<!-- {=badges:"mdt_cli"} -->
<!-- {/badges} -->
```

Arguments work alongside existing features:

- Multiple arguments: `<!-- {@tmpl:"a":"b":"c"} -->`
- With transformers: `<!-- {=badges:"mdt_core"|trim} -->`
- With data interpolation: `{{ crate_name }}` and `{{ pkg.version }}` can coexist
- Single-quoted strings: `<!-- {@tmpl:'param'} -->`

Argument count mismatches between provider parameters and consumer arguments are reported as render errors during `check` and skipped during `update`.

This is a breaking change because the `Block` struct now includes an `arguments: Vec<String>` field.
