# Security Policy

## Supported Versions

Only the latest minor release is actively supported with security fixes.

| Version | Supported |
| ------- | --------- |
| 0.4.x   | Yes       |
| < 0.4   | No        |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly through one of the following channels:

- **GitHub Security Advisories**: Open a [private security advisory](https://github.com/ifiokjr/mdt/security/advisories/new) on the repository. This is the preferred method.
- **Email**: Send a detailed report to [ifiokotung@gmail.com](mailto:ifiokotung@gmail.com).

Please include:

- A description of the vulnerability and its potential impact.
- Steps to reproduce the issue.
- Any relevant logs, configuration, or file contents (redacted as needed).

You can expect an initial response within 72 hours. Security fixes for confirmed vulnerabilities will be prioritized and released as patch versions.

## Scope

The following areas are in scope for security considerations:

- **File I/O** -- `mdt` reads and writes files on disk during scanning and updating. Path traversal or unintended file access is considered a security concern.
- **Template rendering** -- Provider content is rendered through `minijinja`. Template injection that could lead to unintended behavior is in scope.
- **Data parsing** -- `mdt` parses JSON, TOML, YAML, and KDL data sources. Malformed or adversarial input to these parsers is in scope.

## Security Posture

- `unsafe_code` is **denied** workspace-wide. The entire codebase is safe Rust.
- `clippy::correctness` lints are denied, not just warned.
- Dependencies are audited with `cargo-deny` for known vulnerabilities and license compliance.
