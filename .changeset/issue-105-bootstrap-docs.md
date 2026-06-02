---
mdt_cli: none
---

# Align bootstrap docs with the templates directory

Bootstrap documentation now describes `.templates/` as the canonical starter layout. The generated examples and docs consistently point users toward `.templates/template.t.md` instead of older root-level template locations.

This documentation-only change keeps onboarding guidance aligned with the current `mdt init` behavior and reduces confusion for users setting up their first provider and consumer blocks.
