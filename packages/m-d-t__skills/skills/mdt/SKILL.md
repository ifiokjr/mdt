---
name: mdt
description: Manage markdown templates with mdt. Synchronize README sections, docs-site content, and source-code comments (TypeScript, Rust, Dart, Python, Go, and more) from shared provider blocks. Use when editing documentation, creating provider/consumer blocks, running mdt commands, resolving formatter conflicts with synced docs, or working with mdt MCP tools.
---

# mdt — Markdown Template Management

## Install & version

```sh
npm install -g @m-d-t/cli   # provides the `mdt` binary
mdt --version               # check before relying on newer features
npm install -g @m-d-t/cli@latest   # upgrade
```

- If the CLI is already installed, use it as-is; only upgrade when a feature you need is missing.
- Recent additions: hyphenated block names (`my-block`), `.dart` source scanning, formatter-aware `[[formatters]]`. If `mdt list` does not discover such blocks, check `mdt --version` and upgrade with `npm install -g @m-d-t/cli@latest`.

## Quick start

```sh
mdt init     # starter .templates/template.t.md + annotated mdt.toml; leaves the project green
mdt check    # exit 0 = all consumers current; non-zero = stale (CI-friendly)
mdt update   # rewrite all stale consumers from provider content
mdt list     # show every provider/consumer and where it lives
mdt doctor   # run this FIRST when blocks are not behaving as expected
```

## Core workflow

1. **Define once** — create provider blocks in `*.t.md` files (canonical location: `.templates/`):
   ```markdown
   <!-- {@installCommand} -->

   npm install acme-http

   <!-- {/installCommand} -->
   ```

2. **Reuse everywhere** — add consumer blocks in markdown or inside code comments:
   ```markdown
   <!-- {=installCommand|trim|codeBlock:"sh"} -->
   <!-- {/installCommand} -->
   ```
   ```ts
   /**
    * <!-- {=apiDocs|trim|indent:" * ":true} -->
    * <!-- {/apiDocs} -->
    */
   export function createClient() {}
   ```

3. **Sync** — run `mdt update`. If the project formats its files (dprint, prettier, rustfmt, `dart format`), run the formatter too — see [Working with formatters](#working-with-formatters).

4. **Verify** — run `mdt check`; wire it into CI so stale docs fail the build.

## Working with formatters

`mdt check` compares consumer content **byte-for-byte** against the rendered provider. A formatter that rewraps a synced file makes `mdt check` report stale even though no words changed — the classic `mdt update → formatter → mdt check` loop. Never "fix" this by running `mdt update` in a loop; use one of the built-in escapes:

**Fix 1 (preferred): `[[formatters]]` in `mdt.toml`.** mdt runs your formatter itself, so `mdt update` writes and `mdt check` compares formatter-canonical output:

```toml
[[formatters]]
command = "dprint fmt --stdin \"{{ filePath }}\""
patterns = ["**/*.md"]
ignore = ["**/*.t.md"]

[[formatters]]
command = "prettier --stdin-filepath \"{{ filePath }}\""
patterns = ["**/*.ts", "**/*.tsx"]
```

- `{{ filePath }}`, `{{ relativeFilePath }}`, `{{ rootDirectory }}` are available in `command`.
- Keep `*.t.md` files out of formatter `patterns` (they are provider sources; formatting them as markdown can mangle `#`-commented config examples and `**` globs).
- After changing this config, run `mdt update` once, then the formatter, then `mdt check` — all three must be stable.

**Fix 2: `[check] comparison = "lenient"`.** Whitespace-normalized comparison for `mdt check` (blank lines, trailing whitespace, table/JSON formatting). `mdt update` still writes exact bytes. Use when formatter drift is whitespace-only.

**Fix 3 (fallback): formatter-stable providers.** Author provider content in the shape your formatter produces: run `mdt update`, run the formatter, copy the formatted block body back into the `*.t.md` provider, run `mdt update` again (it should now report "already up to date"). Re-do this whenever formatter config (e.g. line width) changes.

For consumers in code comments, set `[padding]` `before = 0, after = 0` so formatters have no stray blank lines to rewrite.

## Code files (TypeScript, Rust, Dart, ...)

Consumer and inline blocks work inside code comments. The tags are HTML comments; wrap them in the language's comment syntax and use transformers to re-apply the comment prefix:

```ts
/**
 * <!-- {=apiDocs|trim|indent:" * ":true} -->
 * <!-- {/apiDocs} -->
 */
```

```rust
/// <!-- {=apiDocs|trim|linePrefix:"/// ":true} -->
/// <!-- {/apiDocs} -->
pub fn create_client() {}
```

```dart
// <!-- {=pkgDescription|trim|linePrefix:"/// ":true} -->
// <!-- {/pkgDescription} -->
library;
```

- Set `[padding]` `before = 0, after = 0` in `mdt.toml` when consumers live in source files — without it, content merges onto the tag line.
- Tag lines keep the comment prefix you author them with; the transformer prefix applies to content lines only. Author the tag lines themselves with the doc-comment prefix (e.g. `///`) when they must be doc comments.
- Only listed extensions are scanned (`.rs`, `.ts`, `.tsx`, `.js`, `.jsx`, `.py`, `.go`, `.java`, `.kt`, `.swift`, `.c`, `.cpp`, `.h`, `.cs`, `.dart`, plus markdown). Other extensions are skipped **silently** — verify with `mdt list` that your consumer appears, or opt the extension in:

  ```toml
  [include]
  patterns = ["**/*.md", "**/*.dart"]
  ```

## Preventing stale docs

Sharing one provider beats copy-pasted docs: edit once, `mdt update`, and every copy is current. To keep it that way:

- Run `mdt check` in CI (it exits non-zero on stale consumers).
- Call `mdt_find_reuse` (MCP) or search `*.t.md` files **before** writing new docs that duplicate an existing provider.
- Prefer data interpolation over duplication: `{{ pkg.version }}` from `[data] pkg = "pubspec.yaml"` never goes stale.
- After editing any provider: `mdt check` then `mdt update`.

## Key rules

- Provider names are **globally unique** across all `*.t.md` files. Names may contain letters, digits, `_`, and `-`; camelCase is the convention.
- Providers live only in `*.t.md` files. Source files can contain **consumer** and **inline** blocks only.
- Tags inside properly closed markdown fences are inert examples. But an example that itself contains a `` ``` `` fence closes the outer fence early and leaks the tags after it into live content — use 4-backtick outer fences for such examples, or exclude the file: `[exclude] patterns = ["GUIDE.md"]`.
- Orphan consumers (no matching provider) only warn; `mdt doctor` catches them.
- Gitignore `.mdt/` — it is a local cache.
- Unknown `mdt.toml` keys are silently ignored; validate spelling against the annotated reference in REFERENCE.md when a config change "does nothing".

## MCP tools (via `mdt mcp`)

When using the MCP server, **always call `mdt_find_reuse` before creating a new provider**:

| Tool             | Purpose                                                     |
| ---------------- | ----------------------------------------------------------- |
| `mdt_find_reuse` | Find similar providers and reuse opportunities — call first |
| `mdt_list`       | List all providers and consumers                            |
| `mdt_check`      | Verify consumers are up-to-date                             |
| `mdt_update`     | Sync all consumers                                          |
| `mdt_preview`    | Preview rendered output before committing (MCP-only)        |
| `mdt_get_block`  | Get a specific block's content                              |
| `mdt_init`       | Initialize a new mdt project                                |

## Detailed reference

For the full transformer table, data interpolation, inline blocks, every configuration option (including `[[formatters]]` and `[check]`), and per-language source-file patterns, see [REFERENCE.md](REFERENCE.md).
