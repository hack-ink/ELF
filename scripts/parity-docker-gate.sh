#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ELF_PARITY_REPORT_DIR:-${ROOT_DIR}/tmp/parity}"
RUN_ID="${ELF_PARITY_RUN_ID:-parity-$(date +%Y%m%d%H%M%S)}"

if [[ ! -f "/.dockerenv" && "${ELF_PARITY_ALLOW_HOST:-0}" != "1" ]]; then
  echo "Refusing to run parity gate outside Docker. Use cargo make parity-docker." >&2
  exit 1
fi

for cmd in cargo curl jq psql; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing ${cmd} in parity runner." >&2
    exit 1
  fi
done

mkdir -p "${REPORT_DIR}" "${ROOT_DIR}/tmp"

ADAPTER_OUT="${REPORT_DIR}/agentmemory-adapter.json"
CONSOLIDATION_LOG="${REPORT_DIR}/consolidation-harness.log"
CONSOLIDATION_BEFORE="${REPORT_DIR}/consolidation-before.json"
CONSOLIDATION_AFTER="${REPORT_DIR}/consolidation-after.json"
REPORT_OUT="${REPORT_DIR}/competitive-parity-report.json"

write_report() {
  local verdict="$1"
  local failure_reason="${2:-}"
  local adapter_status="${3:-not_run}"
  local consolidation_status="${4:-not_run}"

  local note_candidates="0"
  local doc_candidates="0"
  local baseline_queries="0"
  local ignored_items="0"
  local provenance_completeness="0"
  local unsupported_kind_rejected="false"
  local base_recall="0"
  local after_recall="0"
  local base_context="0"
  local after_context="0"

  if [[ -f "${ADAPTER_OUT}" ]]; then
    note_candidates="$(jq -r '.summary.note_candidate_count // 0' "${ADAPTER_OUT}")"
    doc_candidates="$(jq -r '.summary.doc_candidate_count // 0' "${ADAPTER_OUT}")"
    baseline_queries="$(jq -r '.summary.baseline_query_count // 0' "${ADAPTER_OUT}")"
    ignored_items="$(jq -r '.summary.ignored_count // 0' "${ADAPTER_OUT}")"
    provenance_completeness="$(
      jq -r '
        if (.summary.note_candidate_count // 0) == 0 then
          0
        else
          (
            [
              .note_candidates[]
              | select(
                  .notes_ingest_item.source_ref.resolver == "agentmemory_fixture/v1"
                  and (.notes_ingest_item.source_ref.ref.fixture_id | type == "string")
                  and (.notes_ingest_item.source_ref.ref.session_id | type == "string")
                  and (.notes_ingest_item.source_ref.ref.memory_id | type == "string")
                )
            ] | length
          ) / .summary.note_candidate_count
        end
      ' "${ADAPTER_OUT}"
    )"
    unsupported_kind_rejected="$(
      jq -r '[.ignored_items[]? | select(.reason == "unsupported_memory_kind")] | length > 0' \
        "${ADAPTER_OUT}"
    )"
  fi

  if [[ -f "${CONSOLIDATION_BEFORE}" ]]; then
    base_recall="$(jq -r '.summary.avg_recall_at_k // 0' "${CONSOLIDATION_BEFORE}")"
    base_context="$(jq -r '.summary.avg_retrieved_summary_chars // 0' "${CONSOLIDATION_BEFORE}")"
  fi

  if [[ -f "${CONSOLIDATION_AFTER}" ]]; then
    after_recall="$(jq -r '.summary.avg_recall_at_k // 0' "${CONSOLIDATION_AFTER}")"
    after_context="$(jq -r '.summary.avg_retrieved_summary_chars // 0' "${CONSOLIDATION_AFTER}")"
  fi

  jq -n \
    --arg schema "elf.competitive_parity_gate.report/v1" \
    --arg gate_schema "elf.competitive_parity_gate/v1" \
    --arg gate_id "${RUN_ID}" \
    --arg verdict "${verdict}" \
    --arg failure_reason "${failure_reason}" \
    --arg adapter_status "${adapter_status}" \
    --arg consolidation_status "${consolidation_status}" \
    --argjson note_candidates "${note_candidates}" \
    --argjson doc_candidates "${doc_candidates}" \
    --argjson baseline_queries "${baseline_queries}" \
    --argjson ignored_items "${ignored_items}" \
    --argjson provenance_completeness "${provenance_completeness}" \
    --argjson unsupported_kind_rejected "${unsupported_kind_rejected}" \
    --argjson base_recall "${base_recall}" \
    --argjson after_recall "${after_recall}" \
    --argjson base_context "${base_context}" \
    --argjson after_context "${after_context}" \
    '{
      schema: $schema,
      gate_schema: $gate_schema,
      gate_id: $gate_id,
      verdict: $verdict,
      failure_reason: (if $failure_reason == "" then null else $failure_reason end),
      docker_only: true,
      baselines: {
        agentmemory_fixture: {
          status: $adapter_status,
          note_candidate_count: $note_candidates,
          doc_candidate_count: $doc_candidates,
          baseline_query_count: $baseline_queries,
          ignored_count: $ignored_items,
          provenance_completeness: $provenance_completeness,
          unsupported_kind_rejected: $unsupported_kind_rejected
        },
        elf_consolidation_harness: {
          status: $consolidation_status,
          baseline_avg_recall_at_k: $base_recall,
          after_avg_recall_at_k: $after_recall,
          baseline_avg_retrieved_summary_chars: $base_context,
          after_avg_retrieved_summary_chars: $after_context
        }
      },
      dimensions: {
        docker_isolation: {status: "pass"},
        adapter_coverage: {
          status: (if $note_candidates == 2 and $doc_candidates == 2 and $baseline_queries == 1 and $ignored_items == 1 then "pass" else "fail" end)
        },
        provenance_integrity: {
          status: (if $provenance_completeness == 1 then "pass" else "fail" end)
        },
        unsafe_rejection: {
          status: (if $unsupported_kind_rejected then "pass" else "fail" end)
        },
        retrieval_quality: {
          status: (if $consolidation_status == "pass" and $after_recall >= $base_recall then "pass" else "fail" end)
        },
        context_efficiency: {
          status: (if $consolidation_status == "pass" and $after_context <= $base_context then "pass" else "fail" end)
        },
        source_safety: {
          status: (if $consolidation_status == "pass" then "pass" else "fail" end)
        },
        operator_inspectability: {
          status: (if $consolidation_status == "pass" then "pass" else "fail" end),
          checked_route: "GET /viewer"
        },
        cleanup: {
          status: "documented",
          command: "cargo make clean-parity-docker"
        }
      },
      thresholds: {
        agentmemory_fixture: {
          note_candidate_count: 2,
          doc_candidate_count: 2,
          baseline_query_count: 1,
          ignored_count: 1,
          provenance_completeness: 1,
          requires_unsupported_memory_kind_rejection: true
        },
        consolidation: {
          after_recall_must_be_at_least_baseline: true,
          after_context_chars_must_not_exceed_baseline: true,
          viewer_must_return_200: true
        }
      },
      artifacts: {
        adapter_output: "tmp/parity/agentmemory-adapter.json",
        consolidation_log: "tmp/parity/consolidation-harness.log",
        consolidation_before: "tmp/parity/consolidation-before.json",
        consolidation_after: "tmp/parity/consolidation-after.json"
      }
    }' >"${REPORT_OUT}"
}

fail_gate() {
  local reason="$1"
  local adapter_status="${2:-fail}"
  local consolidation_status="${3:-fail}"
  write_report "fail" "${reason}" "${adapter_status}" "${consolidation_status}"
  echo "Parity gate failed: ${reason}" >&2
  echo "Report: ${REPORT_OUT}" >&2
  exit 1
}

assert_passing_report() {
  jq -e '
    .verdict == "pass"
    and ([.dimensions | to_entries[] | select(.key != "cleanup" and .value.status != "pass")] | length == 0)
  ' "${REPORT_OUT}" >/dev/null
}

echo "Waiting for Docker service dependencies."
for _ in $(seq 1 120); do
  if psql "${ELF_PG_DSN}" -tAc "SELECT 1" >/dev/null 2>&1 \
    && curl -fsS "${ELF_QDRANT_HTTP_URL}/collections" >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done

if ! psql "${ELF_PG_DSN}" -tAc "SELECT 1" >/dev/null 2>&1; then
  fail_gate "postgres dependency did not become reachable" "not_run" "not_run"
fi

if ! curl -fsS "${ELF_QDRANT_HTTP_URL}/collections" >/dev/null 2>&1; then
  fail_gate "qdrant dependency did not become reachable" "not_run" "not_run"
fi

echo "Running agentmemory fixture adapter gate."
(cd "${ROOT_DIR}" && cargo run -q -p elf-eval --bin agentmemory_fixture_adapter -- \
  --fixture apps/elf-eval/fixtures/agentmemory/sample_session.json \
  --out "${ADAPTER_OUT}") || fail_gate "agentmemory fixture adapter command failed" "fail" "not_run"

jq -e '
  .schema == "elf.agentmemory_adapter/v1"
  and .summary.note_candidate_count == 2
  and .summary.doc_candidate_count == 2
  and .summary.baseline_query_count == 1
  and .summary.ignored_count == 1
  and (
    [
      .note_candidates[]
      | select(
          .notes_ingest_item.source_ref.resolver != "agentmemory_fixture/v1"
          or (.notes_ingest_item.source_ref.ref.fixture_id | type != "string")
          or (.notes_ingest_item.source_ref.ref.session_id | type != "string")
          or (.notes_ingest_item.source_ref.ref.memory_id | type != "string")
        )
    ] | length == 0
  )
  and ([.ignored_items[]? | select(.reason == "unsupported_memory_kind")] | length >= 1)
' "${ADAPTER_OUT}" >/dev/null \
  || fail_gate "agentmemory fixture adapter thresholds failed" "fail" "not_run"

echo "Running service-backed consolidation parity gate."
(
  cd "${ROOT_DIR}"
  ELF_HARNESS_CHECK_VIEWER=1 \
  bash scripts/consolidation-harness.sh
) 2>&1 | tee "${CONSOLIDATION_LOG}" \
  || fail_gate "consolidation harness thresholds failed" "pass" "fail"

cp "${ROOT_DIR}/tmp/elf.consolidation.out.base.json" "${CONSOLIDATION_BEFORE}"
cp "${ROOT_DIR}/tmp/elf.consolidation.out.after.json" "${CONSOLIDATION_AFTER}"

write_report "pass" "" "pass" "pass"
assert_passing_report || fail_gate "one or more parity report dimensions failed" "pass" "pass"

echo "Parity gate passed."
echo "Report: ${REPORT_OUT}"
