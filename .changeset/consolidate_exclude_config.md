---
mdt: minor
mdt_cli: minor
---

Consolidate `[ignore]` into `[exclude]` and add new exclusion options.

**Breaking:** The `[ignore]` config section has been removed. Its functionality is now part of `[exclude]`, which uses gitignore-style patterns (supporting negation `!`, directory markers `/`, and all standard gitignore wildcards). Existing `[ignore]` patterns should be moved to `[exclude] patterns`.

**New `[exclude]` sub-properties:**

- `markdown_codeblocks`: Controls whether mdt tags inside fenced code blocks in source files are processed. Can be set to `true` (skip all code blocks), a string like `"ignore"` (skip code blocks whose info string contains the string), or an array of strings (skip code blocks matching any). Defaults to `false`.

- `blocks`: An array of block names to exclude from processing. Any provider or consumer block whose name appears in this list is completely ignored during scanning — it won't be matched, checked, or updated.

**DevEnv integration:** Added `mdt check --ignore-unused-blocks` to the `lint:all` command and `mdt update --ignore-unused-blocks` to the `fix:all` command in `devenv.nix`.
