---
mdt_core:
  bump: none
  type: refactor
mdt_cli:
  bump: none
  type: refactor
mdt_lsp:
  bump: none
  type: refactor
mdt_mcp:
  bump: none
  type: refactor
skills:
  bump: none
  type: refactor
---

# Migrate release tooling from knope to monochange

Replace `knope.toml` with `monochange.toml`, convert all existing changeset files to monochange format, and replace the knope-driven release/publish workflows with monochange-based CI workflows including cross-compiled binary uploads, Sigstore provenance attestation, crates.io OIDC publishing, and npm provenance publishing.