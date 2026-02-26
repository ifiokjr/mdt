# mdt

> manage **m**ark**d**own **t**emplates across your project

<br />

[![Status][ci-status-image]][ci-status-link] [![Coverage][coverage-image]][coverage-link] [![Unlicense][unlicense-image]][unlicense-link]

<br />

<!-- {=mdtPackageDocumentation} -->

`mdt` is a data-driven template engine for keeping documentation synchronized across your project. It uses comment-based template tags to define content once and distribute it to multiple locations — markdown files, code documentation comments (in any language), READMEs, mdbook docs, and more.

<!-- {/mdtPackageDocumentation} -->

<!-- {=mdtTemplateSyntax} -->

### Template Syntax

**Provider tag** (defines a template block in `*.t.md` definition files):

```markdown
<!-- {@blockName} -->

Content to inject

<!-- {/blockName} -->
```

**Consumer tag** (marks where content should be injected):

```markdown
<!-- {=blockName} -->

This content gets replaced

<!-- {/blockName} -->
```

**Filters and pipes:** Template values support pipe-delimited transformers:

```markdown
<!-- {=block|prefix:"\n"|indent:"  "} -->
```

Available transformers: `trim`, `trimStart`, `trimEnd`, `indent`, `prefix`, `wrap`, `codeBlock`, `code`, `replace`.

<!-- {/mdtTemplateSyntax} -->

<!-- {=mdtCliUsage} -->

### CLI Commands

- `mdt init [--path <dir>]` — Create a sample `template.t.md` file with getting-started instructions.
- `mdt check [--path <dir>] [--verbose]` — Verify all consumer blocks are up-to-date. Exits non-zero if any are stale.
- `mdt update [--path <dir>] [--verbose] [--dry-run]` — Update all consumer blocks with latest provider content.
- `mdt lsp` — Start the mdt language server (LSP) for editor integration. Communicates over stdin/stdout.
- `mdt mcp` — Start the mdt MCP server for AI assistants. Communicates over stdin/stdout.

<!-- {/mdtCliUsage} -->

## LSP

The `mdt_lsp` crate provides a fully implemented language server for editor integration. Start it with `mdt lsp` or run the `mdt-lsp` binary directly. The server communicates over stdin/stdout and supports the following capabilities:

- **Diagnostics** -- reports stale consumer blocks, missing providers with name suggestions, unclosed blocks, unknown transformers, invalid transformer arguments, unused providers, and provider blocks in non-template files.
- **Completions** -- suggests block names after `{=`, `{@`, and `{/` tags, and transformer names after `|`.
- **Hover** -- shows provider source, rendered content, transformer chain, and consumer count when hovering over a block tag.
- **Go to definition** -- navigates from a consumer block to its provider, or from a provider to all of its consumers.
- **Document symbols** -- lists all provider and consumer blocks in the outline/symbol view.
- **Code actions** -- offers a quick-fix to update stale consumer blocks in place.

## Contributing

[`devenv`](https://devenv.sh/) is used to provide a reproducible development environment for this project. Follow the [getting started instructions](https://devenv.sh/getting-started/).

If you want to use flakes you may need to run the following command after initial setup.

```bash
echo "experimental-features = nix-command flakes" >> $HOME/.config/nix/nix.conf
```

To automatically load the environment you should [install direnv](https://devenv.sh/automatic-shell-activation/) and then load the `direnv`.

```bash
# The security mechanism didn't allow to load the `.envrc`.
# Since we trust it, let's allow it execution.
direnv allow .
```

At this point you should see the `nix` commands available in your terminal. Run `install:all` to install all tooling.

To setup recommended configuration for your favourite editor run the following commands.

```bash
setup:vscode # Setup vscode
setup:helix  # Setup helix configuration
```

### Upgrading `devenv`

If you have an outdated version of `devenv` you can update it by running the following commands. If you know an easier way, please create a PR and I'll update these docs.

```bash
nix profile list # find the index of the nix package
nix profile remove <index>
nix profile install --accept-flake-config github:cachix/devenv/<version>
```

[ci-status-image]: https://github.com/ifiokjr/mdt/workflows/ci/badge.svg
[ci-status-link]: https://github.com/ifiokjr/mdt/actions?query=workflow:ci
[coverage-image]: https://codecov.io/gh/ifiokjr/mdt/branch/main/graph/badge.svg
[coverage-link]: https://codecov.io/gh/ifiokjr/mdt
[unlicense-image]: https://img.shields.io/badge/license-Unlicence-blue.svg
[unlicense-link]: https://opensource.org/license/unlicense
