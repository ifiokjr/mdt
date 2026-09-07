---
"mdt_cli": fix
"mdt_core": fix
"mdt_lsp": fix
---

# Render paths with forward slashes on every platform

CLI diagnostics, JSON reports, and doctor messages now normalize path separators so output is identical across operating systems, keeping the Unix-recorded snapshot corpus valid on Windows. The LSP server also resolves file URIs for stored and synthetic paths consistently across platforms when computing goto definition, references, and duplicate provider diagnostics.
