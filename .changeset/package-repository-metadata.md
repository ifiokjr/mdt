---
mdt_cli: patch
mdt_core: patch
mdt_lsp: patch
"@m-d-t/cli": patch
"@m-d-t/cli-darwin-arm64": patch
"@m-d-t/cli-darwin-x64": patch
"@m-d-t/cli-linux-arm64-gnu": patch
"@m-d-t/cli-linux-arm64-musl": patch
"@m-d-t/cli-linux-x64-gnu": patch
"@m-d-t/cli-linux-x64-musl": patch
"@m-d-t/cli-win32-arm64-msvc": patch
"@m-d-t/cli-win32-x64-msvc": patch
"@m-d-t/skills": patch
---

# Add package repository metadata

Cargo and npm package manifests now include package-specific repository URLs. This keeps package metadata aligned with monochange manifest linting and points registry users directly to each package's source directory.
