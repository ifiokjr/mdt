---
"mdt_core": security
---

# Stop `[[formatters]]` from passing file names through the shell

`{{ filePath }}`, `{{ relativeFilePath }}`, and `{{ rootDirectory }}` in a formatter `command` used to be pasted into the `sh -c` (or `cmd /C`) command line as raw text. A file named ``docs/a$(touch pwned).md`` ran `touch pwned` during `mdt check` or `mdt update`, so a hostile file name in a pull request could execute code in CI.

The placeholders now render as references to environment variables that mdt sets for each formatter run (`MDT_FILE_PATH`, `MDT_RELATIVE_FILE_PATH`, `MDT_ROOT_DIRECTORY`), so the shell expands the path as data and never parses it. The documented form keeps working unchanged:

```toml
[[formatters]]
command = "dprint fmt --stdin \"{{ filePath }}\""
patterns = ["**/*.md"]
```

Keep placeholders in double quotes. A placeholder inside single quotes (`'{{ filePath }}'`) now reaches the formatter as the literal text `${MDT_FILE_PATH}`; switch those to double quotes.
