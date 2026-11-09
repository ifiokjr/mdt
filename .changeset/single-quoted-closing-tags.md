---
mdt_core: fix
---

`mdt check` no longer fails with `mdt::unmatched_closing_tag` when a source file mentions a closing tag inside a single-quoted string on the same line, such as Python or JavaScript `'<!-- {/name} -->'`. Double quotes and backticks were already ignored. A real unmatched closing tag outside a quote is still reported.
