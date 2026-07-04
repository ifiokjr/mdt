# Changelog

All notable changes to this project will be documented in this file.

This changelog is managed by [monochange](https://github.com/ifiokjr/monochange).

## [0.9.0](https://github.com/ifiokjr/mdt/releases/tag/v0.9.0) (2026-07-04)

### 💥 Breaking Change

#### Publish the CLI through npm packages

`mdt` now has an official npm distribution channel. Releases prepare a top-level `@m-d-t/cli` package plus platform-specific binary packages for Linux, macOS, and Windows.

Users can install the CLI globally with npm or run it on demand through npx, making adoption easier in JavaScript-heavy projects and environments that do not already have Rust tooling installed.

```bash
npx @m-d-t/cli init
```

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #121](https://github.com/ifiokjr/mdt/pull/121) · _Related issues:_ [#152](https://github.com/ifiokjr/mdt/issues/152)

### 🐛 Fixed

#### Add package repository metadata

Cargo and npm package manifests now include package-specific repository URLs. This keeps package metadata aligned with monochange manifest linting and points registry users directly to each package's source directory.

_Owner:_ [@ifiokjr](https://github.com/ifiokjr) · _Review:_ [PR #153](https://github.com/ifiokjr/mdt/pull/153)
