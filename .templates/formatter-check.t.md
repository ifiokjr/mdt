<!-- {@mdtFormatterPipelineDocs} -->

`[[formatters]]` entries run your formatter inside `mdt update` and `mdt check`. `mdt update` writes formatted files, and `mdt check` compares against formatted output, so the `mdt update` → formatter → `mdt check` loop settles instead of reporting stale blocks after every format.

```toml
[[formatters]]
command = "dprint fmt --stdin \"{{ "{{" }} filePath {{ "}}" }}\""
patterns = ["**/*.md"]
ignore = ["**/*.t.md"]
```

Each matching entry:

- reads the whole file on stdin and writes the formatted file to stdout
- runs from the project root through `sh -c` (`cmd /C` on Windows)
- runs after block injection in `mdt update`, and before comparison in `mdt check`
- runs in declaration order when several entries match the same file

`command` can use three placeholders. mdt passes their values as the environment variables `MDT_FILE_PATH`, `MDT_RELATIVE_FILE_PATH`, and `MDT_ROOT_DIRECTORY`, so the shell never parses a file name. Keep placeholders inside double quotes; inside single quotes they stay literal.

- `{{ "{{" }} filePath {{ "}}" }}`: absolute path of the file being formatted
- `{{ "{{" }} relativeFilePath {{ "}}" }}`: path relative to the project root
- `{{ "{{" }} rootDirectory {{ "}}" }}`: absolute project root

`patterns` and `ignore` are ordered lists of plain globs, not gitignore patterns. A `!` entry negates an earlier match. `vendor/` matches nothing inside the directory; write `vendor/**`.

A formatter that fails or exits non-zero is an error (exit status 2); mdt never falls back to unformatted output. Without `[[formatters]]`, mdt runs no formatter.

Keep `*.t.md` files out of formatter scope, both in `ignore` and in the formatter's own config (dprint `excludes`, `.prettierignore`): markdown formatters rewrite provider text such as `#` lines and `**` globs. CI must install the same formatter versions you use locally.

<!-- {/mdtFormatterPipelineDocs} -->

<!-- {@mdtFormatterOnlyStaleDocs} -->

With formatters configured, `mdt check` can also report **stale files**: files that contain a consumer and that the formatter would change, even though every consumer block is current. `mdt update` rewrites the whole file, so the drift can be anywhere in it, not only inside a block. Run `mdt update` to normalize them. JSON output and MCP responses list these files in `stale_files`, separate from stale consumers in `stale`.

<!-- {/mdtFormatterOnlyStaleDocs} -->

<!-- {@mdtCheckJsonOutput} -->

`mdt check --format json` prints one object with every key present:

| Key           | Entries                                                                   |
| ------------- | ------------------------------------------------------------------------- |
| `ok`          | `true` when every consumer is linked and current                          |
| `stale`       | Stale consumers: `file`, `block`, `line`, `column`                        |
| `stale_files` | Files a formatter would change: `file`                                    |
| `orphans`     | Consumers with no provider: `file`, `block`, `line`, `column`, `suggestions` |
| `errors`      | Render errors: `file`, `block`, `line`, `column`, `message`               |
| `diagnostics` | Errors and warnings: `severity`, `code`, `file`, `line`, `column`, `message` |

A clean project:

```json
{ "diagnostics": [], "errors": [], "ok": true, "orphans": [], "stale": [], "stale_files": [] }
```

A stale consumer, an orphan, and an unused-provider warning:

```json
{
	"diagnostics": [
		{
			"code": "mdt::unused_provider",
			"column": 1,
			"file": ".templates/template.t.md",
			"line": 11,
			"message": "provider block `unused` has no consumers",
			"severity": "warning"
		}
	],
	"errors": [],
	"ok": false,
	"orphans": [
		{ "block": "intor", "column": 1, "file": "readme.md", "line": 7, "suggestions": ["intro"] }
	],
	"stale": [{ "block": "intro", "column": 1, "file": "readme.md", "line": 3 }],
	"stale_files": []
}
```

When validation errors (such as an unclosed or nested block) stop the check, the same object is printed with `ok: false` and the errors in `diagnostics`, and the exit status is 2. Errors that stop the scan itself (config and data errors, duplicate providers, unreadable files) print only the error report on stderr, also with exit status 2.

<!-- {/mdtCheckJsonOutput} -->
