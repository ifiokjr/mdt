---
"mdt_mcp": security
---

# Stop `mdt_init` from writing outside the server root through a symlink

A tool `path` that did not exist yet could not be canonicalized, so confinement fell back to the lexical path: with `esc` symlinked to a directory outside the server root, `mdt_init` with `path: "esc/sub"` passed the containment check and wrote there. Paths are now resolved through their deepest existing ancestor before the check, and a dangling symlink along the path is rejected (`mdt::path_unresolvable`) because creating directories through it would follow it wherever it points.
