---
"mdt_lsp": fix
---

# Make editor diagnostics and quick fixes agree with `mdt check` and `mdt update`

- Stale-block diagnostics, the "Update block" quick fix, and the hover preview now compute expected content through the same engine as `mdt update`, including `[padding]` and the closing tag's comment prefix. Files that `mdt update` just wrote are no longer reported as out of date, and the quick fix no longer glues content onto the tags (for example `<!-- {=x} -->```sh` or `* <!-- {=x} --> * text`).
- `[check] comparison = "lenient"` and `[exclude] markdown_codeblocks` are honoured, so blocks that pass `mdt check` or that the scanner ignores are not diagnosed.
- A provider that fails to render is reported as an error at the consumer instead of falling back to the raw template, and no quick fix offers to write the raw `{{ … }}` text.
- Consumers without a provider are errors (they fail `mdt check`), still with did-you-mean suggestions. Providers outside `*.t.md` files are warnings.
- New diagnostics for closing tags without an opening tag (warning), comments that look like tags but do not parse (error), and blocks nested inside a consumer or inline block (error).
- `MDT_LOG=info mdt lsp` no longer panics at startup: the server keeps a tracing subscriber the CLI already installed.
- The quick fix computes its edit range from byte offsets in the open document, so lines with non-ASCII text before a tag are no longer corrupted, and it writes CRLF into CRLF documents.
- Files formatted by a `[[formatters]]` entry get no stale diagnostics or quick fixes, because `mdt check` compares formatter output the server does not compute on every keystroke; run `mdt check` or `mdt update` for them.
- Markdown headings and bullets before a closing tag are no longer treated as comment prefixes.
