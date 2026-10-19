---
mdt_core: feat
mdt_cli: feat
---

Add `[format] tables` — mdt's internal GFM table alignment.

When enabled, `mdt update` aligns table columns inside block content using the
same rules as dprint's markdown plugin (column width = widest cell by display
width with CJK counting double, delimiter row fills the width, alignment
colons preserved), applied before transformers and padding. Provider blocks in
`*.t.md` files are rewritten to the aligned form too, so templates and
formatter-owned output files carry identical table bytes and neither side
needs to be excluded from a formatter. `mdt check` reports unaligned provider
tables as stale.

The normalizer is conservative: fenced code blocks, rows with more cells than
the header, pipes inside code spans, and tables glued to prose are left
untouched. Off by default; formatting never runs outside block boundaries.
