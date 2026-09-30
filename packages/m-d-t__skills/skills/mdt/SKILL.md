---
name: mdt
description: Manage markdown templates with mdt. Synchronize README sections, docs-site content, and source-code doc comments (TypeScript, Rust, Dart, Python, Go, and more) from shared provider blocks. Use when editing documentation that contains mdt tags (`<!-- {=name} -->`), creating provider/consumer blocks, running mdt commands, fixing `mdt check` failures in CI, resolving formatter conflicts with synced docs, or working with mdt MCP tools.
---

# mdt — Markdown Template Management

mdt keeps repeated documentation in sync. Content is defined once in a **provider** block inside a `*.t.md` file and copied into every matching **consumer** block — in markdown files and in source-code comments. `mdt update` rewrites consumers; `mdt check` fails CI when any are stale.

## Install & version

```sh
npm install -g @m-d-t/cli   # provides the `mdt` binary (or: cargo install mdt_cli)
mdt --version
mdt skill                   # this skill, matching the installed binary
mdt skill --reference       # the full reference (REFERENCE.md)
```

If an installed copy of this skill disagrees with the CLI, trust `mdt skill` — it is embedded in the binary. Upgrade with `npm install -g @m-d-t/cli@latest`.

## Workflow

1. **Look before adding.** Run `mdt list` (or MCP `mdt_find_reuse`) and search `*.t.md` files; reuse an existing provider instead of creating a near-duplicate.
2. **Edit the provider, never a synced copy.** Consumer content is overwritten by `mdt update`.
3. **Sync:** `mdt update`, then `mdt check`. Both must pass — `mdt check` exits `0` only when every consumer is linked and current.
4. **Never loop `mdt update` against a formatter** — configure `[[formatters]]` instead (see below).

New project: `mdt init` adds an annotated `mdt.toml`, a sample `greeting` provider in `.templates/template.t.md`, a synced sample `readme.md` (only when there is no README), and `.mdt/` to `.gitignore`. It never overwrites existing files. Replace the sample with real providers.

## Blocks

```markdown
<!-- .templates/install.t.md — providers live ONLY in *.t.md files -->
<!-- {@installCommand} -->

npm install acme-http

<!-- {/installCommand} -->
```

```markdown
<!-- README.md — a consumer; mdt writes the content between the tags -->
<!-- {=installCommand|trim|codeBlock:"sh"} -->
<!-- {/installCommand} -->
```

After `mdt update` the consumer contains the fenced command. Rules:

- Names match `[A-Za-z_][A-Za-z0-9_-]*` and are case-sensitive. The sigil must follow `{` directly: `{@name}` works, `{ @name }` is not a tag (reported as an invalid tag in markdown; in code comments its leftover closing tag is reported as unmatched).
- Provider names are unique across the project. A `{@name}` outside a `*.t.md` file is ignored (with a warning).
- Blocks never nest (`mdt::nested_block`): inside a consumer, `mdt update` would overwrite the inner block; inside a provider, its tags would be copied into every consumer. Use `{{ ... }}` data in providers instead of inline blocks.
- Inline blocks render a data value in place: `Version <!-- {~v:"{{ pkg.version }}"} -->0.0.0<!-- {/v} -->`. Use them for values inside sentences and table cells (in a table cell, leave out `|` transformers — the table syntax splits on `|`).
- Tags inside fenced code blocks and inline code in markdown are inert examples. An example that contains its own `` ``` `` fence needs a 4-backtick outer fence, or the inner fence closes the outer one early.

## Transformers

Pipe filters on the consumer tag, applied left to right: `trim`, `trimStart`, `trimEnd`, `indent`, `prefix`, `suffix`, `linePrefix`, `lineSuffix`, `wrap`, `codeBlock`, `code`, `replace`, `if`. Arguments are quoted strings: `linePrefix:"/// ":true`, `replace:"old":"new"`, `codeBlock:"ts"`, `if:"pkg.private"` (a dotted data path, not an expression). `indent:4` prepends the text `4` — write `indent:"    "`.

## Source-file consumers

Tags go inside the language's comments; a transformer re-applies the comment prefix to the content. Always pass `true` so blank lines get the prefix too:

```rust
//! <!-- {=crateDocs|trim|linePrefix:"//! ":true} -->
//! <!-- {/crateDocs} -->
```

```ts
/**
 * <!-- {=clientDocs|trim|linePrefix:" * ":true} -->
 * <!-- {/clientDocs} -->
 */
export function createClient() {}
```

| Comment                            | Transformer                                                         |
| ---------------------------------- | ------------------------------------------------------------------- |
| Rust crate docs (`//!`)            | `linePrefix:"//! ":true`                                            |
| Rust item docs (`///`)             | `linePrefix:"/// ":true`                                            |
| Rust item docs inside an `impl`    | `linePrefix:"    /// ":true` (indentation + prefix)                 |
| TypeScript/JavaScript JSDoc        | `linePrefix:" * ":true`                                             |
| JSDoc on a class method            | `linePrefix:"   * ":true` (the method's indentation, then the star) |
| Go, Java, Kotlin, Swift, C# (`//`) | `linePrefix:"// ":true`                                             |
| Python (`#`)                       | `linePrefix:"# ":true`                                              |
| Dart (`///`)                       | `linePrefix:"/// ":true`                                            |

- Prefer `linePrefix` over `indent` for comments: `linePrefix` trims trailing spaces on blank lines, so formatters leave the output alone.
- Scanned: `.rs .ts .tsx .mts .cts .js .jsx .mjs .cjs .py .go .java .kt .swift .c .cc .cpp .cxx .h .hh .hpp .cs .dart` plus `.md .mdx .markdown`. Other extensions are skipped silently — opt in with `[include] patterns = ["**/*.rb"]` (this **adds** files; it never narrows the scan).
- Complete tags inside string literals are live blocks too; exclude such files (`[exclude] patterns`). A lone closing tag inside quotes or backticks (`"<!-- {/x} -->"`) is ignored.
- Provider text containing `*/` (a `**/*.ts` glob) closes a `/* */` comment. Escape it without writing `*/` in the tag itself: `replace:"*\u{2f}":"*\\/"`.

## Data

```toml
# mdt.toml
[data]
pkg = "package.json" # {{ pkg.version }}
cargo = "crates/core/Cargo.toml" # {{ cargo.package.version }}
release = { command = "git describe --tags", format = "text" }
```

Providers render with minijinja once `[data]` exists: `{{ pkg.version }}`, `{% if %}`, `{% for %}`, filters like `upper`/`replace`/`join`. Literal `{{ }}` in providers (GitHub Actions `${{ secrets.X }}`, Handlebars) must be wrapped in `{% raw %}...{% endraw %}`. Text sources drop one trailing newline. Prefer data over hardcoded versions — they never go stale.

## Formatters

`mdt check` compares bytes. A formatter that rewraps a synced file makes it stale again. Fix it once in `mdt.toml` so mdt runs the formatter itself:

```toml
[[formatters]]
command = "dprint fmt --stdin \"{{ filePath }}\"" # keep placeholders in double quotes
patterns = ["**/*.md"]
ignore = ["**/*.t.md"]

[[formatters]]
command = "prettier --stdin-filepath \"{{ filePath }}\""
patterns = ["**/*.ts", "**/*.tsx"]

[[formatters]]
command = "rustfmt --edition 2021" # stdin to stdout; never pass the file path to rustfmt
patterns = ["**/*.rs"]
```

- Also exclude `*.t.md` in the formatter's own config (dprint `excludes`, `.prettierignore`): markdown formatters rewrite `#` lines and `**` globs inside provider text.
- With formatters, `mdt check` also reports **stale files** (formatting drift anywhere in a file with a consumer); `mdt update` fixes them.
- CI must install the same formatter versions, or `mdt check` fails.
- Whitespace-only drift can instead use `[check] comparison = "lenient"` (trailing spaces and blank-line runs only — not indentation or tables).

## CI

```yaml
- run: npx -y @m-d-t/cli@<version> check --format github
```

Exit codes: `0` in sync; `1` stale, orphan (consumer with no provider), or render error; `2` validation or config error (unclosed, unmatched, or nested tags, invalid tags, unknown transformers, duplicate providers, bad `mdt.toml`).

**Monorepos.** Every directory with its own `mdt.toml` is a separate project that the parent skips silently — run `mdt check --path <dir>` for each in CI. Without `--path`, mdt uses the nearest ancestor directory with an `mdt.toml` inside the git repository, so it works from any subdirectory. To reuse the root's providers in a sub-project, add `[templates] paths = ["../../.templates"]` and redeclare every `[data]` namespace they use (paths may start with `../`); without `[data]`, `{{ pkg.version }}` is copied literally (mdt warns).

## When `mdt check` fails

| Output                                                | Fix                                                                                    |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `consumer block(s) are out of date`                   | `mdt update` (edit the provider if the synced text is wrong)                           |
| `has no provider (did you mean ...?)`                 | Fix the name, or move the provider into a `*.t.md` file                                |
| `missing closing tag` + `has no matching opening tag` | One tag is misspelled — make the names match; don't just add the suggested closing tag |
| `looks like an mdt tag but cannot be parsed`          | Remove the space after `{` or fix the name                                             |
| `is inside consumer`                                  | Move the inner block out                                                               |
| `unknown field` in `mdt.toml`                         | Typo in a config key — unknown keys are rejected                                       |
| Render error                                          | Fix the provider template; other consumers still update                                |

`mdt list` shows every block with `file:line` and `[linked]`/`[orphan]`/`[inline]`, even when there are errors. `mdt doctor` gives health checks with hints. `mdt check --format json` returns `ok`, `stale`, `stale_files`, `orphans`, `errors`, and `diagnostics` with locations.

## Config at a glance

`mdt.toml` (or `.mdt.toml`, `.config/mdt.toml`); unknown keys are errors. Default padding puts content on the line after the opening tag and the closing tag on its own line; `[padding] before/after` adds blank lines (`false` = inline). `[exclude] patterns` (gitignore syntax; negate with `dir/*` + `!dir/keep.md`), `[exclude] blocks` (removes those names entirely — their consumers are never filled or checked), `[include] patterns` (adds files), `[templates] paths` (adds `*.t.md` directories, e.g. a shared `../../.templates`), `[check]`, `[[formatters]]`, `max_file_size`, `disable_gitignore`. `.gitignore` rules apply like git. Gitignore the `.mdt/` cache.

## Agents

- `mdt skill --install .claude/skills` (or `.agents/skills`, `.pi/skills`, `.github/skills`) installs this skill for a project; `mdt assist <claude|cursor|copilot|pi|generic>` prints MCP setup for each client.
- MCP server (`mdt mcp`): `mdt_find_reuse` (call before creating a provider), `mdt_list`, `mdt_check`, `mdt_update` (`dry_run`), `mdt_preview`, `mdt_get_block`, `mdt_init`. Responses are JSON with `ok`, `action`, and `summary`.

For the full reference — every config key, padding rules, transformer details, diagnostics, and JSON output — run `mdt skill --reference` or read [REFERENCE.md](REFERENCE.md).
