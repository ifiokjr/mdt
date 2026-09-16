---
mdt: fix
---

# Decode escape sequences in transformer string arguments

Transformer string arguments kept their backslashes literal, so `indent:"\t"` injected a backslash and a `t` instead of a tab, and `suffix:"\n"` never appended a newline. The lexer stripped the surrounding quotes before handing the value to `snailquote`, which only expands escapes while it is inside a quoted region, so no escape was ever decoded. Arguments are now decoded from the full quoted slice, which makes `\t`, `\n`, `\\` and `\"` work inside double-quoted arguments; single-quoted arguments and unrecognised escapes keep their backslashes so existing tags are unaffected.
