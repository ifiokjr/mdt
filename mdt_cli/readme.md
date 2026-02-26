# mdt_cli

> the cli which updates markdown content anywhere using comments as template tags

<br />

[![Crate][crate-image]][crate-link] [![Docs][docs-image]][docs-link] [![Status][ci-status-image]][ci-status-link] [![Coverage][coverage-image]][coverage-link] [![Unlicense][unlicense-image]][unlicense-link]

<br />

<!-- {=mdtPackageDocumentation} -->

`mdt` is a data-driven template engine for keeping documentation synchronized across your project. It uses comment-based template tags to define content once and distribute it to multiple locations — markdown files, code documentation comments (in any language), READMEs, mdbook docs, and more.

<!-- {/mdtPackageDocumentation} -->

<!-- {=mdtCliUsage} -->

### CLI Commands

- `mdt init [--path <dir>]` — Create a sample `template.t.md` file with getting-started instructions.
- `mdt check [--path <dir>] [--verbose]` — Verify all consumer blocks are up-to-date. Exits non-zero if any are stale.
- `mdt update [--path <dir>] [--verbose] [--dry-run]` — Update all consumer blocks with latest provider content.
- `mdt lsp` — Start the mdt language server (LSP) for editor integration. Communicates over stdin/stdout.
- `mdt mcp` — Start the mdt MCP server for AI assistants. Communicates over stdin/stdout.

<!-- {/mdtCliUsage} -->

<!-- {=mdtBadgeLinks:"mdt_cli"} -->

[coverage-image]: https://codecov.io/gh/ifiokjr/mdt/branch/main/graph/badge.svg
[coverage-link]: https://codecov.io/gh/ifiokjr/mdt

[crate-image]: https://img.shields.io/crates/v/mdt_cli.svg [crate-link]: https://crates.io/crates/mdt_cli [docs-image]: https://docs.rs/mdt_cli/badge.svg [docs-link]: https://docs.rs/mdt_cli/ [ci-status-image]: https://github.com/ifiokjr/mdt/workflows/ci/badge.svg [ci-status-link]: https://github.com/ifiokjr/mdt/actions?query=workflow:ci [unlicense-image]: https://img.shields.io/badge/license-Unlicence-blue.svg [unlicense-link]: https://opensource.org/license/unlicense

<!-- {/mdtBadgeLinks} -->
