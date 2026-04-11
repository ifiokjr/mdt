<!-- {@mdtAnnotatedConfiguration} -->

{% raw %}

# mdt.toml

# 

# This file is intentionally verbose: active entries show one working setup,

# and commented entries document every configuration option currently

# supported by the codebase.

# 

# Rule for contributors: when config behavior changes, update this annotated

# file and the synced configuration guide in the same PR.

# Top-level safety limit for scanned files, in bytes.

# Omit this to use the built-in default of 10 MB.

# Raise it for unusually large generated docs; lower it if you want earlier

# failure on oversized files.

# max_file_size = 10485760

# By default mdt respects `.gitignore` so it behaves like the repo itself.

# Set this to `true` only when ignored/generated files should still be scanned,

# or when you want `[include]` and `[exclude]` to be the only scanning rules.

# disable_gitignore = true

# Padding controls the blank lines between tags and injected content.

# Supported values for both `before` and `after`:

# - false -> keep content inline with the tag

# - 0 -> move content to the next line with no blank line

# - 1 -> one blank line

# - 2+ -> two or more blank lines

# 

# This repo uses `0`/`0` because it keeps comment-based targets formatter-stable

# without introducing extra blank lines for dprint/rustfmt to rewrite.

[padding] before = 0 after = 0

# `[check]` controls how `mdt check` compares expected vs actual content.

# - "strict" -> byte-for-byte comparison (default)

# - "lenient" -> whitespace-normalized comparison; ignores differences in

# blank line count, trailing whitespace, and table/JSON formatting so

# external formatters do not cause false staleness.

# 

# `mdt update` always writes exact bytes regardless of this setting.

# 

# This repo uses `lenient` so that dprint can reformat generated targets

# without tripping `mdt check`.

[check] comparison = "lenient"

[data]

# String values are file-backed namespaces.

# The parser is inferred from the extension: `.json`, `.toml`, `.yaml`,

# `.yml`, `.kdl`, and `.ini` are supported.

# 

# This repo exposes Cargo metadata as `{{ cargo.package.* }}` so templates can

# stay synchronized with workspace package information.

cargo = "Cargo.toml"

# Typed data sources let you force a parser when the extension is missing,

# unusual, or intentionally generic.

# release = { path = "release-info", format = "json" }

# Script-backed data sources shell out and parse stdout.

# `format` accepts: `text`, `string`, `raw`, `txt`, `json`, `toml`, `yaml`,

# `yml`, `kdl`, or `ini`.

# `watch` lists files that invalidate the cached result in

# `.mdt/cache/data-v1.json`.

# 

# Use this when the source of truth comes from tooling instead of a checked-in

# file.

# version = { command = "cat VERSION", format = "text", watch = ["VERSION"] }

# git = { command = "git rev-parse --short HEAD", format = "text" }

[exclude]

# Gitignore-style patterns skip files or directories during scanning.

# Supports `!negation`, trailing `/` for directories, `*`, `**`, and character

# classes.

# 

# This repo excludes test-only fixtures and snapshot directories so mdt only

# scans files that can contain real, maintained blocks.

patterns = [ "**/tests/", "**/__tests.rs", "**/snapshots/", ]

# `markdown_codeblocks` only affects fenced code blocks that appear inside

# source-file comments. It exists so docs/examples can show mdt tags without

# accidentally turning those examples into live targets.

# 

# Supported values:

# - false -> process tags in fenced code blocks normally (default)

# - true -> ignore tags in all fenced code blocks

# - "..." -> ignore code blocks whose info string contains that substring

# - ["...", ...] -> ignore code blocks whose info string matches any substring

# 

# This repo uses `true` because source-comment examples should stay

# illustrative, not executable.

markdown_codeblocks = true

# `blocks` excludes specific block names everywhere, even if their files are

# scanned. Use it when a block name is temporary, experimental, or

# intentionally unmanaged.

# blocks = ["draftSection", "experimentalApi"]

# `include` narrows scanning to only matching files. Use it to opt into a

# smaller search space in large repos once you know exactly where mdt tags live.

# [include]

# patterns = ["docs/**/*.rs", "src/**/_.ts", "packages/_/readme.md"]

# `templates.paths` restricts where `*.t.md` source files are discovered.

# Leave it unset to find template files anywhere in the project.

# Use it when a repo wants a dedicated source-of-truth directory layout.

# [templates]

# paths = [".templates", "shared/templates"]

# `[[formatters]]` lets `mdt update` and `mdt check` compare against your

# formatter's canonical output instead of raw injected text.

# 

# Formatter commands are rendered with minijinja before execution.

# Available variables:

# - `{{ filePath }}` -> absolute path to the file being formatted

# - `{{ relativeFilePath }}` -> path relative to the project root

# - `{{ rootDirectory }}` -> absolute path to the project root

# 

# `patterns` and `ignore` are both ordered rule lists with gitignore-like

# globs. A leading `!` negates a prior match, so later rules can re-include

# paths.

# 

# This repo enables dprint for generated markdown targets to prevent the

# formatter cycle from issue #46, where `mdt update` and `dprint fmt` would

# otherwise keep disagreeing in CI.

[[formatters]] command = "dprint fmt --stdin \"{{ filePath }}\"" patterns = ["**/*.md"] ignore = ["**/*.t.md"]

# Add more formatter stages when different file types need different tools.

# [[formatters]]

# command = "prettier --stdin-filepath \"{{ filePath }}\""

# patterns = ["**/*.ts", "**/*.tsx"]

{% endraw %}

<!-- {/mdtAnnotatedConfiguration} -->

<!-- {@mdtInitAnnotatedConfiguration} -->

{% raw %}

# mdt.toml

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

# - 0 -> put content on the next line with no blank line

# - 1 -> add one blank line

# - 2+ -> add two or more blank lines

# 

# Recommended when your targets live in source-code comments or when formatters

# tend to rewrite surrounding whitespace.

# [padding]

# before = 0

# after = 0

# `[check]` controls how `mdt check` compares expected vs actual content.

# - "strict" -> byte-for-byte comparison (default)

# - "lenient" -> whitespace-normalized comparison; ignores differences in

# blank line count, trailing whitespace, and table/JSON formatting so

# external formatters do not cause false staleness.

# `mdt update` always writes exact bytes regardless of this setting.

# [check]

# comparison = "lenient"

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

# - false -> process tags in code blocks normally (default)

# - true -> ignore tags in all fenced code blocks

# - "..." -> ignore code blocks whose info string contains that text

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

# patterns = ["docs/**/*.rs", "src/**/_.ts", "packages/_/readme.md"]

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

# - `{{ filePath }}` -> absolute path to the file being formatted

# - `{{ relativeFilePath }}` -> path relative to the project root

# - `{{ rootDirectory }}` -> absolute path to the project root

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

{% endraw %}

<!-- {/mdtInitAnnotatedConfiguration} -->

<!-- {@mdtInitAnnotatedConfigurationRust} -->

{% raw %} pub(crate) const DEFAULT_MDT_TOML: &str = r#"# mdt.toml

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

# - 0 -> put content on the next line with no blank line

# - 1 -> add one blank line

# - 2+ -> add two or more blank lines

# 

# Recommended when your targets live in source-code comments or when formatters

# tend to rewrite surrounding whitespace.

# [padding]

# before = 0

# after = 0

# `[check]` controls how `mdt check` compares expected vs actual content.

# - "strict" -> byte-for-byte comparison (default)

# - "lenient" -> whitespace-normalized comparison; ignores differences in

# blank line count, trailing whitespace, and table/JSON formatting so

# external formatters do not cause false staleness.

# `mdt update` always writes exact bytes regardless of this setting.

# [check]

# comparison = "lenient"

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

# - false -> process tags in code blocks normally (default)

# - true -> ignore tags in all fenced code blocks

# - "..." -> ignore code blocks whose info string contains that text

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

# patterns = ["docs/**/*.rs", "src/**/_.ts", "packages/_/readme.md"]

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

# - `{{ filePath }}` -> absolute path to the file being formatted

# - `{{ relativeFilePath }}` -> path relative to the project root

# - `{{ rootDirectory }}` -> absolute path to the project root

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

"#; {% endraw %}

<!-- {/mdtInitAnnotatedConfigurationRust} -->
