#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ELF_LIGHTRAG_CONTEXT_REPORT_DIR:-${ROOT_DIR}/tmp/real-world-memory/lightrag-context}"
FIXTURE_DIR="${ELF_LIGHTRAG_CONTEXT_FIXTURES:-${ROOT_DIR}/apps/elf-eval/fixtures/real_world_memory/retrieval}"
WORK_DIR="${ELF_LIGHTRAG_CONTEXT_WORK_DIR:-/bench/real-world-live-adapters/lightrag}"
API_BASE="${ELF_LIGHTRAG_API_BASE:-http://lightrag:9621}"
ADAPTER_ID="${ELF_LIGHTRAG_ADAPTER_ID:-lightrag_live_real_world}"
ADAPTER_NAME="${ELF_LIGHTRAG_ADAPTER_NAME:-LightRAG Docker context-export adapter}"
STARTUP_ATTEMPTS="${ELF_LIGHTRAG_STARTUP_ATTEMPTS:-6}"
STARTUP_INTERVAL_SECONDS="${ELF_LIGHTRAG_STARTUP_INTERVAL_SECONDS:-2}"
INDEX_ATTEMPTS="${ELF_LIGHTRAG_INDEX_ATTEMPTS:-60}"
INDEX_INTERVAL_SECONDS="${ELF_LIGHTRAG_INDEX_INTERVAL_SECONDS:-2}"

if [[ ! -f "/.dockerenv" && "${ELF_LIGHTRAG_CONTEXT_ALLOW_HOST:-0}" != "1" ]]; then
  echo "Refusing to run LightRAG context smoke outside Docker. Use cargo make lightrag-docker-context-smoke." >&2
  exit 1
fi

for cmd in cargo jq; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing ${cmd} in LightRAG context smoke runner." >&2
    exit 1
  fi
done

mkdir -p "${REPORT_DIR}" "${WORK_DIR}"
rm -rf "${REPORT_DIR:?}/lightrag-fixtures" \
  "${REPORT_DIR:?}/lightrag-materialization.json" \
  "${REPORT_DIR:?}/lightrag-report.json" \
  "${REPORT_DIR:?}/lightrag-report.md" \
  "${REPORT_DIR:?}/summary.json"

cd "${ROOT_DIR}"

cargo run -p elf-eval --bin real_world_live_adapter -- lightrag \
  --fixtures "${FIXTURE_DIR}" \
  --out-fixtures "${REPORT_DIR}/lightrag-fixtures" \
  --evidence-out "${REPORT_DIR}/lightrag-materialization.json" \
  --work-dir "${WORK_DIR}" \
  --api-base "${API_BASE}" \
  --adapter-id "${ADAPTER_ID}" \
  --startup-attempts "${STARTUP_ATTEMPTS}" \
  --startup-interval-seconds "${STARTUP_INTERVAL_SECONDS}" \
  --index-attempts "${INDEX_ATTEMPTS}" \
  --index-interval-seconds "${INDEX_INTERVAL_SECONDS}"

MATERIALIZATION_STATUS="$(jq -r '.status' "${REPORT_DIR}/lightrag-materialization.json")"

cargo run -p elf-eval --bin real_world_job_benchmark -- run \
  --fixtures "${REPORT_DIR}/lightrag-fixtures" \
  --out "${REPORT_DIR}/lightrag-report.json" \
  --run-id real-world-memory-live-lightrag \
  --adapter-id "${ADAPTER_ID}" \
  --adapter-name "${ADAPTER_NAME}" \
  --adapter-behavior docker_api_context_export \
  --adapter-storage-status "${MATERIALIZATION_STATUS}" \
  --adapter-runtime-status "${MATERIALIZATION_STATUS}" \
  --adapter-notes "Materialized by real_world_live_adapter through the LightRAG Docker API using generated source file paths, /documents/texts ingest, /query context export, and reference/content evidence mapping; non-executed suites remain typed non-pass records."

cargo run -p elf-eval --bin real_world_job_benchmark -- publish \
  --report "${REPORT_DIR}/lightrag-report.json" \
  --out "${REPORT_DIR}/lightrag-report.md"

jq -n \
  --slurpfile materialization "${REPORT_DIR}/lightrag-materialization.json" \
  --slurpfile report "${REPORT_DIR}/lightrag-report.json" \
  'def count($key): ($report[0].summary[$key] // 0);
  def scored_status:
    if count("wrong_result") > 0 then "wrong_result"
    elif count("lifecycle_fail") > 0 then "lifecycle_fail"
    elif count("incomplete") > 0 then "incomplete"
    elif count("blocked") > 0 then "blocked"
    elif count("not_encoded") > 0 then "not_encoded"
    elif count("pass") > 0 then "pass"
    else "not_encoded"
    end;
  {
    schema: "elf.lightrag_context_export_smoke/v1",
    generated_at: (now | todateiso8601),
    artifact_dir: (env.ELF_LIGHTRAG_CONTEXT_REPORT_DIR // "tmp/real-world-memory/lightrag-context"),
    fixture_dir: (env.ELF_LIGHTRAG_CONTEXT_FIXTURES // "apps/elf-eval/fixtures/real_world_memory/retrieval"),
    adapter_id: (env.ELF_LIGHTRAG_ADAPTER_ID // "lightrag_live_real_world"),
    evidence_class: (
      if ($materialization[0].status == "pass" or $materialization[0].status == "wrong_result") then
        "live_real_world"
      else
        "research_gate"
      end
    ),
    status_boundary: {
      materialization: "API reachability, ingest, context export, and evidence-mapping state emitted by the adapter",
      report: "post-score real_world_job outcome; use this for quality status"
    },
    scored_benchmark: {
      schema: "elf.scored_benchmark_status/v1",
      source: "real_world_job_benchmark",
      status: scored_status,
      counts: {
        pass: count("pass"),
        wrong_result: count("wrong_result"),
        lifecycle_fail: count("lifecycle_fail"),
        incomplete: count("incomplete"),
        blocked: count("blocked"),
        not_encoded: count("not_encoded")
      },
      job_count: ($report[0].summary.job_count // 0),
      mean_score: ($report[0].summary.mean_score // null),
      evidence_coverage: ($report[0].summary.evidence_coverage // null)
    },
    materialization: $materialization[0],
    report: {
      json: "tmp/real-world-memory/lightrag-context/lightrag-report.json",
      markdown: "tmp/real-world-memory/lightrag-context/lightrag-report.md",
      summary: $report[0].summary,
      suites: $report[0].suites
    }
  }' >"${REPORT_DIR}/summary.json"

echo "LightRAG context-export smoke reports:"
echo "  ${REPORT_DIR}/lightrag-materialization.json"
echo "  ${REPORT_DIR}/lightrag-report.json"
echo "  ${REPORT_DIR}/lightrag-report.md"
echo "  ${REPORT_DIR}/summary.json"
