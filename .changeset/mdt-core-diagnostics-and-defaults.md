---
"mdt_core": change
---

# Catch silent failures: orphan consumers fail checks, malformed tags are reported, unknown config keys are rejected

Several mistakes used to pass `mdt check` silently. They are now reported, and the severities match how much damage each one can do.

- **Orphan consumers fail the check.** `CheckResult` gains `orphans`: consumers whose name matches no provider, each with a location and `suggestions` of similar provider names. `CheckResult::is_ok()` is false while any exist. A misspelled `<!-- {=featrues} -->` used to leave the docs silently unsynced while CI stayed green.
- **Unused providers are warnings, not errors.** `ProjectDiagnostic::is_error` no longer treats a provider without consumers as an error, so adding a provider before wiring its consumers (or running `mdt init` in a repository that already has a README) no longer blocks `check`, `update`, and `list`. The new `ProjectDiagnostic::is_ignored` reports whether `--ignore-unused-blocks` (or another ignore flag) silences a diagnostic.
- **New diagnostics** (`DiagnosticKind` and `ParseDiagnostic` variants):
  - `NestedBlock` (error): a block that opens inside a consumer or inline block. `mdt update` replaced everything between the outer tags and silently destroyed the inner block.
  - `InvalidTag` (error, silenced by `ignore_invalid_names`): a markdown comment that looks like a tag — `{` followed by `@`, `=`, `~`, or `/` — but does not parse, such as `<!-- { @name } -->` or `<!-- {=my.block} -->`. These were silently ignored.
  - `UnmatchedClosingTag` (warning): a `{/name}` with no open block of that name, usually the other half of a misspelled tag.
  - `ProviderOutsideTemplate` (warning): a provider tag outside a `*.t.md` file, which mdt ignores.
- **Unknown `mdt.toml` keys are rejected.** Every config table now denies unknown fields, so `[paddding]` or `max_filesize` fails with the offending key and the config path instead of being ignored. Config parse errors name the config file.
- **The closing tag starts on its own line by default.** Without a `[padding]` section, the default is now `before = 0`, `after = 0`. The old `after = false` default glued the closing tag to the last content line, which broke every trimmed `codeBlock` consumer (```` ```<!-- {/x} --> ```` is not a closing fence, so the fence swallowed the rest of the file) and dropped the comment prefix from source-file closing tags. Existing consumers in projects without `[padding]` will report stale once; run `mdt update`.
- **Text data drops one trailing newline**, as shell `$(...)` does. `release = { command = "cat VERSION", format = "text" }` now renders `1.2.3` rather than `1.2.3\n`, so inline values stay on one line.
- **TOML and KDL integers stay integers.** `port = 8080` renders as `8080`, not `8080.0`, and large integers keep full precision.

The index cache schema moves to version 3 so cached scans pick up the new diagnostics.
- **Unmatched closing tags are errors.** A `{/name}` with no open block is the other half of a misspelled or malformed opening tag, so that block silently stops syncing — in source-file comments nothing else reports it. `--ignore-unclosed-blocks` downgrades it like an unclosed block.
- **Unrendered data references warn.** In a project without `[data]`, providers are copied verbatim; a used provider containing namespaced variables such as `{{ pkg.version }}` now produces a `TemplateWarning` with `template_rendered: false`, instead of copying the braces silently (typically a sub-project reusing shared providers without declaring its own `[data]`).
- **Shared providers are never "unused".** Providers read from a `[templates] paths` directory outside the project are a shared library, so each project may use only some of them without warnings.
- `DiagnosticKind::code()` returns the stable `mdt::*` code for every diagnostic kind, and duplicate-provider errors name both definitions as project-relative `file:line`.
- **Blocks never nest.** `NestedBlock` now also covers blocks inside a provider: their tags would be copied into every consumer, where they nest. The error now appears at the provider instead of in every consumer.
