---
mdt_core: major
---

# Add formatter-aware full-file normalization

`mdt_core` now supports opt-in `[[formatters]]` configuration in `mdt.toml`. Matching formatter commands run against the entire updated target file in declaration order using stdin/stdout, enabling formatter-aware drift detection and update output.

This is a major release because public constructible structs such as `MdtConfig` and `ProjectContext` gained fields. Downstream crates that build these structs with literals must add the new fields or use defaults/builders where available.

```rust
let config = MdtConfig {
    formatters: Vec::new(),
    ..existing_config
};
```

Formatter failures now surface as dedicated diagnostics so callers can distinguish rendering problems from formatter command failures.
