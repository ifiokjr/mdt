# mdt_lsp

> language server for mdt (manage markdown templates)

<br />

[![Crate][crate-image]][crate-link] [![Docs][docs-image]][docs-link] [![Status][ci-status-image]][ci-status-link] [![Coverage][coverage-image]][coverage-link] [![Unlicense][unlicense-image]][unlicense-link]

<br />

<!-- {=mdtLspOverview} -->

`mdt_lsp` is a [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) implementation for the [mdt](https://github.com/ifiokjr/mdt) template engine. It provides real-time editor integration for managing markdown template blocks.

### Capabilities

- **Diagnostics** — reports stale consumer blocks, missing providers (with name suggestions), unclosed blocks, unknown transformers, invalid arguments, unused providers, and provider blocks in non-template files.
- **Completions** — suggests block names after `{=`, `{@`, and `{/` tags, and transformer names after `|`.
- **Hover** — shows provider source, rendered content, transformer chain, and consumer count when hovering over a block tag.
- **Go to definition** — navigates from a consumer block to its provider, or from a provider to all of its consumers.
- **References** — finds all provider and consumer blocks sharing the same name.
- **Rename** — renames a block across all provider and consumer tags (both opening and closing) in the workspace.
- **Document symbols** — lists all provider and consumer blocks in the outline/symbol view.
- **Code actions** — offers a quick-fix to update stale consumer blocks in place.

### Usage

Start the language server via the CLI:

```sh
mdt lsp
```

The server communicates over stdin/stdout using the Language Server Protocol.

<!-- {/mdtLspOverview} -->

## Installation

<!-- {=mdtLspInstall} -->

```toml
[dependencies]
mdt_lsp = "0.6.0"
```

<!-- {/mdtLspInstall} -->

<!-- {=mdtBadgeLinks:"mdt_lsp"} -->

[crate-image]: https://img.shields.io/crates/v/mdt_lsp.svg
[crate-link]: https://crates.io/crates/mdt_lsp
[docs-image]: https://docs.rs/mdt_lsp/badge.svg
[docs-link]: https://docs.rs/mdt_lsp/
[ci-status-image]: https://github.com/ifiokjr/mdt/workflows/ci/badge.svg
[ci-status-link]: https://github.com/ifiokjr/mdt/actions?query=workflow:ci
[coverage-image]: https://codecov.io/gh/ifiokjr/mdt/branch/main/graph/badge.svg
[coverage-link]: https://codecov.io/gh/ifiokjr/mdt
[unlicense-image]: https://img.shields.io/badge/license-Unlicence-blue.svg
[unlicense-link]: https://opensource.org/license/unlicense

<!-- {/mdtBadgeLinks} -->
