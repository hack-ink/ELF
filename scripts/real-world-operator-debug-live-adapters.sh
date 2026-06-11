#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ELF_OPERATOR_DEBUG_LIVE_REPORT_DIR:-${ROOT_DIR}/tmp/real-world-job/operator-ux-live-adapters}"
FIXTURE_DIR="${ELF_OPERATOR_DEBUG_LIVE_FIXTURES:-${ROOT_DIR}/apps/elf-eval/fixtures/real_world_job/operator_debugging_ux}"
WORK_DIR="${ELF_OPERATOR_DEBUG_LIVE_WORK_DIR:-/bench/operator-debug-live-adapters}"
QMD_DIR="${ELF_OPERATOR_DEBUG_QMD_DIR:-/bench/repos/qmd}"

if [[ ! -f "/.dockerenv" && "${ELF_OPERATOR_DEBUG_LIVE_ALLOW_HOST:-0}" != "1" ]]; then
  echo "Refusing to run operator-debug live adapters outside Docker. Use cargo make real-world-job-operator-ux-live-adapters." >&2
  exit 1
fi

for cmd in bash cargo git jq npm npx; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing ${cmd} in operator-debug live adapter runner." >&2
    exit 1
  fi
done

mkdir -p "${REPORT_DIR}" "${WORK_DIR}"
rm -rf "${REPORT_DIR:?}/elf-fixtures" \
  "${REPORT_DIR:?}/qmd-fixtures" \
  "${REPORT_DIR:?}/elf-materialization.json" \
  "${REPORT_DIR:?}/qmd-materialization.json" \
  "${REPORT_DIR:?}/elf-report.json" \
  "${REPORT_DIR:?}/elf-report.md" \
  "${REPORT_DIR:?}/qmd-report.json" \
  "${REPORT_DIR:?}/qmd-report.md" \
  "${REPORT_DIR:?}/summary.json"

cd "${ROOT_DIR}"

cargo run -p elf-eval --bin real_world_live_adapter -- elf \
  --fixtures "${FIXTURE_DIR}" \
  --out-fixtures "${REPORT_DIR}/elf-fixtures" \
  --evidence-out "${REPORT_DIR}/elf-materialization.json" \
  --config config/local/elf.docker.toml \
  --adapter-id elf_operator_debug_live

cargo run -p elf-eval --bin real_world_job_benchmark -- run \
  --fixtures "${REPORT_DIR}/elf-fixtures" \
  --out "${REPORT_DIR}/elf-report.json" \
  --run-id real-world-operator-debug-live-elf \
  --adapter-id elf_operator_debug_live \
  --adapter-name "ELF live operator-debug service adapter" \
  --adapter-behavior live_operator_debug_adapter \
  --adapter-storage-status pass \
  --adapter-runtime-status pass \
  --adapter-notes "Materialized by real_world_live_adapter through ElfService, worker indexing, search_raw trace ids, and operator-debug trace metadata."

cargo run -p elf-eval --bin real_world_job_benchmark -- publish \
  --report "${REPORT_DIR}/elf-report.json" \
  --out "${REPORT_DIR}/elf-report.md"

cargo run -p elf-eval --bin real_world_live_adapter -- qmd \
  --fixtures "${FIXTURE_DIR}" \
  --out-fixtures "${REPORT_DIR}/qmd-fixtures" \
  --evidence-out "${REPORT_DIR}/qmd-materialization.json" \
  --qmd-dir "${QMD_DIR}" \
  --work-dir "${WORK_DIR}/qmd" \
  --adapter-id qmd_operator_debug_live

cargo run -p elf-eval --bin real_world_job_benchmark -- run \
  --fixtures "${REPORT_DIR}/qmd-fixtures" \
  --out "${REPORT_DIR}/qmd-report.json" \
  --run-id real-world-operator-debug-live-qmd \
  --adapter-id qmd_operator_debug_live \
  --adapter-name "qmd live operator-debug CLI adapter" \
  --adapter-behavior live_operator_debug_adapter \
  --adapter-storage-status pass \
  --adapter-runtime-status pass \
  --adapter-notes "Materialized by real_world_live_adapter through qmd collection add, update, embed, query --json, and local replay command metadata; ELF trace/viewer surfaces are not inferred."

cargo run -p elf-eval --bin real_world_job_benchmark -- publish \
  --report "${REPORT_DIR}/qmd-report.json" \
  --out "${REPORT_DIR}/qmd-report.md"

jq -n \
  --slurpfile elf_materialization "${REPORT_DIR}/elf-materialization.json" \
  --slurpfile qmd_materialization "${REPORT_DIR}/qmd-materialization.json" \
  --slurpfile elf_report "${REPORT_DIR}/elf-report.json" \
  --slurpfile qmd_report "${REPORT_DIR}/qmd-report.json" \
  '{
    schema: "elf.real_world_operator_debug_live_adapter_sweep/v1",
    generated_at: (now | todateiso8601),
    artifact_dir: (env.ELF_OPERATOR_DEBUG_LIVE_REPORT_DIR // "tmp/real-world-job/operator-ux-live-adapters"),
    fixture_dir: (env.ELF_OPERATOR_DEBUG_LIVE_FIXTURES // "apps/elf-eval/fixtures/real_world_job/operator_debugging_ux"),
    adapters: [
      {
        adapter_id: "elf_operator_debug_live",
        evidence_class: "live_real_world",
        materialization: $elf_materialization[0],
        report: {
          json: "tmp/real-world-job/operator-ux-live-adapters/elf-report.json",
          markdown: "tmp/real-world-job/operator-ux-live-adapters/elf-report.md",
          summary: $elf_report[0].summary,
          suites: $elf_report[0].suites
        }
      },
      {
        adapter_id: "qmd_operator_debug_live",
        evidence_class: "live_real_world",
        materialization: $qmd_materialization[0],
        report: {
          json: "tmp/real-world-job/operator-ux-live-adapters/qmd-report.json",
          markdown: "tmp/real-world-job/operator-ux-live-adapters/qmd-report.md",
          summary: $qmd_report[0].summary,
          suites: $qmd_report[0].suites
        }
      }
    ],
    scenario_dimensions: [
      "trace_available",
      "replay_command_available",
      "candidate_drop_visibility",
      "repair_action_clarity",
      "raw_sql_needed"
    ],
    boundary: "This narrow sweep scores operator-debugging fixtures only. It does not change core ranking, launch OpenMemory or claude-mem UI flows, or convert fixture-only UX evidence into broad product superiority."
  }' >"${REPORT_DIR}/summary.json"

echo "Operator-debug live adapter reports:"
echo "  ${REPORT_DIR}/elf-report.json"
echo "  ${REPORT_DIR}/elf-report.md"
echo "  ${REPORT_DIR}/qmd-report.json"
echo "  ${REPORT_DIR}/qmd-report.md"
echo "  ${REPORT_DIR}/summary.json"
