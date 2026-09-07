---
"mdt_cli": fix
"mdt_core": fix
---

# Render paths with forward slashes on every platform

CLI diagnostics, JSON reports, and doctor messages now normalize path separators so output is identical across operating systems, keeping the Unix-recorded snapshot corpus valid on Windows.
