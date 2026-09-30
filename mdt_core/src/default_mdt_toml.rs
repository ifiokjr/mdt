// This file is generated via mdt. Edit the source blocks in `.templates/` instead.

// <!-- {=mdtInitAnnotatedConfigurationRust|trim} -->
pub(crate) const DEFAULT_MDT_TOML: &str = r#"# mdt.toml
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
"#;
// <!-- {/mdtInitAnnotatedConfigurationRust} -->
