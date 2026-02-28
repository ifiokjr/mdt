# Benchmarking and Regressions

This project tracks CLI performance continuously in CI.

## What CI benchmarks

The benchmark workflow compares two revisions in the **same CI job** and **same Docker container**:

- `baseline`: merge-base with `main` (for pull requests) or previous commit (for pushes to `main`)
- `candidate`: the commit under test

Each revision is built in `--release` mode, then benchmarked against the same deterministic workload.

Scenarios currently include:

- `check_cold_clean`
- `check_warm_clean`
- `check_cold_stale`
- `check_diff_stale`
- `update_stale`
- `update_noop_clean`
- `list_clean`
- `info_clean`

## Consistency strategy

Absolute runtimes naturally drift as GitHub runners evolve. To keep comparisons stable, we enforce:

- Same machine for both baseline and candidate (single job)
- Same container image (`rust:1.86.0-bookworm`)
- Same generated workload and iteration counts
- Median-based comparisons with combined thresholds

A regression is only flagged when **both** are true:

- Relative slowdown exceeds `BENCH_RELATIVE_THRESHOLD_PCT`
- Absolute slowdown exceeds `BENCH_ABSOLUTE_THRESHOLD_MS`

This avoids failing on tiny/noisy deltas.

## Regression policy

If benchmark regressions are detected on a pull request, the PR must include a `## Benchmark Justification` section in the PR description explaining why the tradeoff is acceptable.

Without this section, the benchmark workflow fails.

## Historical records

Each benchmark run uploads artifacts with:

- Baseline results (`baseline.json`)
- Candidate results (`candidate.json`)
- Comparison report (`compare.json`, `compare.md`)

CI also posts an updated benchmark report comment on pull requests.
