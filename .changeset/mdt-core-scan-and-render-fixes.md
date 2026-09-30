---
"mdt_core": fix
---

# Fix scanning boundaries, symlinks, backtick-safe code transformers, and padding edge cases

- A directory directly under the project root with its own `mdt.toml` is now treated as a separate project. Only directories two or more levels down were isolated, so `tools/mdt.toml` leaked its providers into the parent and caused duplicate-provider errors.
- Symlinked directories no longer abort the scan with a false "symlink cycle" error. Each directory is visited once by canonical path, so aliases (common in iOS `Pods/` and vendored trees) are scanned once, real cycles terminate, and the `[include]` walk no longer loops forever on a cycle. `MdtError::SymlinkCycle` is deprecated.
- `codeBlock` picks a fence longer than any backtick run in the content, and `code` picks a delimiter that does not occur in it, so a provider containing its own fence or inline code can no longer break the surrounding document.
- `replace` with an empty search string leaves content unchanged instead of inserting the replacement between every character.
- `[exclude] blocks` no longer makes `[[formatters]]` fail with "formatter pipeline changed the number of consumer blocks".
- A closing tag indented with whitespace only (in a list item or docstring) keeps its indentation.
- With `before` padding of 1 or more, the first blank-line comment prefix no longer lands on the opening tag's line when the content starts with a newline.
- `[include]` now honours `.gitignore` like the default scan does, and a file that cannot be read as UTF-8 text fails with `mdt::read_file` naming the file.
- `.mjs`, `.cjs`, `.mts`, `.cts`, `.cc`, `.cxx`, `.hh`, and `.hpp` files are scanned by default.
- `compute_updates` no longer aborts on the first provider that fails to render. `UpdateResult::render_errors` lists every consumer that was left untouched, with its location, and all other consumers are still updated. Render messages drop the internal template name and the duplicated "template rendering failed" prefix, and report the template line.
- The new `expected_consumer_content` function (and `ExpectedContent` enum) computes exactly what `mdt update` writes for one consumer. The CLI, MCP server, and language server share it.
- Repeated KDL nodes with the same name (`dep "a"`, `dep "b"`) collect into an array instead of the last one winning.
- The scan cache can no longer write to the wrong file. The project root is always made absolute and normalized (`resolve_root`), and the cache key includes it, so running `mdt update --path ..` from a subdirectory, or scanning a project that moved, rescans instead of reusing file paths recorded from another location. Previously this could splice provider content into an unrelated file.
- Edits that restore the modification time (`cp -p`, `rsync -t`, `touch -r`, archive extraction) are detected: file fingerprints include the inode change time on Unix.
- A script data source whose `watch` entries are not all existing files (a typo, a glob, or a directory) re-runs every time instead of serving cached output forever.
- Git ignore rules match git: nested `.gitignore` files, the ancestors' `.gitignore` files up to the repository root (so `mdt check --path packages/lib` skips the root's `dist/`), and `.git/info/exclude` all apply. Outside a git repository only the root `.gitignore` applies, as before.
- Symlinked files are scanned once (a symlinked `*.t.md` no longer causes a duplicate-provider error), and dangling symlinks are skipped instead of aborting the scan.
- `[templates] paths` adds only `*.t.md` files from each directory, uses the project's `[exclude]` and git ignore rules, and fails with `mdt::templates_path` when an entry is not a directory. A sub-project can point it at a shared template directory outside its root; files there are never treated as its consumers.
- `[exclude] blocks` also silences structural diagnostics (unclosed, unmatched, or nested tags) for the excluded names, and invalid `[include] patterns` globs are rejected when the config loads instead of being dropped.
- Files without any `<!--` are no longer parsed, which removes the dominant cost of scanning large tag-free markdown files.
- `TransformerType::NAMES` lists every transformer's canonical name, so help text can no longer drift from the implementation.
