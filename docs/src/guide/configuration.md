# Configuration

mdt works without a config file. Add one when you want data in providers, different scan rules, padding control, or formatter integration. For every key, type, and default, see the [Configuration Reference](../reference/configuration.md).

mdt reads the first of `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml` in the project root. Unknown keys are rejected with the key name and the config path, so a typo such as `[paddding]` fails with exit status 2 instead of being ignored.

## Start from `mdt init`

`mdt init` writes an annotated starter `mdt.toml` with every option commented out. Uncomment a table header together with the keys directly below it:

<!-- {=mdtInitAnnotatedConfiguration|trim|codeBlock:"toml"} -->

```toml
# mdt.toml
#
# Starter configuration for mdt. mdt works without a config file; every option
# below is optional and commented out. To enable one, uncomment its [table]
# header together with the keys you need directly below it.
#
# mdt reads the first of `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml` in the
# project root. Unknown keys are rejected, so a typo fails instead of being
# ignored. mdt caches scan results in `.mdt/`; keep that directory out of git.

# Top-level keys must stay above the first [table] header.

# Largest file mdt will scan, in bytes. A scanned file above the limit is an
# error that names the file. Default: 10 MB.
# max_file_size = 10485760

# mdt follows git's ignore rules: `.gitignore` files in the project and its
# parent directories up to the repository root, plus `.git/info/exclude`.
# Set `true` to scan ignored files as well. Hidden directories (except
# `.templates`), `node_modules`, and `target` are skipped either way.
# disable_gitignore = true

# [padding] sets the blank lines between a consumer's tags and its content.
#   false -> content on the same line as the tag
#   0     -> content on the next line (the default without this section)
#   1     -> one blank line (2, 3, ... for more)
# With the section present, an omitted key defaults to 1. Padding is added on
# top of the provider's own leading and trailing newlines; add `|trim` to a
# consumer for exact control. Inline blocks are never padded.
# [padding]
# before = 0
# after = 0

# [check] sets how `mdt check` compares consumers with their expected content.
#   "strict"  -> exact bytes (default)
#   "lenient" -> ignore trailing whitespace and runs of blank lines
# Lenient mode still compares indentation, table padding, and JSON layout; use
# [[formatters]] for those. `mdt update` always writes exact bytes.
# [check]
# comparison = "lenient"

# [data] maps namespaces to data that providers can use as minijinja
# variables, for example `{{ package.version }}`. Entry forms:
#   "file.json"                            -> parser chosen by extension
#   { path = "file", format = "json" }     -> explicit parser
#   { command = "...", format = "text" }   -> parse a command's stdout
# Formats: json, toml, yaml (yml), kdl, ini, and text (also string, raw, txt).
# Text drops one trailing newline. Commands run from the project root. Their
# output is cached in `.mdt/cache/data-v1.json` while every `watch` entry is an
# existing file; without `watch` the command runs on every mdt run.
# Once [data] is set, all provider content is rendered as a template: wrap
# literal `{{` or `{%` text in `{% raw %}...{% endraw %}`.
# [data]
# package = "package.json"
# cargo = "Cargo.toml"
# release = { path = "release-info", format = "json" }
# version = { command = "cat VERSION", format = "text", watch = ["VERSION"] }

# [exclude] skips files and block names.
#   patterns            -> gitignore-style patterns. To re-include a file with
#                          `!`, exclude the directory's contents (`generated/*`),
#                          not the directory itself (`generated/`), as in git.
#   blocks              -> block names to ignore everywhere, diagnostics included
#   markdown_codeblocks -> ignore tags in fenced code blocks inside source-file
#                          comments: true (all), "text" (info string contains
#                          it), or a list of such strings. Default: false.
#                          Tags in fenced code blocks in markdown files are
#                          always ignored.
# [exclude]
# patterns = ["vendor/", "generated/*", "!generated/keep.md"]
# blocks = ["draftSection"]
# markdown_codeblocks = true

# [include] adds files to the default scan (markdown files and supported source
# files); it never narrows it. Included files still follow the ignore rules and
# [exclude]. Use it for other file types, and avoid broad globs such as `src/**`
# that match binary files.
# [include]
# patterns = ["scripts/**/*.rb"]

# [templates] adds the `*.t.md` provider files in extra directories, relative to
# the project root. They may be hidden (`.github/templates`) or outside the
# project (`../shared/templates`). `*.t.md` files in the project are always read.
# [templates]
# paths = ["../shared/templates"]

# [[formatters]] runs your formatter as part of `mdt update` and `mdt check`, so
# synced files stay formatted and `mdt check` compares formatted output.
# Each entry reads the whole file on stdin and writes the result to stdout. It
# runs from the project root through `sh -c` (`cmd /C` on Windows). Matching
# entries run in order, and a failing formatter is an error.
#   command  -> may use {{ filePath }} (absolute), {{ relativeFilePath }}, and
#               {{ rootDirectory }}. mdt passes them as environment variables,
#               so keep them inside double quotes.
#   patterns -> globs of files to format; a `!` entry excludes. These are plain
#               globs: write `vendor/**`, not `vendor/`.
#   ignore   -> globs to skip, same syntax
# Keep `*.t.md` files out of formatter scope, here and in the formatter's own
# config, because markdown formatters rewrite provider text.
# [[formatters]]
# command = "dprint fmt --stdin \"{{ filePath }}\""
# patterns = ["**/*.md"]
# ignore = ["**/*.t.md"]
```

<!-- {/mdtInitAnnotatedConfiguration} -->

## Use project data in providers

Map namespaces to data files or commands:

```toml
[data]
package = "package.json"
commit = { command = "git rev-parse --short HEAD", format = "text" }
```

Providers can then use `{{ package.version }}` or `{{ commit }}`. Once `[data]` exists, every provider is rendered as a template, so wrap literal `{{` text in `{% raw %}...{% endraw %}`. See [Data Interpolation](./data-interpolation.md).

## Choose which files are scanned

By default mdt scans markdown files, supported source files, and `*.t.md` providers, and skips hidden directories (except `.templates`), `node_modules/`, `target/`, sub-projects, and anything git ignores.

Skip more files with gitignore-style patterns. To keep one file from an excluded directory, exclude the directory's contents rather than the directory itself, as in git:

```toml
[exclude]
patterns = ["vendor/", "generated/*", "!generated/keep.md"]
```

Scan an extra file type. `[include]` adds files to the default scan and never narrows it:

```toml
[include]
patterns = ["scripts/**/*.rb"]
```

Read providers from a hidden directory or a shared directory outside the project. `[templates] paths` adds the `*.t.md` files it finds there:

```toml
[templates]
paths = [".github/templates", "../shared/templates"]
```

Scan files that git ignores, such as generated docs:

```toml
disable_gitignore = true
```

Top-level keys like `disable_gitignore` and `max_file_size` must come before the first `[table]` header.

## Control blank lines around content

`[padding]` sets the blank lines between a consumer's tags and its content. Without the section, both values are `0`: content starts on the line after the opening tag, and the closing tag starts on its own line.

```toml
[padding]
before = 1
after = 1
```

Values are `false` (same line as the tag), `0` (next line), `1` (one blank line), and so on. With the section present, an omitted key defaults to `1`.

Padding is added on top of the provider's own leading and trailing newlines. Providers are usually written with blank lines around their content, so add `|trim` to a consumer when you need exact output, for example in source-file comments:

```rust
//! <!-- {=docs|trim|linePrefix:"//! ":true} -->
//! This content stays inside the doc comment.
//! <!-- {/docs} -->
```

## Keep formatters and mdt in agreement

If a formatter such as dprint or Prettier rewrites synced files, `mdt check` would report them as stale after every format. Declare the formatter so mdt formats what it writes and compares against formatted output:

```toml
[[formatters]]
command = "dprint fmt --stdin \"{{ filePath }}\""
patterns = ["**/*.md"]
ignore = ["**/*.t.md"]
```

For whitespace-only differences, `[check] comparison = "lenient"` ignores trailing whitespace and runs of blank lines instead. It does not ignore indentation, table padding, or JSON layout. See [`[[formatters]]`](../reference/configuration.md#formatters) for placeholders, glob rules, and verified commands.

## Sub-projects

A directory below the root that contains `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml` is a separate project. The parent's scan skips it, so run `mdt check --path <dir>` for each project. See [Monorepos](../advanced/monorepos.md).

## This repository's `mdt.toml`

The mdt repository's own annotated config, synced from the same source as the file in the repository root:

<!-- {=mdtAnnotatedConfiguration|trim|codeBlock:"toml"} -->

```toml
# mdt.toml
#
# The mdt repository's own configuration, annotated as a reference. Active
# entries are the settings this repo uses; commented entries document the
# remaining options. When config behavior changes, update this file and the
# configuration guide in the same PR.
#
# mdt reads the first of `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml` in the
# project root. Unknown keys are rejected, so a typo fails instead of being
# ignored.

# Top-level keys must stay above the first [table] header.

# Largest file mdt will scan, in bytes. A scanned file above the limit is an
# error that names the file. Default: 10 MB.
# max_file_size = 10485760

# mdt follows git's ignore rules: `.gitignore` files in the project and its
# parent directories up to the repository root, plus `.git/info/exclude`.
# Set `true` to scan ignored files as well. Hidden directories (except
# `.templates`), `node_modules`, and `target` are skipped either way.
# disable_gitignore = true

# [padding] sets the blank lines between a consumer's tags and its content.
#   false -> content on the same line as the tag
#   0     -> content on the next line (the default without this section)
#   1     -> one blank line (2, 3, ... for more)
# With the section present, an omitted key defaults to 1. Padding is added on
# top of the provider's own leading and trailing newlines; add `|trim` to a
# consumer for exact control. Inline blocks are never padded.
#
# This repo writes out the default so the section is documented here.
[padding]
before = 0
after = 0

# [check] sets how `mdt check` compares consumers with their expected content.
#   "strict"  -> exact bytes (default)
#   "lenient" -> ignore trailing whitespace and runs of blank lines
# Lenient mode still compares indentation, table padding, and JSON layout; use
# [[formatters]] for those. `mdt update` always writes exact bytes.
#
# This repo keeps strict comparison and configures [[formatters]] below.
# [check]
# comparison = "lenient"

# [data] maps namespaces to data that providers can use as minijinja
# variables, for example `{{ cargo.workspace.package.version }}`. Entry forms:
#   "file.json"                            -> parser chosen by extension
#   { path = "file", format = "json" }     -> explicit parser
#   { command = "...", format = "text" }   -> parse a command's stdout
# Formats: json, toml, yaml (yml), kdl, ini, and text (also string, raw, txt).
# Text drops one trailing newline. Commands run from the project root. Their
# output is cached in `.mdt/cache/data-v1.json` while every `watch` entry is an
# existing file; without `watch` the command runs on every mdt run.
# Once [data] is set, all provider content is rendered as a template: wrap
# literal `{{` or `{%` text in `{% raw %}...{% endraw %}`.
[data]
cargo = "Cargo.toml"
# release = { path = "release-info", format = "json" }
# version = { command = "cat VERSION", format = "text", watch = ["VERSION"] }

# [exclude] skips files and block names.
#   patterns            -> gitignore-style patterns. To re-include a file with
#                          `!`, exclude the directory's contents (`generated/*`),
#                          not the directory itself (`generated/`), as in git.
#   blocks              -> block names to ignore everywhere, diagnostics included
#   markdown_codeblocks -> ignore tags in fenced code blocks inside source-file
#                          comments: true (all), "text" (info string contains
#                          it), or a list of such strings. Default: false.
#                          Tags in fenced code blocks in markdown files are
#                          always ignored.
#
# This repo skips test fixtures and snapshots, and treats tags in Rust doc
# comment examples as illustrations rather than live blocks.
[exclude]
patterns = [
  "**/tests/",
  "**/__tests.rs",
  "**/snapshots/",
]
markdown_codeblocks = true
# blocks = ["draftSection"]

# [include] adds files to the default scan (markdown files and supported source
# files); it never narrows it. Included files still follow the ignore rules and
# [exclude]. Use it for other file types, and avoid broad globs such as `src/**`
# that match binary files.
# [include]
# patterns = ["scripts/**/*.rb"]

# [templates] adds the `*.t.md` provider files in extra directories, relative to
# the project root. They may be hidden (`.github/templates`) or outside the
# project (`../shared/templates`). `*.t.md` files in the project are always read.
# [templates]
# paths = ["../shared/templates"]

# [[formatters]] runs your formatter as part of `mdt update` and `mdt check`, so
# synced files stay formatted and `mdt check` compares formatted output.
# Each entry reads the whole file on stdin and writes the result to stdout. It
# runs from the project root through `sh -c` (`cmd /C` on Windows). Matching
# entries run in order, and a failing formatter is an error.
#   command  -> may use {{ filePath }} (absolute), {{ relativeFilePath }}, and
#               {{ rootDirectory }}. mdt passes them as environment variables,
#               so keep them inside double quotes.
#   patterns -> globs of files to format; a `!` entry excludes. These are plain
#               globs: write `vendor/**`, not `vendor/`.
#   ignore   -> globs to skip, same syntax
# Keep `*.t.md` files out of formatter scope, here and in the formatter's own
# config, because markdown formatters rewrite provider text.
#
# This repo formats synced markdown with dprint, the formatter used for the
# rest of the workspace (`dprint.json` also excludes `.templates/**`).
[[formatters]]
command = "dprint fmt --stdin \"{{ filePath }}\""
patterns = ["**/*.md"]
ignore = ["**/*.t.md"]

# Add an entry per tool when file types need different formatters.
# [[formatters]]
# command = "prettier --stdin-filepath \"{{ filePath }}\""
# patterns = ["**/*.ts", "**/*.tsx"]
```

<!-- {/mdtAnnotatedConfiguration} -->
