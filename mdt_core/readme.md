# mdt

> update markdown content anywhere using comments as template tags

<br />

[![Crate][crate-image]][crate-link] [![Docs][docs-image]][docs-link] [![Status][ci-status-image]][ci-status-link] [![Coverage][coverage-image]][coverage-link] [![Unlicense][unlicense-image]][unlicense-link]

<br />

<!-- {=mdtPackageDocumentation} -->

`mdt` is a data-driven template engine for keeping documentation synchronized across your project. It uses comment-based template tags to define content once and distribute it to multiple locations — markdown files, code documentation comments (in any language), READMEs, mdbook docs, and more.

<!-- {/mdtPackageDocumentation} -->

## Installation

<!-- {=mdtCoreInstall} -->

```toml
[dependencies]
mdt_core = "0.3.0"
```

<!-- {/mdtCoreInstall} -->

[coverage-image]: https://codecov.io/gh/ifiokjr/mdt/branch/main/graph/badge.svg
[coverage-link]: https://codecov.io/gh/ifiokjr/mdt
[crate-image]: https://img.shields.io/crates/v/mdt.svg
[crate-link]: https://crates.io/crates/mdt
[docs-image]: https://docs.rs/mdt/badge.svg
[docs-link]: https://docs.rs/mdt/
[ci-status-image]: https://github.com/ifiokjr/mdt/workflows/ci/badge.svg
[ci-status-link]: https://github.com/ifiokjr/mdt/actions?query=workflow:ci
[unlicense-image]: https://img.shields.io/badge/license-Unlicence-blue.svg
[unlicense-link]: https://opensource.org/license/unlicense
