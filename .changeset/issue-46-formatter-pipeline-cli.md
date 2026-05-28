---
mdt_cli: minor
---

Support `[[formatters]]` config for `mdt update` and `mdt check`.

When configured, `mdt` now runs matching formatter commands on the entire updated target file, making `mdt update` and `mdt check` converge with project formatters like dprint and Prettier.
