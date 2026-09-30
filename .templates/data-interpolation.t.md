<!-- {@mdtScriptDataSourcesGuide} -->

A `[data]` entry can run a shell command and parse its stdout. Use it for values that come from tooling, such as git metadata, Nix, or a generated version file.

```toml
[data]
release = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

- `command`: shell command, run from the project root.
- `format`: parser for stdout: `text` (also `string`, `raw`, `txt`), `json`, `toml`, `yaml`, `yml`, `kdl`, or `ini`. Text drops one trailing newline, so `cat VERSION` renders `1.2.3`, not `1.2.3` plus a line break.
- `watch`: files whose changes invalidate the cached output.

<!-- {/mdtScriptDataSourcesGuide} -->

<!-- {@mdtScriptDataSourcesNotes} -->

- Output is cached in `.mdt/cache/data-v1.json`, keyed by namespace, command, format, and watch list. mdt reuses it while the watched files are unchanged.
- Caching needs every `watch` entry to be an existing file. Without `watch`, or when an entry is missing, a glob, or a directory, the command runs on every mdt run.
- A command that exits non-zero fails the run with `mdt::data_script` (exit status 2).

<!-- {/mdtScriptDataSourcesNotes} -->
