<!-- {@mdtFormatterPipelineDocs} -->

Formatter entries make `mdt update` and `mdt check` converge with your formatter's canonical **full-file** output instead of comparing raw injected block text.

This is the recommended long-term fix for the `mdt update → formatter → mdt check` cycle described in issue #46, and the best way to keep CI green when external formatters rewrite synced files.

Each matching formatter entry:

- reads the full candidate file from stdin
- writes the full replacement file to stdout
- runs from the project root
- runs after block injection during `mdt update`
- runs before expected-output comparison during `mdt check`
- runs in declaration order when multiple entries match the same file

`command` is rendered with minijinja before execution. Available variables:

- `{{ "{{" }} filePath {{ "}}" }}` — absolute path to the file being formatted
- `{{ "{{" }} relativeFilePath {{ "}}" }}` — path relative to the project root
- `{{ "{{" }} rootDirectory {{ "}}" }}` — absolute project root

`patterns` and `ignore` are ordered gitignore-style rule lists. Leading `!` entries negate a prior match, so later rules can re-include paths for a single formatter stage.

If a formatter command fails, exits non-zero, or renders an invalid minijinja command template, mdt returns an explicit formatter error instead of silently falling back to unformatted output.

```toml
[[formatters]]
command = "dprint fmt --stdin \"{{ "{{" }} filePath {{ "}}" }}\""
patterns = ["**/*.md", "!docs/generated/**"]
ignore = ["vendor/**", "docs/generated/**", "!docs/generated/keep.md"]
```

Repositories without configured formatters keep the legacy fast path, so formatter support only adds work when you opt in.

<!-- {/mdtFormatterPipelineDocs} -->

<!-- {@mdtFormatterOnlyStaleDocs} -->

Formatter-aware checking can also report **formatter-only** drift. This happens when the formatter would rewrite the full file, but no individual managed block body is stale.

In that case mdt reports the file in `stale_files` so automation can distinguish surrounding-formatting drift from block-content drift. The CLI JSON output and MCP responses include `stale_files` for this reason.

<!-- {/mdtFormatterOnlyStaleDocs} -->

<!-- {@mdtCheckJsonOutput} -->

`mdt check --format json` returns:

- `ok` — overall success boolean
- `stale` — block-level drift entries with `file` and `block`
- `stale_files` — formatter-only file drift entries with `file`

When formatter-aware normalization would change the full file without changing any managed block body, `stale_files` is populated and `stale` can remain empty.

Clean output:

```json
{ "ok": true, "stale": [], "stale_files": [] }
```

Formatter-only drift example:

```json
{
	"ok": false,
	"stale": [],
	"stale_files": [{ "file": "docs/readme.md" }]
}
```

<!-- {/mdtCheckJsonOutput} -->
