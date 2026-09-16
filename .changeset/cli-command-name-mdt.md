---
"mdt_cli": fix
---

# Show the mdt command name in version and help output

`mdt --version` printed the crate name (`mdt_cli 0.9.4`) instead of the command users invoke. The clap command is now explicitly named `mdt`, so `--version`, `--help`, and usage errors all display `mdt`. The published crate and install command (`cargo install mdt_cli`) are unchanged.
