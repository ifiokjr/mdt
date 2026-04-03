// This file is generated via mdt. Edit `template.t.md` instead.

// <!-- {=mdtInitAnnotatedConfigurationRust|trim} -->
pub(crate) const DEFAULT_MDT_TOML: &str = r####"# mdt.toml
#
# Welcome to mdt. This starter config is intentionally fully annotated so you
# can discover every supported option in one place.
#
# Uncomment only what your project needs. mdt works without a config file, but
# `mdt.toml` becomes useful once you want data interpolation, custom scanning
# rules, padding control, or formatter-aware convergence.
#
# When in doubt, start with a sample template + target block, run `mdt update`,
# and then come back here to enable the options that match your workflow.

# Maximum file size (in bytes) that mdt will scan before failing fast.
# Leave this commented to use the built-in default of 10 MB.
# max_file_size = 10485760

# By default mdt respects `.gitignore` and skips ignored files.
# Uncomment this only when ignored/generated files should still be scanned, or
# when you want `[include]` / `[exclude]` to be your only scanning rules.
# disable_gitignore = true

# `[padding]` controls the whitespace between tags and injected content.
# Supported values for `before` and `after`:
# - false -> keep content inline with the tag
# - 0     -> put content on the next line with no blank line
# - 1     -> add one blank line
# - 2+    -> add two or more blank lines
#
# Recommended when your targets live in source-code comments or when formatters
# tend to rewrite surrounding whitespace.
# [padding]
# before = 0
# after = 0

# `[data]` maps namespaces to external data sources. These values are available
# in source blocks through minijinja templates like `{{ package.version }}`.
#
# String values are file-backed sources. The parser is inferred from the file
# extension (`.json`, `.toml`, `.yaml`, `.yml`, `.kdl`, `.ini`).
# [data]
# package = "package.json"
# cargo = "Cargo.toml"
# config = "config.yaml"
#
# Typed data sources force a parser when the extension is missing or unusual.
# release = { path = "release-info", format = "json" }
#
# Script-backed data sources run a shell command from the project root and parse
# stdout. `format` accepts: `text`, `string`, `raw`, `txt`, `json`, `toml`,
# `yaml`, `yml`, `kdl`, or `ini`.
# `watch` files control cache invalidation in `.mdt/cache/data-v1.json`.
# version = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
# git = { command = "git rev-parse --short HEAD", format = "text" }

# `[exclude]` skips files, directories, or block names during scanning.
# `patterns` use gitignore-style syntax, including `!negation`, trailing `/`,
# `*`, `**`, and character classes.
# [exclude]
# patterns = ["vendor/", "dist/", "generated/", "!generated/keep.md"]
#
# `markdown_codeblocks` only affects fenced code blocks inside source-file
# comments. Supported values:
# - false        -> process tags in code blocks normally (default)
# - true         -> ignore tags in all fenced code blocks
# - "..."        -> ignore code blocks whose info string contains that text
# - ["...", ...] -> ignore code blocks matching any listed info-string text
# markdown_codeblocks = true
# markdown_codeblocks = "ignore"
# markdown_codeblocks = ["ignore", "example"]
#
# `blocks` excludes specific block names everywhere, even if their files are
# still scanned.
# blocks = ["draftSection", "deprecatedApi"]

# `[include]` narrows scanning to only matching files. Use it when you want a
# smaller, more predictable scan surface in large repos.
# [include]
# patterns = ["docs/**/*.rs", "src/**/*.ts", "packages/*/readme.md"]

# `[templates]` restricts where `*.t.md` provider files are discovered.
# Leave it commented to allow template discovery anywhere in the project.
# [templates]
# paths = [".templates", "shared/templates"]

# `[[formatters]]` makes `mdt update` and `mdt check` converge with your
# formatter's canonical output.
#
# This is the recommended fix when `mdt update`, your formatter, and
# `mdt check` would otherwise bounce back and forth in CI.
#
# Formatter `command` values are rendered with minijinja before execution.
# Available variables:
# - `{{ filePath }}`         -> absolute path to the file being formatted
# - `{{ relativeFilePath }}` -> path relative to the project root
# - `{{ rootDirectory }}`    -> absolute path to the project root
#
# `patterns` and `ignore` are both ordered gitignore-style rule lists. A
# leading `!` negates a prior match, so later rules can re-include paths.
#
# Start with one catch-all formatter when your repo already uses a router like
# dprint. Add more entries when different file types need different tools.
# [[formatters]]
# command = "dprint fmt --stdin \"{{ filePath }}\""
# patterns = ["**/*.md"]
# ignore = ["**/*.t.md", "**/*.snap"]
#
# [[formatters]]
# command = "prettier --stdin-filepath \"{{ filePath }}\""
# patterns = ["**/*.ts", "**/*.tsx"]
# ignore = ["dist/**"]
"####;
// <!-- {/mdtInitAnnotatedConfigurationRust} -->
