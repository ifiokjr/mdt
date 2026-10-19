---
"mdt_cli": patch
---

# GitHub annotations resolve paths against the repository root

`mdt check --format github` now writes annotation paths relative to the nearest enclosing git work tree instead of the process working directory. Annotations already landed correctly for `--path <sub-project>` runs from the checkout root; now they also land correctly when mdt runs from inside a sub-project — a CI step with `working-directory:` — where working-directory-relative paths used to point GitHub at the wrong file. Outside a git repository, paths stay relative to the working directory.

The CI integration guide no longer describes the old behavior as a limitation: it documents the repository-root rule and that scan-stopping errors print one file-less `::error` annotation.
