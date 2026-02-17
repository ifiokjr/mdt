---
mdt: minor
mdt_cli: minor
---

Add config file support (`mdt.toml`), minijinja template rendering, and source file scanning.

**Config file (`mdt.toml`):** Map data files to namespaces under `[data]`. Supports JSON, TOML, YAML, and KDL data sources.

**Template variables:** Provider blocks can use `{{ namespace.key }}` syntax (powered by minijinja) to interpolate data from configured files.

**Source file scanning:** Consumer blocks are now detected in source code comments (`.ts`, `.rs`, `.py`, `.go`, `.java`, etc.), not just markdown files.
