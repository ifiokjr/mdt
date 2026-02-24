---
mdt_core: minor
mdt_cli: minor
---

Add comprehensive validation diagnostics with file location reporting.

**`mdt_core` changes:**

- Add `ProjectDiagnostic` and `DiagnosticKind` types for reporting validation issues during project scanning, including unclosed blocks, unknown transformers, invalid transformer arguments, and unused providers.
- Add `ValidationOptions` struct to control which diagnostics are treated as errors vs warnings.
- Add `parse_with_diagnostics()` function that collects parse issues as diagnostics instead of hard-erroring, enabling lenient parsing for editor tooling and better error reporting.
- Add `parse_source_with_diagnostics()` for source file scanning with diagnostic collection.
- Add `line` and `column` fields to `StaleEntry` for precise location reporting in check results.
- Project scanning now collects diagnostics for all validation issues and attaches file/line/column context.

**`mdt_cli` changes:**

- Add `--ignore-unclosed-blocks` flag to suppress unclosed block errors during validation.
- Add `--ignore-unused-blocks` flag to suppress warnings about providers with no consumers.
- Add `--ignore-invalid-names` flag to suppress invalid block name errors.
- Add `--ignore-invalid-transformers` flag to suppress unknown transformer and invalid argument errors.
- Error and check output now includes `file:line:column` location information.
- JSON check output now includes `line` and `column` fields in stale entries.
- GitHub Actions annotation format now includes `line` and `col` parameters.
