---
mdt_core: fix
---

# Parse YAML data files with serde-saphyr

`[data]` files with a `.yaml` or `.yml` extension are now parsed with [`serde-saphyr`](https://crates.io/crates/serde-saphyr) instead of `serde_yaml_ng`. `serde_yaml_ng` depends on `unsafe-libyaml`, which is unmaintained ([RUSTSEC-2024-0449](https://rustsec.org/advisories/RUSTSEC-2024-0449)) and failed the `cargo-deny` advisories check. The new parser is written in safe Rust and deserializes into the same `serde_json::Value`, so templates see the same data. No configuration change is needed.
