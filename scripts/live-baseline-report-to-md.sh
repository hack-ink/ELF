#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT="${1:-${ELF_BASELINE_REPORT:-${ROOT_DIR}/tmp/live-baseline/live-baseline-report.json}}"
OUT="${2:-${ELF_BASELINE_MARKDOWN_REPORT:-}}"
REPORT_DISPLAY="${REPORT}"
if [[ "${REPORT_DISPLAY}" == "${ROOT_DIR}/"* ]]; then
  REPORT_DISPLAY="${REPORT_DISPLAY#"${ROOT_DIR}/"}"
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "Missing jq; cannot render live baseline Markdown report." >&2
  exit 1
fi

if [[ ! -f "${REPORT}" ]]; then
  echo "Missing report: ${REPORT}" >&2
  exit 1
fi

render_report() {
  jq -r --arg report_path "${REPORT_DISPLAY}" '
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
    ("- Corpus track: `" + ((.corpus.track // "generated_public") | md) + "`"),
    (
      if (.corpus.manifest_id // null) == null then empty
      else "- Corpus manifest: `" + (.corpus.manifest_id | md) + "`"
      end
    ),
    ("- Documents: `" + (.corpus.document_count | tostring) + "`"),
    ("- Queries: `" + (.corpus.query_count | tostring) + "`"),
    ("- Wrong-result count: `" + ((.wrong_result_count // 0) | tostring) + "`"),
    ("- Query latency mean: `" + ((.latency_ms.mean // 0) | tostring) + " ms`"),
    ("- Project summary: `" + (.summary.pass // 0 | tostring) + " pass`, `" + (.summary.wrong_result // 0 | tostring) + " wrong_result`, `" + (.summary.lifecycle_fail // 0 | tostring) + " lifecycle_fail`, `" + (.summary.blocked // 0 | tostring) + " blocked`, `" + (.summary.incomplete // 0 | tostring) + " incomplete`, `" + (.summary.not_encoded // 0 | tostring) + " not_encoded`"),
    ("- Same-corpus summary: `" + (.same_corpus_summary.pass // 0 | tostring) + " pass`, `" + (.same_corpus_summary.wrong_result // 0 | tostring) + " wrong_result`, `" + (.same_corpus_summary.blocked // 0 | tostring) + " blocked`, `" + (.same_corpus_summary.incomplete // 0 | tostring) + " incomplete`, `" + (.same_corpus_summary.not_encoded // 0 | tostring) + " not_encoded`"),
    ("- Full check summary: `" + (.full_check_summary.pass // 0 | tostring) + "/" + (.full_check_summary.total // 0 | tostring) + " pass`, `" + (.full_check_summary.wrong_result // 0 | tostring) + " wrong_result`, `" + (.full_check_summary.lifecycle_fail // 0 | tostring) + " lifecycle_fail`, `" + (.full_check_summary.blocked // 0 | tostring) + " blocked`, `" + (.full_check_summary.incomplete // 0 | tostring) + " incomplete`, `" + (.full_check_summary.not_encoded // 0 | tostring) + " not_encoded`"),
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
      [.projects[] | select(.adapter != null)] as $adapters
      | if ($adapters | length) > 0 then
          "## Adapter Behavior",
          "",
          "| Project | Storage | Retrieval | Update | Delete/Expire | Cold Start | Scale/Stress |",
          "| --- | --- | --- | --- | --- | --- | --- |",
          (
            $adapters[]
            | "| " + (.project | md)
              + " | `" + (.adapter.storage.status | md) + "`"
              + " | `" + (.adapter.behaviors.same_corpus_retrieval.status | md) + "`"
              + " | `" + (.adapter.behaviors.update.status | md) + "`"
              + " | `" + (.adapter.behaviors.delete_or_expire.status | md) + "`"
              + " | `" + (.adapter.behaviors.cold_start_reload.status | md) + "`"
              + " | `" + (.adapter.behaviors.scale_stress_profile.status | md) + "` |"
          ),
          ""
        else empty end
    ),
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
    (
      [.projects[] | {project, queries: (.queries // [])} | select((.queries | length) > 0)] as $query_projects
      | if ($query_projects | length) > 0 then
          "## Query Evidence",
          "",
          "| Project | Query | Task | Expected Evidence | Allowed Alternates | Top Evidence | Matched | Latency |",
          "| --- | --- | --- | --- | --- | --- | --- | --- |",
          (
            $query_projects[]
            | .project as $project
            | .queries[]
            | "| " + ($project | md)
              + " | `" + (.id | md) + "`"
              + " | `" + ((.task // "-") | md) + "`"
              + " | `" + (((.expected_evidence_ids // []) | join(", ")) | md) + "`"
              + " | `" + (((.allowed_alternate_evidence_ids // []) | join(", ")) | md) + "`"
              + " | `" + ((.top_evidence_id // "-") | md) + "`"
              + " | `" + (.matched | tostring) + "`"
              + " | `" + ((.latency_ms // 0) | tostring) + " ms` |"
          ),
          ""
        else empty end
    ),
    (
      [.projects[] | select(.backfill != null)] as $backfilled
      | if ($backfilled | length) > 0 then
          "## Backfill",
          "",
          "| Project | Sources | Completed | Batch | Workers | Resume | Duplicates | Backfill Elapsed |",
          "| --- | --- | --- | --- | --- | --- | --- | --- |",
          (
            $backfilled[]
            | "| " + (.project | md)
              + " | `" + (.backfill.source_count | tostring) + "`"
              + " | `" + (.backfill.completed_count | tostring) + "`"
              + " | `" + (.backfill.batch_size | tostring) + "`"
              + " | `" + (.backfill.worker_concurrency | tostring) + "`"
              + " | `" + (
                  if .backfill.resume.enabled then
                    "resumed after " + (.backfill.resume.completed_before_resume | tostring)
                    + "/" + (.backfill.resume.completed_after_resume | tostring)
                  else
                    "disabled"
                  end
                ) + "`"
              + " | `" + ((.backfill.duplicate_source_notes | length) | tostring) + "`"
              + " | `" + (.backfill.elapsed_seconds | tostring) + "s` |"
          ),
          ""
        else empty end
    ),
    "## Result Semantics",
    "",
    "- `pass`: every encoded check for the selected project and profile passed.",
    "- `wrong_result`: a retrieval check completed but returned the wrong memory or missed expected evidence.",
    "- `lifecycle_fail`: same-corpus retrieval may pass, but an encoded update, delete, cold-start, persistence, or related lifecycle check failed.",
    "- `incomplete`: setup or a declared check could not complete because install, runtime, dependency, or adapter wiring failed in Docker.",
    "- `blocked`: a safe check cannot run without external credentials, manual setup, durable runtime wiring, or host integration outside this run.",
    "- `not_encoded`: the capability is not covered by the current adapter, so no pass/fail claim is allowed.",
    "",
    "`incomplete`, `blocked`, and `not_encoded` are not passes; treat them as benchmark coverage debt."
  ' "${REPORT}"
}

if [[ -n "${OUT}" ]]; then
  mkdir -p "$(dirname "${OUT}")"
  render_report >"${OUT}"
  echo "Wrote ${OUT}"
else
  render_report
fi
