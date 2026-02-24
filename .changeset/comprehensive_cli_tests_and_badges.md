---
mdt_cli: patch
---

Add comprehensive CLI integration tests covering all commands and features.

**New tests (28 added, 47 total):**

- `mdt init` — fresh directory creation and existing template detection
- `mdt list` — block listing with provider/consumer counts, empty project, verbose output
- `mdt check --format json` — JSON output for stale and up-to-date states
- `mdt check --format github` — GitHub Actions annotation format
- `mdt check --diff` — unified diff output for stale blocks
- `mdt update --verbose` — verbose output with provider listing and updated file paths
- `--ignore-unused-blocks` — suppresses unused provider diagnostics
- `--ignore-invalid-transformers` — suppresses unknown transformer errors
- `--ignore-unclosed-blocks` — suppresses unclosed block errors
- Missing provider warnings — consumers referencing non-existent providers
- Multiple providers — multiple blocks consumed across multiple files
- Empty project — no providers or consumers
- No subcommand — error message when running `mdt` without a command

**Bug fixes:**

- Sort provider names in verbose output for deterministic ordering

**Snapshot stability:**

- Add path redaction (`[TEMP_DIR]`) to all snapshots containing absolute paths, ensuring reproducibility across machines
- Enable `insta` `filters` feature for regex-based path filtering
