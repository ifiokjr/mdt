---
mdt_lsp: minor
---

Enhanced LSP diagnostics to surface all errors that the CLI `check` command reports. The language server now detects:

- **Unclosed blocks**: Opening tags without matching close tags are reported as errors with the block name and position.
- **Unknown transformers**: Invalid transformer names (e.g., `|foobar`) are reported as errors.
- **Invalid transformer arguments**: Transformers with the wrong number of arguments are reported as errors.
- **Unused providers**: Provider blocks in template files that have no matching consumers are reported as warnings.
- **Name suggestions**: When a consumer references a missing provider, the LSP now suggests similar provider names using Levenshtein distance matching (e.g., "Did you mean: `greeting`?").

The parser was upgraded from `parse()` to `parse_with_diagnostics()` to capture parse-level diagnostics that were previously silently discarded.

Added `cargo-semver-checks` CI job to pull requests that detects breaking API changes in published crates and enforces that a `major` changeset is included when breakage is found. The job posts a PR comment with the semver-checks output on failure.
