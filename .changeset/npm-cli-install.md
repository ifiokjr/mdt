---
mdt_cli: minor
"@m-d-t/cli-darwin-arm64": major
"@m-d-t/cli-darwin-x64": major
"@m-d-t/cli-linux-arm64-gnu": major
"@m-d-t/cli-linux-arm64-musl": major
"@m-d-t/cli-linux-x64-gnu": major
"@m-d-t/cli-linux-x64-musl": major
"@m-d-t/cli-win32-arm64-msvc": major
"@m-d-t/cli-win32-x64-msvc": major
---

# Publish the CLI through npm packages

`mdt` now has an official npm distribution channel. Releases prepare a top-level `@m-d-t/cli` package plus platform-specific binary packages for Linux, macOS, and Windows.

Users can install the CLI globally with npm or run it on demand through npx, making adoption easier in JavaScript-heavy projects and environments that do not already have Rust tooling installed.

```bash
npx @m-d-t/cli init
```
