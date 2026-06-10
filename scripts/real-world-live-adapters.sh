#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ELF_REAL_WORLD_LIVE_REPORT_DIR:-${ROOT_DIR}/tmp/real-world-memory/live-adapters}"
FIXTURE_DIR="${ELF_REAL_WORLD_LIVE_FIXTURES:-${ROOT_DIR}/apps/elf-eval/fixtures/real_world_memory}"
WORK_DIR="${ELF_REAL_WORLD_LIVE_WORK_DIR:-/bench/real-world-live-adapters}"
QMD_DIR="${ELF_REAL_WORLD_QMD_DIR:-/bench/repos/qmd}"

if [[ ! -f "/.dockerenv" && "${ELF_REAL_WORLD_LIVE_ALLOW_HOST:-0}" != "1" ]]; then
  echo "Refusing to run live real-world adapters outside Docker. Use cargo make real-world-memory-live-adapters." >&2
  exit 1
fi

for cmd in bash cargo git jq npm npx; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing ${cmd} in live adapter runner." >&2
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
  "${REPORT_DIR:?}/lightrag" \
  "${REPORT_DIR:?}/summary.json"

cd "${ROOT_DIR}"

cargo run -p elf-eval --bin real_world_live_adapter -- elf \
  --fixtures "${FIXTURE_DIR}" \
  --out-fixtures "${REPORT_DIR}/elf-fixtures" \
  --evidence-out "${REPORT_DIR}/elf-materialization.json" \
  --config config/local/elf.docker.toml

cargo run -p elf-eval --bin real_world_job_benchmark -- run \
  --fixtures "${REPORT_DIR}/elf-fixtures" \
  --out "${REPORT_DIR}/elf-report.json" \
  --run-id real-world-memory-live-elf \
  --adapter-id elf_live_real_world \
  --adapter-name "ELF live real-world service adapter" \
  --adapter-behavior live_real_world_adapter \
  --adapter-storage-status pass \
  --adapter-runtime-status pass \
  --adapter-notes "Materialized by real_world_live_adapter through ElfService, worker indexing, and search_raw across the encoded real-world suite corpus; unsupported suite capabilities remain typed non-pass records."

cargo run -p elf-eval --bin real_world_job_benchmark -- publish \
  --report "${REPORT_DIR}/elf-report.json" \
  --out "${REPORT_DIR}/elf-report.md"

cargo run -p elf-eval --bin real_world_live_adapter -- qmd \
  --fixtures "${FIXTURE_DIR}" \
  --out-fixtures "${REPORT_DIR}/qmd-fixtures" \
  --evidence-out "${REPORT_DIR}/qmd-materialization.json" \
  --qmd-dir "${QMD_DIR}" \
  --work-dir "${WORK_DIR}/qmd"

cargo run -p elf-eval --bin real_world_job_benchmark -- run \
  --fixtures "${REPORT_DIR}/qmd-fixtures" \
  --out "${REPORT_DIR}/qmd-report.json" \
  --run-id real-world-memory-live-qmd \
  --adapter-id qmd_live_real_world \
  --adapter-name "qmd live real-world CLI adapter" \
  --adapter-behavior live_real_world_adapter \
  --adapter-storage-status pass \
  --adapter-runtime-status pass \
  --adapter-notes "Materialized by real_world_live_adapter through qmd collection add, update, embed, and query --json across the encoded real-world suite corpus; unsupported suite capabilities remain typed non-pass records."

cargo run -p elf-eval --bin real_world_job_benchmark -- publish \
  --report "${REPORT_DIR}/qmd-report.json" \
  --out "${REPORT_DIR}/qmd-report.md"

if [[ "${ELF_REAL_WORLD_LIVE_ENABLE_LIGHTRAG:-0}" == "1" ]]; then
  ELF_LIGHTRAG_CONTEXT_REPORT_DIR="${REPORT_DIR}/lightrag" \
    ELF_LIGHTRAG_CONTEXT_FIXTURES="${ELF_LIGHTRAG_CONTEXT_FIXTURES:-${FIXTURE_DIR}/retrieval}" \
    bash scripts/lightrag-docker-context-smoke.sh
fi

jq -n \
  --slurpfile elf_materialization "${REPORT_DIR}/elf-materialization.json" \
  --slurpfile qmd_materialization "${REPORT_DIR}/qmd-materialization.json" \
  --slurpfile elf_report "${REPORT_DIR}/elf-report.json" \
  --slurpfile qmd_report "${REPORT_DIR}/qmd-report.json" \
  '{
    schema: "elf.real_world_live_adapter_sweep/v1",
    generated_at: (now | todateiso8601),
    artifact_dir: (env.ELF_REAL_WORLD_LIVE_REPORT_DIR // "tmp/real-world-memory/live-adapters"),
    fixture_dir: (env.ELF_REAL_WORLD_LIVE_FIXTURES // "apps/elf-eval/fixtures/real_world_memory"),
    adapters: [
      {
        adapter_id: "elf_live_real_world",
        evidence_class: "live_real_world",
        materialization: $elf_materialization[0],
        report: {
          json: "tmp/real-world-memory/live-adapters/elf-report.json",
          markdown: "tmp/real-world-memory/live-adapters/elf-report.md",
          summary: $elf_report[0].summary,
          suites: $elf_report[0].suites
        }
      },
      {
        adapter_id: "qmd_live_real_world",
        evidence_class: "live_real_world",
        materialization: $qmd_materialization[0],
        report: {
          json: "tmp/real-world-memory/live-adapters/qmd-report.json",
          markdown: "tmp/real-world-memory/live-adapters/qmd-report.md",
          summary: $qmd_report[0].summary,
          suites: $qmd_report[0].suites
        }
      }
    ]
  }' >"${REPORT_DIR}/summary.json"

if [[ -f "${REPORT_DIR}/lightrag/summary.json" ]]; then
  jq \
    --slurpfile lightrag_summary "${REPORT_DIR}/lightrag/summary.json" \
    '.adapters += [
      {
        adapter_id: $lightrag_summary[0].adapter_id,
        evidence_class: $lightrag_summary[0].evidence_class,
        materialization: $lightrag_summary[0].materialization,
        report: $lightrag_summary[0].report
      }
    ]' "${REPORT_DIR}/summary.json" >"${REPORT_DIR}/summary.json.tmp"
  mv "${REPORT_DIR}/summary.json.tmp" "${REPORT_DIR}/summary.json"
fi

echo "Live real-world adapter reports:"
echo "  ${REPORT_DIR}/elf-report.json"
echo "  ${REPORT_DIR}/elf-report.md"
echo "  ${REPORT_DIR}/qmd-report.json"
echo "  ${REPORT_DIR}/qmd-report.md"
if [[ -f "${REPORT_DIR}/lightrag/summary.json" ]]; then
  echo "  ${REPORT_DIR}/lightrag/lightrag-report.json"
  echo "  ${REPORT_DIR}/lightrag/lightrag-report.md"
fi
echo "  ${REPORT_DIR}/summary.json"
