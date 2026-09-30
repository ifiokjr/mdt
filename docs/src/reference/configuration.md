# Configuration Reference

mdt reads an optional TOML file from the project root. Every key is optional. For a task-oriented walkthrough, see the [Configuration guide](../guide/configuration.md).

## File location

The first file found wins:

1. `mdt.toml`
2. `.mdt.toml`
3. `.config/mdt.toml`

Without `--path`, mdt walks up from the current directory, staying inside the git repository, to the nearest directory containing one of these files and uses it as the project root; outside a git repository, or with none found, the current directory is the root. `mdt init` always uses the current directory or `--path`.

A directory below the root that contains any of these files is a separate project (a sub-project) and is skipped by the parent's scan. See [Monorepos](../advanced/monorepos.md).

## Validation

Unknown keys are rejected, so a typo fails instead of being ignored:

```text
mdt::config_parse

  x failed to parse config file: /path/to/project/mdt.toml: TOML parse error at line 1, column 2
  |   |
  | 1 | [paddding]
  |   |  ^^^^^^^^
  | unknown field `paddding`, expected one of `data`, `exclude`, `include`,
  | `templates`, `max_file_size`, `padding`, `formatters`,
  | `disable_gitignore`, `check`
```

Config errors exit with status 2.

## Overview

| Key                             | Type                   | Default    | Purpose                                                   |
| ------------------------------- | ---------------------- | ---------- | --------------------------------------------------------- |
| `max_file_size`                 | integer (bytes)        | 10485760   | Largest file mdt scans; a larger scanned file is an error |
| `disable_gitignore`             | boolean                | `false`    | Scan files that git ignores                               |
| `[padding] before`, `after`     | `false` or integer     | `0` / `0`  | Blank lines between consumer tags and content             |
| `[check] comparison`            | `"strict"`/`"lenient"` | `"strict"` | How `mdt check` compares consumer content                 |
| `[data]`                        | table                  | none       | Namespaces for template variables in providers            |
| `[exclude] patterns`            | array of strings       | `[]`       | Gitignore-style patterns for files to skip                |
| `[exclude] blocks`              | array of strings       | `[]`       | Block names to ignore everywhere                          |
| `[exclude] markdown_codeblocks` | bool, string, or array | `false`    | Ignore tags in fenced code blocks in source-file comments |
| `[include] patterns`            | array of strings       | `[]`       | Extra files to scan                                       |
| `[templates] paths`             | array of strings       | `[]`       | Extra directories to read `*.t.md` providers from         |
| `[[formatters]]`                | array of tables        | none       | Formatter commands run by `mdt update` and `mdt check`    |

Top-level keys (`max_file_size`, `disable_gitignore`) must appear before the first `[table]` header.

## Which files are scanned

Without configuration, mdt scans:

- Markdown files: `.md`, `.mdx`, `.markdown`.
- Source files: `.rs .ts .tsx .mts .cts .js .jsx .mjs .cjs .py .go .java .kt .swift .c .cc .cpp .cxx .h .hh .hpp .cs .dart`.
- `*.t.md` files anywhere in the project. Only these can hold providers.

It always skips hidden directories (except `.templates`), `node_modules/`, `target/`, and sub-projects. It follows git's ignore rules unless `disable_gitignore = true`.

The scan configuration then applies in this order:

1. `[include] patterns` add files.
2. `[templates] paths` add `*.t.md` files from other directories.
3. Git ignore rules and `[exclude] patterns` remove files.

mdt writes its cache to `.mdt/` in the project root. Add `.mdt/` to `.gitignore`; `mdt init` does this in git repositories.

## `max_file_size`

```toml
max_file_size = 10485760
```

A scanned file larger than the limit is an error that names the file (`mdt::file_too_large`); it is never skipped silently. Raise the limit or exclude the file.

## `disable_gitignore`

```toml
disable_gitignore = true
```

By default mdt applies git's ignore rules with git's precedence (deeper files win):

- `.gitignore` files in the project, including nested ones.
- `.gitignore` files in parent directories up to the git repository root.
- `.git/info/exclude`.

Outside a git repository, only the `.gitignore` files inside the project apply.

`true` turns all of this off. Hidden directories, `node_modules/`, `target/`, and sub-projects are still skipped, and `[exclude]` still applies.

## `[padding]`

Blank lines between a consumer's tags and its content. Applies to markdown and source-file consumers alike.

```toml
[padding]
before = 0
after = 0
```

| Value   | Result                                            |
| ------- | ------------------------------------------------- |
| `false` | Content on the same line as the tag               |
| `0`     | Content on the next line, no blank line (default) |
| `1`     | One blank line                                    |
| `2`, …  | That many blank lines                             |

- Without a `[padding]` section, both values are `0`. With the section present, an omitted key defaults to `1`.
- Padding is added on top of the rendered content's own leading and trailing newlines. The usual provider style has blank lines around its content, so add `|trim` to a consumer when you need exact output.
- In source files, blank lines and the closing tag keep the comment prefix, and an indented closing tag keeps its indentation.
- Inline blocks (`{~name}`) are never padded.

With the default padding and `|trim`, a Rust consumer renders as:

```rust
//! <!-- {=docs|trim|linePrefix:"//! ":true} -->
//! This content stays inside the doc comment.
//! <!-- {/docs} -->
```

With `before = 1` and `after = 1`:

```rust
//! <!-- {=docs|trim|linePrefix:"//! ":true} -->
//!
//! This content stays inside the doc comment.
//!
//! <!-- {/docs} -->
```

Upgrading from a release where the default glued the closing tag to the last content line makes affected consumers stale once; run `mdt update`.

## `[check]`

```toml
[check]
comparison = "lenient"
```

| Value                | Behavior                                                                                   |
| -------------------- | ------------------------------------------------------------------------------------------ |
| `"strict"` (default) | Byte-for-byte comparison                                                                   |
| `"lenient"`          | Trims trailing whitespace on every line and collapses runs of blank lines before comparing |

Lenient mode does not normalize indentation, table padding, or JSON layout; use [`[[formatters]]`](#formatters) for those. `mdt update` always writes exact bytes.

## `[data]`

Maps namespaces to data sources. Each key becomes a template variable namespace in providers (`{{ key.field }}`).

```toml
[data]
package = "package.json"
settings = { path = "settings", format = "yaml" }
version = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

| Entry form                                           | Meaning                          |
| ---------------------------------------------------- | -------------------------------- |
| `"path"`                                             | File; parser chosen by extension |
| `{ path = "...", format = "..." }`                   | File with an explicit parser     |
| `{ command = "...", format = "...", watch = [...] }` | Command whose stdout is parsed   |

| Format                               | Extensions      | Notes                                             |
| ------------------------------------ | --------------- | ------------------------------------------------- |
| `json`                               | `.json`         |                                                   |
| `toml`                               | `.toml`         | Integers stay integers (`8080`)                   |
| `yaml`, `yml`                        | `.yaml`, `.yml` |                                                   |
| `kdl`                                | `.kdl`          | Repeated nodes with the same name become an array |
| `ini`                                | `.ini`          |                                                   |
| `text` (also `string`, `raw`, `txt`) | `.txt`          | One trailing newline is dropped                   |

Paths are relative to the project root. Errors stop the command with exit status 2, for example:

```text
mdt::data_file

  x failed to load data file `missing.json`: No such file or directory (os
  | error 2)
```

```text
mdt::unsupported_format

  x unsupported data file format: `xml`
  help: supported formats: text, json, toml, yaml, yml, kdl, ini
```

### Script-backed data sources

<!-- {=mdtScriptDataSourcesGuide} -->

A `[data]` entry can run a shell command and parse its stdout. Use it for values that come from tooling, such as git metadata, Nix, or a generated version file.

```toml
[data]
release = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

- `command`: shell command, run from the project root.
- `format`: parser for stdout: `text` (also `string`, `raw`, `txt`), `json`, `toml`, `yaml`, `yml`, `kdl`, or `ini`. Text drops one trailing newline, so `cat VERSION` renders `1.2.3`, not `1.2.3` plus a line break.
- `watch`: files whose changes invalidate the cached output.

<!-- {/mdtScriptDataSourcesGuide} -->

<!-- {=mdtScriptDataSourcesNotes} -->

- Output is cached in `.mdt/cache/data-v1.json`, keyed by namespace, command, format, and watch list. mdt reuses it while the watched files are unchanged.
- Caching needs every `watch` entry to be an existing file. Without `watch`, or when an entry is missing, a glob, or a directory, the command runs on every mdt run.
- A command that exits non-zero fails the run with `mdt::data_script` (exit status 2).

<!-- {/mdtScriptDataSourcesNotes} -->

### Rendering rules

- Provider content is rendered with minijinja only when `[data]` is configured. Without `[data]`, `{{ ... }}` is copied as written, and mdt warns when a used provider contains namespaced variables such as `{{ pkg.version }}`. A sub-project that reuses shared providers needs its own `[data]`; its paths may point outside it (`"../package.json"`).
- With `[data]`, every provider is a template. Wrap literal `{{` or `{%` text, such as GitHub Actions `${{ secrets.TOKEN }}`, in `{% raw %}...{% endraw %}`.
- Undefined variables render as empty text, and `mdt check`/`mdt update` warn about them.
- Only minijinja's built-in filters are available. See [Data Interpolation](../guide/data-interpolation.md).

## `[exclude]`

```toml
[exclude]
patterns = ["vendor/", "generated/*", "!generated/keep.md"]
blocks = ["draftSection"]
markdown_codeblocks = true
```

**`patterns`**: gitignore-style patterns relative to the project root, with `!` negation, trailing `/` for directories, `*`, `**`, and character classes. As in git, a file inside an excluded directory cannot be re-included: exclude the directory's contents (`generated/*`) rather than the directory (`generated/`) when you need a `!` exception.

**`blocks`**: block names to remove from the project. Providers and consumers with these names are ignored entirely: consumers are never filled or checked, and the blocks produce no diagnostics.

**`markdown_codeblocks`**: ignore tags inside fenced code blocks that appear in **source-file comments**.

| Value             | Tags ignored in                                                 |
| ----------------- | --------------------------------------------------------------- |
| `false` (default) | No fenced code blocks                                           |
| `true`            | Every fenced code block                                         |
| `"text"`          | Fenced code blocks whose info string contains `text`            |
| `["a", "b"]`      | Fenced code blocks whose info string contains any listed string |

In markdown files, tags inside fenced code blocks and inline code spans are always inert, whatever this setting says.

## `[include]`

```toml
[include]
patterns = ["scripts/**/*.rb"]
```

**`patterns`**: globs of extra files to scan. `[include]` only adds to the default scan; it never narrows it. Included files still follow git ignore rules and `[exclude]`, and hidden directories stay skipped. Invalid globs are rejected.

Included files that are not markdown are read with the source-file scanner, which finds tags in any comment. Keep patterns specific: a broad glob such as `src/**` also matches binary files, and a file that is not valid UTF-8 fails the run with `mdt::read_file`.

## `[templates]`

```toml
[templates]
paths = [".github/templates", "../shared/templates"]
```

**`paths`**: directories, relative to the project root, whose `*.t.md` files are read as providers. Only `*.t.md` files are added from these directories.

- `*.t.md` files inside the project are always providers; `paths` never restricts that.
- Use it for hidden directories (other than `.templates`) and for providers shared from outside the project. Providers from a directory outside the project are never reported as unused, since other projects may consume them.
- A path that is not a directory is an error (`mdt::templates_path`).

## `[[formatters]]`

<!-- {=mdtFormatterPipelineDocs} -->

`[[formatters]]` entries run your formatter inside `mdt update` and `mdt check`. `mdt update` writes formatted files, and `mdt check` compares against formatted output, so the `mdt update` → formatter → `mdt check` loop settles instead of reporting stale blocks after every format.

```toml
[[formatters]]
command = "dprint fmt --stdin \"{{ filePath }}\""
patterns = ["**/*.md"]
ignore = ["**/*.t.md"]
```

Each matching entry:

- reads the whole file on stdin and writes the formatted file to stdout
- runs from the project root through `sh -c` (`cmd /C` on Windows)
- runs after block injection in `mdt update`, and before comparison in `mdt check`
- runs in declaration order when several entries match the same file

`command` can use three placeholders. mdt passes their values as the environment variables `MDT_FILE_PATH`, `MDT_RELATIVE_FILE_PATH`, and `MDT_ROOT_DIRECTORY`, so the shell never parses a file name. Keep placeholders inside double quotes; inside single quotes they stay literal.

- `{{ filePath }}`: absolute path of the file being formatted
- `{{ relativeFilePath }}`: path relative to the project root
- `{{ rootDirectory }}`: absolute project root

`patterns` and `ignore` are ordered lists of plain globs, not gitignore patterns. A `!` entry negates an earlier match. `vendor/` matches nothing inside the directory; write `vendor/**`.

A formatter that fails or exits non-zero is an error (exit status 2); mdt never falls back to unformatted output. Without `[[formatters]]`, mdt runs no formatter.

Keep `*.t.md` files out of formatter scope, both in `ignore` and in the formatter's own config (dprint `excludes`, `.prettierignore`): markdown formatters rewrite provider text such as `#` lines and `**` globs. CI must install the same formatter versions you use locally.

<!-- {/mdtFormatterPipelineDocs} -->

Verified commands:

| Formatter | `command`                                    |
| --------- | -------------------------------------------- |
| dprint    | `dprint fmt --stdin "{{ filePath }}"`        |
| Prettier  | `prettier --stdin-filepath "{{ filePath }}"` |
| rustfmt   | `rustfmt --edition 2021`                     |
| gofmt     | `gofmt`                                      |

Do not pass rustfmt a path: `rustfmt --emit stdout <path>` reads the file on disk and prints a file-name header, which corrupts the output.

## Examples

A config that uses most options:

```toml
max_file_size = 10485760

[padding]
before = 0
after = 0

[check]
comparison = "strict"

[data]
package = "package.json"
version = { command = "cat VERSION", format = "text", watch = ["VERSION"] }

[exclude]
patterns = ["vendor/", "generated/*", "!generated/keep.md"]
blocks = ["draftSection"]
markdown_codeblocks = true

[include]
patterns = ["scripts/**/*.rb"]

[templates]
paths = [".github/templates"]

[[formatters]]
command = "prettier --stdin-filepath \"{{ filePath }}\""
patterns = ["**/*.md", "**/*.ts"]
ignore = ["**/*.t.md"]
```

A minimal config for data interpolation:

```toml
[data]
package = "package.json"
```

## Defaults without a config file

- No template rendering: `{{ ... }}` in providers is copied as written.
- The default scan and built-in skips, plus git ignore rules.
- `*.t.md` files anywhere in the project are providers.
- Padding `0` / `0`.
- Strict comparison in `mdt check`.
- No formatters.
- `max_file_size` of 10 MB.
