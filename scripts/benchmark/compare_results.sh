#!/usr/bin/env bash
set -euo pipefail

usage() {
	cat <<'USAGE'
Compare two benchmark result JSON files.

Usage:
  scripts/benchmark/compare_results.sh \
    --baseline <path> \
    --candidate <path> \
    --output <path> \
    [--markdown <path>] \
    [--relative-threshold-pct <value>] \
    [--absolute-threshold-ms <value>] \
    [--fail-on-regression]

Options:
  --baseline <path>                 Baseline benchmark JSON (required).
  --candidate <path>                Candidate benchmark JSON (required).
  --output <path>                   Comparison JSON output (required).
  --markdown <path>                 Optional markdown report path.
  --relative-threshold-pct <value>  Percentage threshold to classify regression (default: 8).
  --absolute-threshold-ms <value>   Absolute ms threshold to classify regression (default: 5).
  --fail-on-regression              Exit non-zero when regressions are detected.
USAGE
}

baseline=""
candidate=""
output=""
markdown=""
relative_threshold_pct="8"
absolute_threshold_ms="5"
fail_on_regression=false

while [[ $# -gt 0 ]]; do
	case "$1" in
	--baseline)
		baseline="${2:-}"
		shift 2
		;;
	--candidate)
		candidate="${2:-}"
		shift 2
		;;
	--output)
		output="${2:-}"
		shift 2
		;;
	--markdown)
		markdown="${2:-}"
		shift 2
		;;
	--relative-threshold-pct)
		relative_threshold_pct="${2:-}"
		shift 2
		;;
	--absolute-threshold-ms)
		absolute_threshold_ms="${2:-}"
		shift 2
		;;
	--fail-on-regression)
		fail_on_regression=true
		shift
		;;
	-h | --help)
		usage
		exit 0
		;;
	*)
		echo "unknown option: $1" >&2
		usage >&2
		exit 1
		;;
	esac
done

if [[ -z "$baseline" || -z "$candidate" || -z "$output" ]]; then
	echo "--baseline, --candidate, and --output are required" >&2
	exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
	echo "jq is required for compare_results.sh" >&2
	exit 1
fi

mkdir -p "$(dirname "$output")"

jq -n \
	--slurpfile baseline "$baseline" \
	--slurpfile candidate "$candidate" \
	--argjson relative_threshold_pct "$relative_threshold_pct" \
	--argjson absolute_threshold_ms "$absolute_threshold_ms" '
	def to_map($arr):
		reduce $arr[] as $item ({}; .[$item.name] = $item);

	($baseline[0]) as $baseline_doc
	| ($candidate[0]) as $candidate_doc
	| ($baseline_doc.scenarios // []) as $baseline_scenarios
	| ($candidate_doc.scenarios // []) as $candidate_scenarios
	| (to_map($baseline_scenarios)) as $baseline_map
	| (to_map($candidate_scenarios)) as $candidate_map
	| (((($baseline_map | keys) + ($candidate_map | keys)) | unique | sort)) as $scenario_names
	| {
		schema_version: 1,
		generated_at_utc: (now | strftime("%Y-%m-%dT%H:%M:%SZ")),
		baseline: {
			label: ($baseline_doc.label // "baseline"),
			generated_at_utc: $baseline_doc.generated_at_utc,
			binary: $baseline_doc.binary
		},
		candidate: {
			label: ($candidate_doc.label // "candidate"),
			generated_at_utc: $candidate_doc.generated_at_utc,
			binary: $candidate_doc.binary
		},
		relative_threshold_pct: $relative_threshold_pct,
		absolute_threshold_ms: $absolute_threshold_ms,
		scenarios: [
			$scenario_names[] as $name
			| ($baseline_map[$name]) as $base
			| ($candidate_map[$name]) as $cand
			| if ($base == null or $cand == null) then
				{
					name: $name,
					status: "missing"
				}
			else
				($base.stats.median_ms) as $base_ms
				| ($cand.stats.median_ms) as $cand_ms
				| ($cand_ms - $base_ms) as $delta_ms
				| (if $base_ms == 0 then 0 else ($delta_ms / $base_ms * 100) end) as $delta_pct
				| {
					name: $name,
					baseline_ms: $base_ms,
					candidate_ms: $cand_ms,
					delta_ms: $delta_ms,
					delta_pct: $delta_pct,
					status: (
						if ($delta_ms > $absolute_threshold_ms and $delta_pct > $relative_threshold_pct) then "regression"
						elif ($delta_ms < (-$absolute_threshold_ms) and $delta_pct < (-$relative_threshold_pct)) then "improvement"
						else "neutral"
						end
					)
				}
			end
		]
	}
	| .missing_count = ([.scenarios[] | select(.status == "missing")] | length)
	| .regression_count = ([.scenarios[] | select(.status == "regression")] | length)
	| .improvement_count = ([.scenarios[] | select(.status == "improvement")] | length)
	| .status = (
		if .missing_count > 0 then "invalid"
		elif .regression_count > 0 then "regression"
		else "ok"
		end
	)
' >"$output"

if [[ -n "$markdown" ]]; then
	mkdir -p "$(dirname "$markdown")"
	{
		echo "## mdt Benchmark Comparison"
		echo
		echo "- Baseline: \`$(jq -r '.baseline.label' "$output")\`"
		echo "- Candidate: \`$(jq -r '.candidate.label' "$output")\`"
		echo "- Thresholds: > ${relative_threshold_pct}% and > ${absolute_threshold_ms}ms"
		echo
		echo "| Scenario | Baseline (ms) | Candidate (ms) | Delta (ms) | Delta (%) | Status |"
		echo "| --- | ---: | ---: | ---: | ---: | --- |"
		jq -r '
			def fmt:
				if . == null then "-"
				else (((. * 1000) | round) / 1000 | tostring)
				end;
			.scenarios[]
			| "| `\(.name)` | \((.baseline_ms | fmt)) | \((.candidate_ms | fmt)) | \((.delta_ms | fmt)) | \((.delta_pct | fmt)) | \(.status) |"
		' "$output"
		echo
		echo "Regressions: $(jq '.regression_count' "$output")"
		echo "Improvements: $(jq '.improvement_count' "$output")"
		echo "Missing scenarios: $(jq '.missing_count' "$output")"
	} >"$markdown"
fi

status=$(jq -r '.status' "$output")
regression_count=$(jq -r '.regression_count' "$output")
missing_count=$(jq -r '.missing_count' "$output")

echo "comparison status: $status"
echo "regression_count: $regression_count"
echo "missing_count: $missing_count"

if ((missing_count > 0)); then
	echo "missing scenarios detected between baseline and candidate" >&2
	exit 2
fi

if [[ "$fail_on_regression" == true && "$regression_count" -gt 0 ]]; then
	echo "benchmark regressions exceeded thresholds" >&2
	exit 3
fi
