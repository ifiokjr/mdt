# Changelog

All notable changes to this project will be documented in this file.

This changelog is managed by [monochange](https://github.com/ifiokjr/monochange).

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
