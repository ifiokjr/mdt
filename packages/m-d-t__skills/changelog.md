# Changelog

All notable changes to this project will be documented in this file.

This changelog is managed by [monochange](https://github.com/ifiokjr/monochange).

## [0.9.6](https://github.com/ifiokjr/mdt/releases/tag/v0.9.6) (2026-09-30)

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
