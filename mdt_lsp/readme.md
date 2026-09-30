# mdt_lsp

> language server for mdt (manage markdown templates)

<br />

[![Crate][crate-image]][crate-link] [![Docs][docs-image]][docs-link] [![Status][ci-status-image]][ci-status-link] [![Coverage][coverage-image]][coverage-link] [![Unlicense][unlicense-image]][unlicense-link]

<br />

<!-- {=mdtLspOverview} -->

`mdt_lsp` is a [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) server for [mdt](https://github.com/ifiokjr/mdt). It shows mdt diagnostics in your editor and lets you navigate, rename, and update blocks without leaving it.

### Capabilities

- **Diagnostics**: stale consumers and inline blocks, provider render failures, orphan consumers (with a did-you-mean suggestion), unclosed blocks, unmatched closing tags, comments that look like tags but do not parse, blocks nested inside a consumer, unknown transformers and wrong argument counts, duplicate providers, unused providers, and providers outside `*.t.md` files.
- **Quick fix**: "Update block" rewrites a stale consumer exactly as `mdt update` would, padding and comment prefixes included.
- **Completions**: block names after `{=`, `{~`, `{@`, and `{/`, and transformer names after `|`.
- **Hover**: a block's provider, rendered content, transformer chain, and consumer count.
- **Go to definition**: from a consumer to its provider, or from a provider to its consumers.
- **References**: every provider, consumer, and inline block with the same name.
- **Rename**: a block name in every opening and closing tag in the workspace.
- **Document symbols**: providers, consumers, and inline blocks in the outline view.

The server reads `mdt.toml` and honors `[padding]`, `[check] comparison`, and `[exclude] markdown_codeblocks`. It does not run `[[formatters]]`, so it reports no stale blocks (and offers no quick fix) in files a formatter owns; run `mdt check` or `mdt update` for those.

### Usage

Start the language server through the CLI:

```sh
mdt lsp
```

It communicates over stdin/stdout and uses the editor's workspace folder as the project root.

<!-- {/mdtLspOverview} -->

## Installation

<!-- {=mdtLspInstall} -->

```toml
[dependencies]
mdt_lsp = "0.9.5"
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
[unlicense-image]: https://img.shields.io/badge/license-Unlicense-blue.svg
[unlicense-link]: https://opensource.org/license/unlicense

<!-- {/mdtBadgeLinks} -->
