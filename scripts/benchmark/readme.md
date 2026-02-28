# Benchmark Harness

This folder contains the CLI benchmark harness used by CI.

## Scripts

- `generate_workload.sh`: creates a deterministic synthetic project with stale consumer blocks.
- `run_suite.sh`: runs benchmark scenarios for a single `mdt` binary and writes JSON output.
- `compare_results.sh`: compares baseline/candidate benchmark JSON and writes comparison JSON + markdown.

## Local usage

Build a release binary first:

```bash
cargo build --release --locked --manifest-path mdt_cli/Cargo.toml
```

Run the suite:

```bash
scripts/benchmark/run_suite.sh \
  --binary ./target/release/mdt \
  --output /tmp/mdt-benchmark-candidate.json \
  --label candidate
```

Compare two runs:

```bash
scripts/benchmark/compare_results.sh \
  --baseline /tmp/mdt-benchmark-baseline.json \
  --candidate /tmp/mdt-benchmark-candidate.json \
  --output /tmp/mdt-benchmark-compare.json \
  --markdown /tmp/mdt-benchmark-compare.md
```
