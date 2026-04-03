# Configuration

mdt is configured through an `mdt.toml` file placed in the project root. Configuration is optional — mdt works without it using sensible defaults.

## Creating a config file

Create `mdt.toml` in your project root:

```toml
[data]
package = "package.json"

[exclude]
patterns = ["vendor/", "dist/"]
```

## What `mdt init` writes

`mdt init` creates a fully annotated starter `mdt.toml` so new projects can see every currently supported option before uncommenting anything.

<!-- {=mdtInitAnnotatedConfiguration|trim|codeBlock:"toml"} -->

```toml
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
```

<!-- {/mdtInitAnnotatedConfiguration} -->

## Sections

### `[data]` — Data file mappings

Maps namespace names to data files. Each entry makes the file's contents available as template variables under that namespace.

```toml
[data]
package = "package.json"
cargo = "Cargo.toml"
config = "config.yaml"
```

This creates three namespaces:

- `{{ package.name }}` reads from `package.json`
- `{{ cargo.package.version }}` reads from `Cargo.toml`
- `{{ config.database.host }}` reads from `config.yaml`

Paths are relative to the project root (where `mdt.toml` lives).

**Supported formats:** JSON, TOML, YAML (`.yaml`/`.yml`), and KDL.

See [Data Interpolation](./data-interpolation.md) for full details.

### `[exclude]` — Exclude patterns

Patterns for files and directories to skip during scanning. Uses **gitignore-style syntax** — the same pattern format as `.gitignore` files, including negation (`!`), directory markers (`/`), wildcards (`*`, `**`), and character classes.

```toml
[exclude]
patterns = [
	"vendor/",
	"dist/",
	"generated/",
	"**/*.generated.md",
	"!generated/keep-this.md",
]
```

These patterns are checked relative to the project root. In addition to your explicit patterns, mdt always skips hidden directories (`.git`, `.vscode`, etc.), `node_modules/`, and `target/`.

#### `markdown_codeblocks` — Skip tags in code blocks

Controls whether mdt tags inside fenced code blocks in **source-file comments** are processed. This is useful when doc comments contain fenced examples that show mdt tag syntax but should not be treated as real tags.

```toml
[exclude]
# Skip tags inside ALL fenced code blocks
markdown_codeblocks = true

# Skip only code blocks whose info string contains "ignore"
markdown_codeblocks = "ignore"

# Skip code blocks whose info string contains any of these
markdown_codeblocks = ["ignore", "example"]
```

The default is `false`, meaning tags in fenced source-comment code blocks are processed normally.

#### `blocks` — Exclude specific block names

Array of block names to exclude. Any block (source or target) whose name appears in this list is completely ignored.

```toml
[exclude]
blocks = ["draft-section", "deprecated-api"]
```

### `[include]` — Include patterns

Restrict scanning to only files matching these patterns:

```toml
[include]
patterns = ["docs/**/*.rs", "src/**/*.ts"]
```

When set, only files matching at least one include pattern are scanned (in addition to markdown and template files which are always included).

### `[templates]` — Template search paths

By default, mdt finds `*.t.md` files anywhere in the project. You can restrict where it looks:

```toml
[templates]
paths = ["templates", "shared/docs"]
```

When set, only `*.t.md` files within these directories are recognized as template files.

### `[padding]` — Block content padding

Controls blank lines between block tags and their content. This is recommended when using target blocks in source code files.

```toml
[padding]
before = 0
after = 0
```

`before` and `after` accept `false` (inline), `0` (next line), `1` (one blank line), `2`, etc. When `[padding]` is present but values are omitted, they default to `1`. In source code files, blank lines use the same comment prefix as surrounding lines (e.g., `//!`, `///`, `*`).

Without this setting, transformers like `trim` can cause content to merge directly into the surrounding tags, breaking the structure of code comments.

**Recommended for projects with formatters:** Use `before = 0, after = 0` to minimize whitespace that formatters might alter.

### `[[formatters]]` — Formatter-aware update/check pipeline

Use formatter entries to make `mdt update` and `mdt check` converge with your project's formatter.

```toml
[[formatters]]
command = "dprint fmt --stdin \"{{ filePath }}\""
patterns = ["**"]
ignore = ["**/*.snap", "docs/generated/**"]

[[formatters]]
command = "prettier --stdin-filepath \"{{ filePath }}\""
patterns = ["**/*.ts", "**/*.tsx"]
```

Each formatter entry:

- reads the full candidate file from stdin
- writes the full formatted file to stdout
- runs from the project root
- applies to files whose relative path matches any of its `patterns`
- skips files whose relative path matches any of its `ignore` patterns
- evaluates both lists in order, with leading `!` entries acting as negation rules

If multiple formatter entries match the same file, they run in declaration order.

Use `ignore` when a formatter should generally apply to a file type but skip specific paths:

```toml
[[formatters]]
command = "dprint fmt --stdin \"{{ filePath }}\""
patterns = ["**/*.md", "!docs/generated/**"]
ignore = ["vendor/**", "docs/generated/**", "!docs/generated/keep.md"]
```

Here:

- `patterns` includes markdown files, but excludes `docs/generated/**`
- `ignore` excludes `vendor/**` and most generated docs
- `!docs/generated/keep.md` re-allows one ignored path for this formatter entry

This integration applies to both:

- `mdt update` — after target content is injected
- `mdt check` — before expected output is compared to the file on disk

That means `mdt update → formatter → mdt check` should converge without extra repair loops.

#### Minijinja variables available to formatter commands

Formatter commands are rendered with minijinja before execution.

- `{{ filePath }}` — absolute path to the file being formatted
- `{{ relativeFilePath }}` — path relative to the project root
- `{{ rootDirectory }}` — absolute project root

#### Recommended patterns

If you already use a formatter router like dprint, a single catch-all entry is often enough:

```toml
[[formatters]]
command = "dprint fmt --stdin \"{{ filePath }}\""
patterns = ["**"]
```

If you use separate tools per file type, add multiple entries in the order you want them applied.

### `max_file_size` — Safety limit for scanned files

Set the maximum file size (in bytes) that mdt will scan. Files larger than this limit return an error.

```toml
max_file_size = 10485760 # 10 MB
```

If omitted, mdt uses a default of `10 MB`.

### `disable_gitignore` — Disable `.gitignore` integration

By default, mdt respects `.gitignore` rules when scanning for files, skipping anything that git would ignore. Set `disable_gitignore = true` to turn off this behavior:

```toml
disable_gitignore = true
```

When this option is enabled, mdt scans all files regardless of `.gitignore` rules. You can still control which files are scanned using the `[exclude]` and `[include]` sections.

**When to use this:**

- **Generated files with mdt blocks** — If your build output or generated files contain target blocks that need updating, those files are typically listed in `.gitignore` but still need to be scanned by mdt.
- **Working outside a git repository** — If the project is not a git repo, `.gitignore` resolution can cause unnecessary overhead or errors. Disabling it avoids those issues.
- **Full control over scanning** — When you prefer to manage file inclusion/exclusion entirely through `[exclude]` and `[include]` patterns rather than relying on `.gitignore`.

If omitted, defaults to `false` (`.gitignore` rules are respected).

## Sub-project boundaries

If mdt encounters a directory containing its own `mdt.toml`, it treats that directory as a separate project and skips it. This is useful in monorepos where each package manages its own templates:

```
my-monorepo/
  mdt.toml                    # root project config
  .templates/
    template.t.md
  packages/
    lib-a/
      mdt.toml                # lib-a is a separate mdt project
      .templates/
        template.t.md
    lib-b/
      mdt.toml                # lib-b is a separate mdt project
      .templates/
        template.t.md
```

Running `mdt update` from the root updates only the root project's targets. Each sub-project is managed independently.

## Annotated `mdt.toml` reference

The example below is synced from the repository's annotated `mdt.toml` so the config reference and the real config evolve together.

<!-- {=mdtAnnotatedConfiguration|trim|codeBlock:"toml"} -->

```toml
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
# - 0     -> move content to the next line with no blank line
# - 1     -> one blank line
# - 2+    -> two or more blank lines
#
# This repo uses `0`/`0` because it keeps comment-based targets formatter-stable
# without introducing extra blank lines for dprint/rustfmt to rewrite.
[padding]
before = 0
after = 0

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
patterns = [
	"**/tests/",
	"**/__tests.rs",
	"**/snapshots/",
]

# `markdown_codeblocks` only affects fenced code blocks that appear inside
# source-file comments. It exists so docs/examples can show mdt tags without
# accidentally turning those examples into live targets.
#
# Supported values:
# - false        -> process tags in fenced code blocks normally (default)
# - true         -> ignore tags in all fenced code blocks
# - "..."        -> ignore code blocks whose info string contains that substring
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
# patterns = ["docs/**/*.rs", "src/**/*.ts", "packages/*/readme.md"]

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
# - `{{ filePath }}`         -> absolute path to the file being formatted
# - `{{ relativeFilePath }}` -> path relative to the project root
# - `{{ rootDirectory }}`    -> absolute path to the project root
#
# `patterns` and `ignore` are both ordered rule lists with gitignore-like
# globs. A leading `!` negates a prior match, so later rules can re-include
# paths.
#
# This repo enables dprint for generated markdown targets so docs stay in sync
# with the same formatter used elsewhere in the workspace.
[[formatters]]
command = "dprint fmt --stdin \"{{ filePath }}\""
patterns = ["**/*.md"]
ignore = ["**/*.t.md"]

# Add more formatter stages when different file types need different tools.
# [[formatters]]
# command = "prettier --stdin-filepath \"{{ filePath }}\""
# patterns = ["**/*.ts", "**/*.tsx"]
```

<!-- {/mdtAnnotatedConfiguration} -->
