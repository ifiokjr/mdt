---
"mdt_cli": docs
---

# Homebrew install channel

`brew install ifiokjr/mdt/mdt` installs the prebuilt `mdt` binary on macOS and Linux — a fourth install channel next to npm, the GitHub release archives, and cargo, for users without Node.js or a Rust toolchain.

A new `homebrew-tap` workflow regenerates `Formula/mdt.rb` (macOS arm/intel, Linux arm/intel glibc, SHA256-pinned release archives) and pushes it to the `ifiokjr/homebrew-mdt` tap whenever a release is published. One-time owner setup is documented in the workflow: create the tap repository and add a fine-grained `HOMEBREW_TAP_TOKEN` secret with Contents access to it.
