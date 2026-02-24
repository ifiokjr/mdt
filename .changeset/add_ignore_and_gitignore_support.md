---
mdt_core: major
---

Add file ignore support with `.gitignore` integration.

**New `[ignore]` config section:** The `mdt.toml` configuration file now supports an `[ignore]` section with gitignore-style patterns for skipping files and directories during scanning. These patterns follow `.gitignore` syntax and are applied on top of any `.gitignore` rules.

```toml
[ignore]
patterns = ["build/", "dist/", "*.generated.md"]
```

**`.gitignore` integration:** By default, mdt now respects `.gitignore` files in the project root. Files that would be ignored by git are automatically skipped during scanning. This eliminates the need to manually exclude common build artifacts, dependencies, and other generated files.

**`disable_gitignore` setting:** A new top-level `disable_gitignore` boolean setting disables `.gitignore` integration when set to `true`. This is useful when working outside a git repository or when full control over file filtering is needed.

```toml
disable_gitignore = true

[ignore]
patterns = ["build/", "dist/"]
```

**Breaking change:** The `scan_project_with_options()` function signature now requires two additional parameters: `ignore_patterns: &[String]` and `disable_gitignore: bool`. The `MdtConfig` struct has two new fields: `ignore: IgnoreConfig` and `disable_gitignore: bool`.
