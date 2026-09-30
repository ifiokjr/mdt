---
"mdt_core": feat
---

# Support hyphenated block names and Dart source files

Block names may now contain hyphens (`install-command`), matching kebab-case conventions; previously such tags failed tokenization and were silently ignored — they never appeared in `mdt list` or `mdt check`. `.dart` files are now scanned for consumer and inline blocks by default, like other supported source languages, so Dart doc comments can be synchronized from shared providers. Use `[include] patterns` to opt unlisted extensions in on older versions.
