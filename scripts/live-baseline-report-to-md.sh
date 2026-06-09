#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT="${1:-${ELF_BASELINE_REPORT:-${ROOT_DIR}/tmp/live-baseline/live-baseline-report.json}}"
OUT="${2:-${ELF_BASELINE_MARKDOWN_REPORT:-}}"

if ! command -v jq >/dev/null 2>&1; then
  echo "Missing jq; cannot render live baseline Markdown report." >&2
  exit 1
fi

if [[ ! -f "${REPORT}" ]]; then
  echo "Missing report: ${REPORT}" >&2
  exit 1
fi

render_report() {
  jq -r --arg report_path "${REPORT}" '
    def dash:
      if . == null then "-" else tostring end;
    def md:
      dash | gsub("\\|"; "\\|") | gsub("\n"; " ");
    def checks:
      ((.check_summary.pass // 0 | tostring) + "/" + (.check_summary.total // 0 | tostring));

    "# Live Baseline Benchmark Report",
    "",
    "Goal: Publish a Markdown summary for one generated live baseline aggregate report.",
    "Read this when: You need a durable, reviewable summary of a live baseline JSON report.",
    ("Inputs: `" + $report_path + "`."),
    "Depends on: `scripts/live-baseline-benchmark.sh` and `docs/guide/benchmarking/live_baseline_benchmark.md`.",
    "Verification: Compare this Markdown summary with the source JSON before committing.",
    "",
    "## Summary",
    "",
    ("- Run ID: `" + (.run_id | md) + "`"),
    ("- Generated at: `" + (.generated_at | md) + "`"),
    ("- Verdict: `" + (.verdict | md) + "`"),
    ("- Project filter: `" + (.project_filter | md) + "`"),
    ("- Corpus profile: `" + (.corpus.profile | md) + "`"),
    ("- Documents: `" + (.corpus.document_count | tostring) + "`"),
    ("- Queries: `" + (.corpus.query_count | tostring) + "`"),
    ("- Project summary: `" + (.summary.pass | tostring) + " pass`, `" + (.summary.fail | tostring) + " fail`, `" + (.summary.incomplete | tostring) + " incomplete`"),
    ("- Same-corpus summary: `" + (.same_corpus_summary.pass | tostring) + " pass`, `" + (.same_corpus_summary.fail | tostring) + " fail`, `" + (.same_corpus_summary.incomplete | tostring) + " incomplete`"),
    ("- Full check summary: `" + (.full_check_summary.pass | tostring) + "/" + (.full_check_summary.total | tostring) + " pass`"),
    "",
    "## Projects",
    "",
    "| Project | Status | Retrieval | Checks | Elapsed | Reason |",
    "| --- | --- | --- | --- | --- | --- |",
    (
      .projects[]
      | "| " + (.project | md)
        + " | `" + (.status | md) + "`"
        + " | `" + (.retrieval_status | md) + "`"
        + " | `" + checks + "`"
        + " | `" + (.elapsed_seconds | tostring) + "s`"
        + " | " + (.reason | md) + " |"
    ),
    "",
    (
      [.projects[] | select(.embedding != null)] as $embedded
      | if ($embedded | length) > 0 then
          "## Embedding",
          "",
          "| Project | Mode | Provider | Model | Dimensions | Timeout | API Base | Path |",
          "| --- | --- | --- | --- | --- | --- | --- | --- |",
          (
            $embedded[]
            | "| " + (.project | md)
              + " | `" + (.embedding.mode | md) + "`"
              + " | `" + (.embedding.provider_id | md) + "`"
              + " | `" + (.embedding.model | md) + "`"
              + " | `" + (.embedding.dimensions | tostring) + "`"
              + " | `" + (.embedding.timeout_ms | tostring) + "ms`"
              + " | `" + (.embedding.api_base | md) + "`"
              + " | `" + (.embedding.path | md) + "` |"
          ),
          ""
        else empty end
    ),
    "## Result Semantics",
    "",
    "- `pass`: every encoded check for the selected project and profile passed.",
    "- `fail`: clone, install, import, build, retrieval, lifecycle, recovery, concurrency, soak, resource-envelope, or another declared check failed.",
    "- `incomplete`: the encoded check could not complete without extra provider keys, host integration, native dependency support, durable runtime wiring, or more adapter work.",
    "",
    "`incomplete` is not a pass; treat it as benchmark wiring debt."
  ' "${REPORT}"
}

if [[ -n "${OUT}" ]]; then
  mkdir -p "$(dirname "${OUT}")"
  render_report >"${OUT}"
  echo "Wrote ${OUT}"
else
  render_report
fi
