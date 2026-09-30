# mdt Reference

The complete reference for the mdt skill. Print it with `mdt skill --reference`.

## Tags

Every tag is an HTML comment, so tags are invisible in rendered markdown.

```text
<!-- {@name} -->   provider opening tag (only in *.t.md files)
<!-- {=name} -->   consumer opening tag (markdown and source files)
<!-- {~name:"{{ template }}"} -->   inline opening tag
<!-- {/name} -->   closing tag (all block types)
```

- **Names** match `[A-Za-z_][A-Za-z0-9_-]*` and are case-sensitive (`installCommand`, `install-command`, `_private`). camelCase is the convention.
- **The sigil must follow `{` directly.** Whitespace after the sigil is fine (`{@ name }`), but `{ @name }`, `{=my.block}`, and `{=1starts}` are not tags. In markdown files mdt reports such comments as `mdt::invalid_tag` errors (`--ignore-invalid-names` skips the check).
- **Transformers** follow the name, separated by `|`: `<!-- {=name|trim|codeBlock:"sh"} -->`.
- **Arguments** follow the name, separated by `:` — see [Block arguments](#block-arguments).

### Providers

```markdown
<!-- {@installCommand} -->

npm install acme-http

<!-- {/installCommand} -->
```

- Providers are read only from files whose name ends in `.t.md` (canonical location: `.templates/`). A provider tag anywhere else is ignored and reported as the warning `mdt::provider_outside_template`.
- Names are unique across the project; a duplicate is an error that names both files.
- The provider's content is everything between its tags, including the surrounding blank lines. Consumers usually add `|trim`.
- Blocks cannot be nested, not even inside a provider: its tags would be copied into every consumer, where they nest (`mdt::nested_block`). Reference data with `{{ ... }}` instead.

### Consumers

```markdown
<!-- {=installCommand|trim|codeBlock:"sh"} -->
<!-- {/installCommand} -->
```

- `mdt update` replaces everything between the tags; never edit consumer content by hand.
- A consumer whose name matches no provider is an **orphan**: `mdt check` fails (exit 1) and suggests similar provider names.
- A block inside another block is an error (`mdt::nested_block`).

### Inline blocks

Inline blocks render their first argument as a minijinja template with the `[data]` context — no provider needed.

```markdown
Install version <!-- {~version:"{{ pkg.version }}"} -->0.0.0<!-- {/version} --> today.

| Package | Version                                                 |
| ------- | ------------------------------------------------------- |
| acme    | <!-- {~ver:"{{ pkg.version }}"} -->0.0.0<!-- {/ver} --> |
```

- Padding never applies to inline blocks, so they stay on one line.
- In a table cell, leave out `|` transformers: GFM splits the row on `|` before mdt sees the comment, and the tag is not recognized.

### Block arguments

Providers declare parameters; consumers pass string values in the same order. Parameters become template variables.

```markdown
<!-- {@installCmd:"manager":"package"} -->

{{ manager }} install {{ package }}

<!-- {/installCmd} -->

<!-- {=installCmd:"npm":"acme-http"|trim} -->
<!-- {/installCmd} -->
```

A consumer that passes a different number of arguments than the provider declares is a render error (`mdt check` exit 1).

### Examples in markdown

Tags inside fenced code blocks and inline code spans in markdown files are inert. An example that itself contains a `` ``` `` fence closes a 3-backtick outer fence early, so everything after it becomes live; use a 4-backtick outer fence.

In **source files**, fences inside comments are live by default. Set `[exclude] markdown_codeblocks = true` to make tags inside fenced blocks in comments inert.

## Transformers

Applied left to right after the provider renders. Arguments are double-quoted strings with escapes (`\n`, `\t`, `\"`, `\\`, `\u{…}`); single-quoted strings do not decode escapes. Numbers and booleans are converted to text where a string is expected, so `indent:4` prepends the literal `4`. Every name also accepts a snake_case alias (`line_prefix`, `trim_start`, `code_block`).

| Transformer  | Arguments               | Effect                                                                                                                    |
| ------------ | ----------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `trim`       | —                       | Strip whitespace from both ends                                                                                           |
| `trimStart`  | —                       | Strip leading whitespace                                                                                                  |
| `trimEnd`    | —                       | Strip trailing whitespace                                                                                                 |
| `indent`     | `string` [, `bool`]     | Prefix non-empty lines; `true` also prefixes empty lines with the full string (trailing spaces kept)                      |
| `linePrefix` | `string` [, `bool`]     | Prefix non-empty lines; `true` also prefixes empty lines with the prefix's trailing whitespace trimmed (`"//! "` → `//!`) |
| `lineSuffix` | `string` [, `bool`]     | Suffix non-empty lines; `true` also suffixes empty lines with leading whitespace trimmed                                  |
| `prefix`     | `string`                | Prepend once                                                                                                              |
| `suffix`     | `string`                | Append once                                                                                                               |
| `wrap`       | `string`                | Prepend and append                                                                                                        |
| `codeBlock`  | [`language`]            | Fenced code block; the fence outruns any backtick run in the content                                                      |
| `code`       | —                       | Inline code; the delimiter avoids backtick runs in the content                                                            |
| `replace`    | `search`, `replacement` | Replace every occurrence (an empty `search` does nothing)                                                                 |
| `if`         | `data.path`             | Keep content when the dotted `[data]` path is truthy (not missing, `false`, `null`, `""`, or `0`); otherwise empty        |

`if` takes a path, not an expression: `if:"pkg.private"` works, `if:"pkg.version == '1.0'"` is always false.

### Comment prefixes by language

Write the tag lines with the comment prefix, and re-apply the same prefix to the content with `linePrefix`:

| Comment                             | Consumer tag transformers                      |
| ----------------------------------- | ---------------------------------------------- |
| Rust crate docs (`//!`)             | `trim\|linePrefix:"//! ":true`                 |
| Rust item docs (`///`)              | `trim\|linePrefix:"/// ":true`                 |
| Rust item docs inside an `impl`     | `trim\|linePrefix:"    /// ":true`             |
| Rust crate docs with a shell sample | `trim\|codeBlock:"sh"\|linePrefix:"//! ":true` |
| TypeScript/JavaScript JSDoc         | `trim\|linePrefix:" * ":true`                  |
| JSDoc on a class method             | `trim\|linePrefix:"   * ":true`                |
| Go, Java, Kotlin, Swift, C# (`//`)  | `trim\|linePrefix:"// ":true`                  |
| Python (`#`)                        | `trim\|linePrefix:"# ":true`                   |
| Dart (`///`)                        | `trim\|linePrefix:"/// ":true`                 |

- Always pass `true`: without it, blank lines lose the prefix (invalid Rust/Python, split Go comment groups).
- Put nesting indentation inside the prefix; otherwise formatters re-indent the lines and `mdt check` loops.
- Prefer `linePrefix` over `indent` for comments — `indent:" * ":true` leaves `*` with a trailing space on blank lines, which formatters strip.
- Tag lines keep the prefix they were written with; the transformer's prefix applies to content lines. Use line comments (or one `/** */` block); per-line block comments (`/* <!-- ... --> */`) do not round-trip.
- Provider text containing `*/` ends a surrounding `/* */` comment (a `**/*.ts` glob does). Escape it with `replace:"*\u{2f}":"*\\/"` — writing `*/` inside the tag would close the comment the tag sits in.
- `go doc` and Python's `help()` show HTML comments verbatim; rustdoc, dartdoc, and TSDoc hide them.

## Padding

Padding controls the lines between the tags and the content.

- **Default** (no `[padding]` section): `before = 0`, `after = 0` — content starts on the line after the opening tag and the closing tag starts on its own line, keeping its comment prefix (`//! <!-- {/x} -->`).
- **Values:** `false` keeps content on the tag's line; `0` puts it on the next line; `1` adds one blank line; `2+` adds more. In source files blank padding lines take the comment prefix with trailing whitespace trimmed (`//!`, `///`, `*`).
- **Padding adds to the content's own newlines.** Untrimmed provider content keeps its surrounding blank lines; use `|trim` when you want padding alone to decide.
- With a `[padding]` section, an omitted key defaults to `1`.
- An indented closing tag (in a list item or docstring) keeps its indentation.

```toml
[padding]
before = 0
after = 0
```

## Data interpolation

```toml
[data]
pkg = "package.json" # file; format from the extension
cargo = "crates/core/Cargo.toml" # nested paths are relative to the project root
release = { path = "release-info", format = "json" } # typed file: force a parser
version = { command = "cat VERSION", format = "text", watch = ["VERSION"] } # script
```

| Format | Extensions / `format` values           |
| ------ | -------------------------------------- |
| JSON   | `.json`, `json`                        |
| TOML   | `.toml`, `toml`                        |
| YAML   | `.yaml`, `.yml`, `yaml`, `yml`         |
| KDL    | `.kdl`, `kdl`                          |
| INI    | `.ini`, `ini`                          |
| Text   | `.txt`, `text`, `string`, `raw`, `txt` |

- Namespaces are arbitrary names; access nested keys with dots: `{{ pkg.version }}`, `{{ cargo.package.edition }}`.
- **Text** sources are one string with one trailing newline removed (like shell `$(...)`).
- TOML and KDL integers stay integers (`8080`). Repeated KDL nodes with the same name become an array.
- **Scripts** run from the project root with `sh -c`. With `watch`, output is cached in `.mdt/cache/data-v1.json` until a watched file changes — only while every `watch` entry is an existing file (a typo, glob, or directory disables caching). Without `watch`, the script runs on every command.
- Providers render with [minijinja](https://docs.rs/minijinja) **only when `[data]` is configured**: variables, `{% if %}`, `{% for %}`, and built-in filters (`upper`, `lower`, `title`, `trim`, `replace`, `join`, `length`, `default`, ...). There is no `truncate`.
- Once `[data]` exists, literal `{{ ... }}` and `{% ... %}` in providers are template syntax. Wrap literal examples in `{% raw %}...{% endraw %}` (for example GitHub Actions `${{ secrets.TOKEN }}`).
- Undefined variables render as empty strings; `check` and `update` warn about undefined namespaces.
- Rendering happens before transformers.

## Configuration

mdt reads the first of `mdt.toml`, `.mdt.toml`, `.config/mdt.toml` in the project root. **Unknown keys are rejected** with the key and the config path, so a typo fails loudly. Every key:

```toml
max_file_size = 10485760 # bytes; a scanned file above this is an error naming the file
disable_gitignore = false # true: ignore .gitignore rules (hidden dirs, node_modules, target stay skipped)

[data]
pkg = "package.json"

[padding]
before = 0
after = 0

[check]
comparison = "strict" # or "lenient"

[exclude]
patterns = ["vendor/", "generated/*", "!generated/keep.md"] # gitignore syntax
blocks = ["draftSection"] # block names ignored everywhere, including their diagnostics
markdown_codeblocks = true # source-file comments only: true, "substring", or ["a", "b"]

[include]
patterns = ["**/*.rb"] # extra files to scan (adds; never narrows)

[templates]
paths = ["../../.templates"] # extra directories to read *.t.md providers from

[[formatters]]
command = "dprint fmt --stdin \"{{ filePath }}\""
patterns = ["**/*.md"]
ignore = ["**/*.t.md"]
```

### `[exclude]`

- `patterns` use gitignore syntax on top of `.gitignore`. To keep one file inside an excluded directory, exclude the directory's **contents**: `["generated/*", "!generated/keep.md"]` (excluding `generated/` itself stops the walk, as in git).
- `markdown_codeblocks` affects only fenced blocks inside source-file comments; markdown fences are always inert.
- `blocks` removes those names from the project entirely: providers, consumers, and their diagnostics. A consumer with an excluded name is never filled or checked, so do not use it to silence warnings.

### `[include]`

- Adds files matching the globs to the default scan. It never removes markdown or supported source files. Use it to opt in other extensions.
- Included files respect `.gitignore` and `[exclude]`; hidden directories stay skipped. Non-markdown files are parsed like source files (tags in any comment).
- Avoid broad globs such as `src/**` that match binary files: a file that is not UTF-8 fails with `mdt::read_file`. Invalid globs are rejected when the config loads.

### `[templates]`

- `paths` adds only `*.t.md` files from each directory, relative to the project root. `*.t.md` files elsewhere in the project are always providers too.
- A path may leave the project — for example a monorepo package reading shared providers from `../../.templates`. Files found there are never treated as the package's consumers.
- It is the way to read providers from a hidden directory such as `.github/`.
- A path that is not a directory is an error (`mdt::templates_path`).

### `[check]`

- `"strict"` (default): byte-for-byte comparison.
- `"lenient"`: trims trailing whitespace on each line and collapses runs of blank lines before comparing. It does not normalize indentation, table alignment, or JSON layout — use `[[formatters]]` for those.
- `mdt update` always writes exact bytes.

### `[[formatters]]`

Each matching entry formats the whole candidate file (stdin to stdout, run from the project root with `sh -c`, or `cmd /C` on Windows) after injection in `mdt update` and before comparison in `mdt check`. Entries run in declaration order.

- `command` placeholders `{{ filePath }}` (absolute), `{{ relativeFilePath }}`, and `{{ rootDirectory }}` expand to the environment variables `MDT_FILE_PATH`, `MDT_RELATIVE_FILE_PATH`, and `MDT_ROOT_DIRECTORY`, so file names are never parsed by the shell. Keep placeholders in double quotes; inside single quotes they stay literal. `"$MDT_FILE_PATH"` works directly too.
- `patterns` and `ignore` are ordered glob lists; a leading `!` negates an earlier match. They are plain globs, not gitignore rules: write `vendor/**`, not `vendor/`.
- A failing formatter is an error (exit 2); mdt never falls back to unformatted output.
- With formatters, `mdt check` also reports **stale files**: formatting drift anywhere in a file that contains a consumer. `mdt update` rewrites those files.
- Verified commands: `dprint fmt --stdin "{{ filePath }}"`, `prettier --stdin-filepath "{{ filePath }}"`, `rustfmt --edition 2021` (never pass a path — `rustfmt --emit stdout <path>` reads the file on disk and prints a header), and `gofmt`.
- Keep `*.t.md` out of formatter scope here and in the formatter's own config (dprint `excludes`, `.prettierignore`).
- CI needs the same formatter binaries and versions.

## Scanning

- **Scanned:** markdown (`.md`, `.mdx`, `.markdown`) and source files `.rs .ts .tsx .mts .cts .js .jsx .mjs .cjs .py .go .java .kt .swift .c .cc .cpp .cxx .h .hh .hpp .cs .dart`, plus `[include]` and `[templates]` additions. Files without an HTML comment are skipped cheaply.
- **Skipped:** hidden files and directories (except `.templates/`), `node_modules/`, `target/`, and paths ignored by git.
- **Git ignore rules** follow git: nested `.gitignore` files, the `.gitignore` files of parent directories up to the repository root, and `.git/info/exclude` all apply. Outside a git repository only the project root's `.gitignore` applies.
- **Project root:** `--path <DIR>`, or else the nearest directory from the current one upward that contains `mdt.toml`, `.mdt.toml`, or `.config/mdt.toml` (the current directory when there is none; `mdt init` always uses the current directory).
- **Sub-projects:** a directory containing a config file is a separate project that the parent skips silently. Check each with `mdt check --path <dir>`. A sub-project that reads shared providers through `[templates] paths` must declare its own `[data]` namespaces (paths may start with `../`); providers shared this way are never reported as unused.
- **Symlinks:** each directory and file is scanned once, even through aliases or cycles; dangling links are skipped.
- **Tags in source-code strings** are live blocks. Exclude test fixtures and code that builds tags in strings with `[exclude] patterns`.

## Diagnostics

| Code                             | Severity | Meaning and fix                                                                                       |
| -------------------------------- | -------- | ----------------------------------------------------------------------------------------------------- |
| `mdt::unclosed_block`            | error    | Opening tag without a closing tag. Often a misspelled name — look for an unmatched closing tag nearby |
| `mdt::unmatched_closing_tag`     | error    | Closing tag with no open block of that name                                                           |
| `mdt::nested_block`              | error    | A block inside another block; move it out                                                             |
| `mdt::invalid_tag`               | error    | A markdown comment that looks like a tag but does not parse (`{ @name }`, `{=a.b}`)                   |
| `mdt::unknown_transformer`       | error    | Misspelled transformer                                                                                |
| `mdt::invalid_transformer_args`  | error    | Wrong number of transformer arguments                                                                 |
| `mdt::unused_provider`           | warning  | Provider without consumers                                                                            |
| `mdt::provider_outside_template` | warning  | Provider tag outside a `*.t.md` file (ignored)                                                        |
| `mdt::duplicate_provider`        | error    | Two providers share a name                                                                            |
| `mdt::config_parse`              | error    | Invalid `mdt.toml`, including unknown keys                                                            |
| `mdt::templates_path`            | error    | A `[templates] paths` entry is not a directory                                                        |
| `mdt::read_file`                 | error    | A scanned file could not be read as UTF-8                                                             |

Global flags: `--ignore-unclosed-blocks` (unclosed and unmatched tags), `--ignore-invalid-transformers`, `--ignore-invalid-names` (downgrade those errors), `--ignore-unused-blocks` (silence that warning). Warnings print by default; `--verbose` also shows ignored ones.

## CLI

| Command                                                             | Purpose                                                                                          |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `mdt init`                                                          | Add a starter `mdt.toml`, sample provider, synced sample readme (no README only), `.mdt/` ignore |
| `mdt check [--diff] [--format text\|json\|github] [--watch]`        | Verify every consumer is linked and current                                                      |
| `mdt update [--dry-run \| --watch]`                                 | Rewrite stale consumers                                                                          |
| `mdt list`                                                          | Every block with `file:line`, transformers, and `[linked]`/`[orphan]`/`[inline]`                 |
| `mdt info [--format json]`                                          | Project summary: blocks, data, templates, diagnostics, cache                                     |
| `mdt doctor [--format json]`                                        | Health checks with hints (exit 1 on any FAIL)                                                    |
| `mdt skill [--reference \| --install <DIR>]`                        | Print this skill, its reference, or write both to `<DIR>/mdt/`                                   |
| `mdt assist <generic\|claude\|cursor\|copilot\|pi> [--format json]` | MCP config and skill setup for an assistant                                                      |
| `mdt lsp` / `mdt mcp`                                               | Language server / MCP server over stdio                                                          |

Global: `-p, --path <DIR>` (must exist, except for `init`), `-v, --verbose`, `--no-color`, and the `--ignore-*` flags. Environment: `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`, `MDT_LOG=debug` (logs to stderr), `MDT_CACHE_VERIFY_HASH=1` (hash-verified cache).

### Exit codes

| Command      | 0                              | 1                                                              | 2                                             |
| ------------ | ------------------------------ | -------------------------------------------------------------- | --------------------------------------------- |
| `mdt check`  | Every consumer linked, current | Stale consumers or files, orphans, or render errors            | Validation, config, data, or formatter errors |
| `mdt update` | Done (warnings allowed)        | Some consumers skipped because their provider failed to render | Validation, config, data, or formatter errors |
| `mdt list`   | Listed                         | —                                                              | Listed, but validation errors were found      |
| `mdt doctor` | No FAIL checks                 | At least one FAIL                                              | —                                             |

### `mdt check --format json`

```json
{
	"ok": false,
	"stale": [
		{ "file": "readme.md", "block": "install", "line": 3, "column": 1 }
	],
	"stale_files": [{ "file": "docs/guide.md" }],
	"orphans": [
		{
			"file": "readme.md",
			"block": "featrues",
			"line": 9,
			"column": 1,
			"suggestions": ["features"]
		}
	],
	"errors": [
		{
			"file": "readme.md",
			"block": "badges",
			"line": 12,
			"column": 1,
			"message": "..."
		}
	],
	"diagnostics": [
		{
			"severity": "warning",
			"code": "mdt::unused_provider",
			"file": ".templates/a.t.md",
			"line": 7,
			"column": 1,
			"message": "..."
		}
	]
}
```

`--format github` prints `::error` annotations for every failure and `::warning` for warnings.

## MCP server

`mdt mcp` serves these tools over stdio. The server manages its working directory, or `mdt mcp --path <DIR>`; a tool's optional `path` argument must resolve inside it.

| Tool             | Parameters                                       | Returns (besides `ok`, `action`, `summary`)                                                        |
| ---------------- | ------------------------------------------------ | -------------------------------------------------------------------------------------------------- |
| `mdt_find_reuse` | `block_name?`, `content_query?`, `limit?` (1–20) | `candidates` ranked exact → same ignoring case/separators → prefix → substring → close spelling    |
| `mdt_list`       | `include_content?` (default false), `ignore_*?`  | `providers`, `consumers` (with `type`, `line`, `status`), `diagnostics`                            |
| `mdt_check`      | `ignore_*?`                                      | `stale`, `stale_files`, `orphans` (with `suggestions`), `render_errors`, `diagnostics`, `warnings` |
| `mdt_update`     | `dry_run?`, `ignore_*?`                          | `updated_count`, `updated_files`, `render_errors`, `diagnostics`; refuses to write on errors       |
| `mdt_preview`    | `block_name`                                     | The provider rendered for each consumer: `rendered_content`, `current_content`, `status`           |
| `mdt_get_block`  | `block_name`                                     | `provider` (or null) and `consumers`                                                               |
| `mdt_init`       | `path?`                                          | `config`, `sample`, `gitignore` outcomes, `written_files`, `next_steps` (same as `mdt init`)       |

- Every response is a JSON object with `ok`, `action`, and `summary`. Block `status` is `current`, `stale`, `render_error`, or `orphan`, exactly as `mdt check` sees it.
- `ignore_unclosed_blocks`, `ignore_unused_blocks`, `ignore_invalid_names`, and `ignore_invalid_transformers` mirror the CLI flags.
- Failures (bad config, missing data file, duplicate providers, a path outside the root) are tool results with `ok: false` and `error: { code, message, help? }`, where `code` is a diagnostic code such as `mdt::config_parse`.
- Setup for each client: `mdt assist <client>`.

## Files

| Path                                          | Role                                                           |
| --------------------------------------------- | -------------------------------------------------------------- |
| `*.t.md` (canonical: `.templates/`)           | Provider definitions                                           |
| `*.md`, `*.mdx`, `*.markdown`                 | Consumers and inline blocks                                    |
| Supported source files                        | Consumers and inline blocks inside comments                    |
| `mdt.toml` / `.mdt.toml` / `.config/mdt.toml` | Configuration                                                  |
| `.mdt/cache/`                                 | Local scan and data cache, written by every scan; gitignore it |
