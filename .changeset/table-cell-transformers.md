---
"mdt_core": minor
"mdt_cli": minor
"@m-d-t/skills": docs
---

# Parse inline-block transformers inside GFM table cells

GFM splits table rows on `|` before inline content is parsed, so an inline tag with a transformer chain (`<!-- {~name:"{{ pkg.version }}"|code} -->`) was cut into text fragments that never reached mdt, and its closing tag became a `mdt::unmatched_closing_tag` error. mdt now scans each table row's raw text before that split:

- Inline blocks inside table cells keep their full transformer chain; the rendered value is written back with only that value changing, preserving the rest of the row byte-for-byte.
- Pipes inside a quoted template argument (`{~name:"a|b"}`) also stay part of the tag in table cells.
- A second `mdt check` after `mdt update` is clean, and `mdt update` is idempotent.
- Escaped pipes (`\|`) and other cell text keep behaving exactly as GFM defines; behavior outside tables is unchanged.
- One caveat: a markdown formatter that rewrites tables and does not know mdt tags (such as dprint formatting fenced `markdown` blocks) can still split such a row on the pipe — keep examples in `text` fences.
