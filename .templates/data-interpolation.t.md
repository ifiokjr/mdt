<!-- {@mdtScriptDataSourcesGuide} -->

`[data]` entries can run shell commands and use stdout as template data. This is useful for values that come from tooling (for example Nix, git metadata, or generated version files).

```toml
[data]
release = { command = "cat VERSION", format = "text", watch = ["VERSION"] }
```

- `command`: shell command executed from the project root.
- `format`: parser for stdout (`text`, `json`, `toml`, `yaml`, `yml`, `kdl`, `ini`).
- `watch`: files that control cache invalidation.

When `watch` files are unchanged, mdt reuses cached script output from `.mdt/cache/data-v1.json` instead of re-running the command.

<!-- {/mdtScriptDataSourcesGuide} -->

<!-- {@mdtScriptDataSourcesNotes} -->

- Script outputs are cached per namespace, command, format, and watch list.
- If `watch` is empty, mdt re-runs the script every load (no cache hit).
- A non-zero script exit status fails data loading with an explicit error.

<!-- {/mdtScriptDataSourcesNotes} -->
