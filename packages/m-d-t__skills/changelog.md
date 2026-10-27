# Changelog

All notable changes to this project will be documented in this file.

This changelog is managed by [monochange](https://github.com/ifiokjr/monochange).

## [0.9.8](https://github.com/ifiokjr/mdt/releases/tag/v0.9.8) (2026-10-27)

### 🚀 Feature

#### update and list JSON output, shell completions, and a pre-commit hook installer

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #236](https://github.com/ifiokjr/mdt/pull/236)

**update --format json** and **list --format json** report results as one machine-readable JSON object (updated blocks and files, render errors, orphans, undefined-variable warnings, diagnostics), so CI pipelines and the MCP server can consume them; validation and config failures keep their exit codes and surface as diagnostics entries instead of empty stdout.

**mdt completions <shell>** prints completion scripts for bash, zsh, fish, PowerShell, and Elvish.

**mdt hook install** writes an executable .git/hooks/pre-commit that runs mdt check before every commit; it refuses to clobber a differing hook without --force and is a no-op when the mdt hook is already installed. A .pre-commit-hooks.yaml at the repository root exposes the same check to pre-commit-framework users as mdt-check.

New editor setup documentation covers Neovim, Helix, and Emacs configuration for the existing mdt lsp server, and the CLI reference documents all of the above. The embedded agent skill command table lists the new commands.

<details>
<summary><strong>📖 Documentation</strong></summary>

#### [format] preset and trailing-whitespace rule

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #218](https://github.com/ifiokjr/mdt/pull/218)

`[format]` grows from the single `tables` switch to a small rule set with a named preset:

- `preset = "dprint-markdown"` turns on every rule mdt implements — today `tables` and the new `trailing_whitespace` — so one line replaces the `[[formatters]]` subprocess pipeline for the common case. An explicit rule key overrides the preset for that rule alone.
- `trailing_whitespace = true` trims trailing whitespace from every line of block content, outside fenced code blocks, in providers and consumers alike. This is the classic strict-check divergence: formatters trim trailing spaces, so `mdt check` failed forever on files a formatter touched inside a block. With the rule on, mdt and the formatter agree byte-for-byte.

Like `tables`, the new rule runs inside block boundaries only, before transformers and padding, is idempotent, and `mdt update` rewrites provider blocks in `*.t.md` files that need it. The repo's own `mdt.toml` now uses the preset (dogfooding).

#### Add mdt_doctor and mdt_info MCP tools

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #215](https://github.com/ifiokjr/mdt/pull/215)

The MCP server can now self-diagnose, exactly like the CLI. The doctor and info report building moved from `mdt_cli` into `mdt_core`, so both surfaces share one implementation and the CLI output is unchanged:

- `mdt_core::doctor` runs the same health checks as `mdt doctor` (config discovery and parsing, data sources, template layout, duplicate providers, orphan consumers, unused providers, parser diagnostics, consumer sync, cache artifact, hash mode, and efficiency) and returns a serializable `DoctorReport`.
- `mdt_core::info` builds the same report as `mdt info` (`InfoReport`) with block counts, data namespaces, template directories and files, diagnostic totals, and cache state.
- New `mdt_doctor` tool: read-only health checks with `counts` (`pass`/`warn`/`fail`/`skip`) and `checks` (each with `id`, `status`, `message`, and a fix `hint`); accepts the `ignore_*` options like `mdt_check`. A broken config is reported as a failing check, not a tool error.
- New `mdt_info` tool: read-only project inspection with the report sections (`project`, `blocks`, `data`, `templates`, `diagnostics`, `cache`) flattened at the top level.

#### Scoped update operations

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #217](https://github.com/ifiokjr/mdt/pull/217)

`mdt update` can now sync a subset of the project instead of everything, for focused edits and agent workflows:

- `mdt update --block <NAME>` (repeatable) updates only consumers of the named blocks. An unknown name exits 2 with the closest existing block names.
- `mdt update --file <PATH>` (repeatable) updates only consumers in the given files. Paths resolve against the current directory and must stay inside the project root; a path matching no scanned file with consumers is an error.
- The MCP `mdt_update` tool accepts `block_names` for the same block scoping; an unknown name is an `isError` result with code `mdt::block_not_found` and suggested provider names. `dry_run` keeps its meaning.
- Output states the scope (`scoped to 1 block(s): intro`), and counts plus file lists cover only the scoped work.

Design notes:

- The scan stays whole-project (cheap, cached), so missing-provider and orphan semantics are unchanged; only the write is scoped.
- Validation still gates project-wide, exactly like `mdt update` — any validation error refuses the scoped write.
- Render failures in out-of-scope consumers never block a scoped run; in-scope failures are still reported and counted. `[[formatters]]` only run for files the scope touches.
- `--watch` composes with scoping and re-runs the same scope; `--block` and `--file` combine as an intersection.
- `mdt_core` exposes this as `UpdateScope` (`compute_updates(&ctx, &scope)`), with `UpdateScope::all()` matching the previous behavior.

#### Parse inline-block transformers inside GFM table cells

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #216](https://github.com/ifiokjr/mdt/pull/216)

GFM splits table rows on `|` before inline content is parsed, so an inline tag with a transformer chain (`<!-- {~name:"{{ pkg.version }}"|code} -->`) was cut into text fragments that never reached mdt, and its closing tag became a `mdt::unmatched_closing_tag` error. mdt now scans each table row's raw text before that split:

- Inline blocks inside table cells keep their full transformer chain; the rendered value is written back with only that value changing, preserving the rest of the row byte-for-byte.
- Pipes inside a quoted template argument (`{~name:"a|b"}`) also stay part of the tag in table cells.
- A second `mdt check` after `mdt update` is clean, and `mdt update` is idempotent.
- Escaped pipes (`\|`) and other cell text keep behaving exactly as GFM defines; behavior outside tables is unchanged.
- One caveat: a markdown formatter that rewrites tables and does not know mdt tags (such as dprint formatting fenced `markdown` blocks) can still split such a row on the pipe — keep examples in `text` fences.

#### One padding default: omitted keys mean 0

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #213](https://github.com/ifiokjr/mdt/pull/213)

`[padding]` had two defaults: an absent section meant `before = 0, after = 0`, but a present section with an omitted key defaulted to `1` (one blank line). `[padding]` with only `after = 2` silently meant `before = 1, after = 2` — the most-documented confusion point in mdt's config.

Now an omitted key behaves exactly like an absent section: content on the next line, no blank lines. Padding applies only when configured; write `before = 1` / `after = 1` explicitly when you want blank lines.

Configs that relied on the implicit `1` (a bare `[padding]` section, or a section listing only one key) must now state the values. The annotated `mdt.toml`, the configuration guide, and the agent skill are updated to the new rule.

</details>

## [0.9.7](https://github.com/ifiokjr/mdt/releases/tag/v0.9.7) (2026-10-23)

### Changed

- **No package-specific changes were recorded; `@m-d-t/skills` was updated to 0.9.7 as part of group `mdt`.**

## [0.9.6](https://github.com/ifiokjr/mdt/releases/tag/v0.9.6) (2026-10-07)

### 🚀 Feature

#### Rewrite the mdt skill from end-to-end agent evaluations and ship it inside the CLI

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #202](https://github.com/ifiokjr/mdt/pull/202)

The skill was rebuilt from evaluations where agents adopted mdt in polyglot monorepos using only the skill. It is now also embedded in the `mdt` binary: agents without the skill installed can run `mdt skill` (and `mdt skill --reference`), and `mdt skill --install <dir>` installs it for Claude Code, Pi, Copilot, or any agent that reads `.agents/skills`.

- A shorter `SKILL.md` covering the workflow, block rules, transformers, per-language comment prefixes (including indented class methods and `impl` blocks), data, formatter recipes verified with dprint, prettier, and rustfmt, CI and exit codes, monorepos with shared providers, and a table mapping each `mdt check` failure to its fix.
- A complete `REFERENCE.md`: tag syntax, block arguments, the transformer table, exact padding semantics, data formats and script caching, every `mdt.toml` key, scanning and git-ignore rules, diagnostic codes, CLI exit codes and JSON output, and MCP tool contracts.
- Corrects claims that were wrong: `[include]` and `[templates] paths` add to the scan, `lenient` comparison only handles trailing whitespace and blank lines, unclosed tags are errors, blocks cannot be nested, formatter placeholders must stay double-quoted, and a `replace` example no longer closes the comment it sits in.

- **Rewrite the mdt skill around formatter conflicts, code files, and stale-doc prevention.** The skill now teaches the `mdt update → formatter → mdt check` loop with the built-in escapes (`[[formatters]]`, `[check] comparison = "lenient"`, formatter-stable providers), adds complete TypeScript, Rust, and Dart consumer examples with `[padding]` semantics, documents the block-name charset and the silent-skip behavior for unscanned extensions, adds install/upgrade guidance for `@m-d-t/cli`, corrects the closing-tag prefix behavior, and fixes broken code fences in the data-interpolation reference. _Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #198](https://github.com/ifiokjr/mdt/pull/198)

<details>
<summary><strong>📖 Documentation</strong></summary>

#### Rewrite the documentation to match actual behavior

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #202](https://github.com/ifiokjr/mdt/pull/202)

An audit ran every documented command, flag, default, and example against the CLI. The docs site, crate readmes, the annotated `mdt.toml` that `mdt init` writes, and generated doc comments now match the implementation:

- `[include]` and `[templates] paths` are documented as additive (they were described as narrowing the scan), `[check] comparison = "lenient"` no longer claims to normalize tables or JSON, exclude negation examples work, and formatter patterns are documented as plain globs.
- Exit codes (`0`/`1`/`2`), the global `--ignore-*` flags, project-root discovery, `mdt skill`, per-client `mdt assist` output, the complete JSON payload, GitHub annotations, and every quoted command output are current.
- Source-file examples re-apply comment prefixes with `linePrefix:"...":true`, so copied examples compile.
- Terminology is provider/consumer throughout.
- Crate readme badges render again (link definitions were joined onto one line).
- Links that left the book now point at GitHub, and the Pi link points at pi.dev.

</details>

## [0.9.5](https://github.com/ifiokjr/mdt/releases/tag/v0.9.5) (2026-09-20)

### Changed

- **No package-specific changes were recorded; `@m-d-t/skills` was updated to 0.9.5 as part of group `mdt`.**

## [0.9.4](https://github.com/ifiokjr/mdt/releases/tag/v0.9.4) (2026-09-16)

### Changed

- **No package-specific changes were recorded; `@m-d-t/skills` was updated to 0.9.4 as part of group `mdt`.**

## [0.9.3](https://github.com/ifiokjr/mdt/releases/tag/v0.9.3) (2026-09-16)

### Changed

- **No package-specific changes were recorded; `@m-d-t/skills` was updated to 0.9.3 as part of group `mdt`.**

## [0.9.2](https://github.com/ifiokjr/mdt/releases/tag/v0.9.2) (2026-09-07)

### Changed

- No package-specific changes were recorded; `@m-d-t/skills` was updated to 0.9.2 as part of group `mdt`.

## [0.9.1](https://github.com/ifiokjr/mdt/releases/tag/v0.9.1) (2026-08-21)

<details>
<summary><strong>📖 Documentation</strong></summary>

#### Documentation rewrite

Rewrite documentation across the repo: tighten prose, drop AI-flavored phrasing, fix stale references (knope to monochange, version numbers, missing `mdt list` command, license badge typo), align the annotated config docs with the actual strict-default plus formatters setup, and add the missing `if` transformer docs. Template-driven content was updated in `.templates/` and synced via `mdt update`.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #167](https://github.com/ifiokjr/mdt/pull/167)

</details>

## [0.9.0](https://github.com/ifiokjr/mdt/releases/tag/v0.9.0) (2026-07-04)

### 🐛 Fixed

#### Add package repository metadata

Cargo and npm package manifests now include package-specific repository URLs. This keeps package metadata aligned with monochange manifest linting and points registry users directly to each package's source directory.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #153](https://github.com/ifiokjr/mdt/pull/153)

<details>
<summary><strong>🔨 Refactor</strong></summary>

#### Remove the legacy npm source folder

The old `npm/` tree has been removed now that npm packages live under `packages/`. Tests and repository metadata now point at the generated package launcher and package directories under `packages/`.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #147](https://github.com/ifiokjr/mdt/pull/147) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

</details>
