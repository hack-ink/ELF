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
  "${REPORT_DIR:?}/ragflow" \
  "${REPORT_DIR:?}/lightrag" \
  "${REPORT_DIR:?}/graphrag" \
  "${REPORT_DIR:?}/graphiti-zep" \
  "${REPORT_DIR:?}/graphify" \
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

if [[ "${ELF_REAL_WORLD_LIVE_ENABLE_RAGFLOW:-0}" == "1" ]]; then
  ELF_RAGFLOW_SMOKE_ARTIFACT_DIR="${REPORT_DIR}/ragflow" \
    bash scripts/ragflow-docker-evidence-smoke.sh
fi

if [[ "${ELF_REAL_WORLD_LIVE_ENABLE_LIGHTRAG:-0}" == "1" ]]; then
  ELF_LIGHTRAG_CONTEXT_REPORT_DIR="${REPORT_DIR}/lightrag" \
    ELF_LIGHTRAG_CONTEXT_FIXTURES="${ELF_LIGHTRAG_CONTEXT_FIXTURES:-${FIXTURE_DIR}/retrieval}" \
    bash scripts/lightrag-docker-context-smoke.sh
fi

if [[ "${ELF_REAL_WORLD_LIVE_ENABLE_GRAPHRAG:-0}" == "1" ]]; then
  ELF_GRAPHRAG_SMOKE_REPORT_DIR="${REPORT_DIR}/graphrag" \
    python3 scripts/graphrag-docker-smoke.py
fi

if [[ "${ELF_REAL_WORLD_LIVE_ENABLE_GRAPHITI_ZEP:-0}" == "1" ]]; then
  ELF_GRAPHITI_ZEP_SMOKE_REPORT_DIR="${REPORT_DIR}/graphiti-zep" \
    python3 scripts/graphiti-zep-docker-temporal-smoke.py
fi

if [[ "${ELF_REAL_WORLD_LIVE_ENABLE_GRAPHIFY:-0}" == "1" ]]; then
  ELF_GRAPHIFY_SMOKE_REPORT_DIR="${REPORT_DIR}/graphify" \
    python3 scripts/graphify-docker-graph-report-smoke.py
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
	    graph_rag_smoke_controls: {
	      inclusion_flags: {
	        ragflow: (env.ELF_REAL_WORLD_LIVE_ENABLE_RAGFLOW // "0"),
	        lightrag: (env.ELF_REAL_WORLD_LIVE_ENABLE_LIGHTRAG // "0"),
	        graphrag: (env.ELF_REAL_WORLD_LIVE_ENABLE_GRAPHRAG // "0"),
	        graphiti_zep: (env.ELF_REAL_WORLD_LIVE_ENABLE_GRAPHITI_ZEP // "0"),
	        graphify: (env.ELF_REAL_WORLD_LIVE_ENABLE_GRAPHIFY // "0")
	      },
	      live_attempt_boundary: "Inclusion flags only add smoke adapters to this aggregate sweep. Provider, service-start, and resource-heavy live attempts still require each adapter-specific control.",
	      service_start_controls: {
	        lightrag: (env.ELF_LIGHTRAG_CONTEXT_START // "0"),
	        graphiti_zep: (env.ELF_GRAPHITI_ZEP_SMOKE_START // "0")
	      },
	      provider_or_resource_controls_forwarded: [
	        "ELF_RAGFLOW_SMOKE_START",
	        "ELF_RAGFLOW_SMOKE_ACCEPT_RESOURCE_ENVELOPE",
	        "ELF_GRAPHRAG_SMOKE_RUN",
	        "ELF_GRAPHRAG_API_KEY",
	        "ELF_GRAPHITI_ZEP_SMOKE_RUN",
	        "ELF_GRAPHITI_ZEP_API_KEY",
	        "ELF_GRAPHIFY_SMOKE_RUN"
	      ]
	    },
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

if [[ -f "${REPORT_DIR}/ragflow/summary.json" ]]; then
  jq \
    --slurpfile ragflow_summary "${REPORT_DIR}/ragflow/summary.json" \
    '.adapters += [
      {
	        adapter_id: $ragflow_summary[0].adapter_id,
	        evidence_class: $ragflow_summary[0].evidence_class,
	        status_boundary: $ragflow_summary[0].status_boundary,
	        scored_benchmark: $ragflow_summary[0].scored_benchmark,
	        materialization: $ragflow_summary[0].materialization,
	        report: $ragflow_summary[0].report
      }
    ]' "${REPORT_DIR}/summary.json" >"${REPORT_DIR}/summary.json.tmp"
  mv "${REPORT_DIR}/summary.json.tmp" "${REPORT_DIR}/summary.json"
fi

if [[ -f "${REPORT_DIR}/lightrag/summary.json" ]]; then
  jq \
    --slurpfile lightrag_summary "${REPORT_DIR}/lightrag/summary.json" \
    '.adapters += [
      {
	        adapter_id: $lightrag_summary[0].adapter_id,
	        evidence_class: $lightrag_summary[0].evidence_class,
	        status_boundary: $lightrag_summary[0].status_boundary,
	        scored_benchmark: $lightrag_summary[0].scored_benchmark,
	        materialization: $lightrag_summary[0].materialization,
	        report: $lightrag_summary[0].report
      }
    ]' "${REPORT_DIR}/summary.json" >"${REPORT_DIR}/summary.json.tmp"
  mv "${REPORT_DIR}/summary.json.tmp" "${REPORT_DIR}/summary.json"
fi

if [[ -f "${REPORT_DIR}/graphrag/summary.json" ]]; then
  jq \
    --slurpfile graphrag_summary "${REPORT_DIR}/graphrag/summary.json" \
    '.adapters += [
      {
	        adapter_id: $graphrag_summary[0].adapter_id,
	        evidence_class: $graphrag_summary[0].evidence_class,
	        status_boundary: $graphrag_summary[0].status_boundary,
	        scored_benchmark: $graphrag_summary[0].scored_benchmark,
	        materialization: $graphrag_summary[0].materialization,
	        report: $graphrag_summary[0].report
      }
    ]' "${REPORT_DIR}/summary.json" >"${REPORT_DIR}/summary.json.tmp"
  mv "${REPORT_DIR}/summary.json.tmp" "${REPORT_DIR}/summary.json"
fi

if [[ -f "${REPORT_DIR}/graphiti-zep/summary.json" ]]; then
  jq \
    --slurpfile graphiti_summary "${REPORT_DIR}/graphiti-zep/summary.json" \
    '.adapters += [
      {
	        adapter_id: $graphiti_summary[0].adapter_id,
	        evidence_class: $graphiti_summary[0].evidence_class,
	        status_boundary: $graphiti_summary[0].status_boundary,
	        scored_benchmark: $graphiti_summary[0].scored_benchmark,
	        materialization: $graphiti_summary[0].materialization,
	        report: $graphiti_summary[0].report
      }
    ]' "${REPORT_DIR}/summary.json" >"${REPORT_DIR}/summary.json.tmp"
  mv "${REPORT_DIR}/summary.json.tmp" "${REPORT_DIR}/summary.json"
fi

if [[ -f "${REPORT_DIR}/graphify/summary.json" ]]; then
  jq \
    --slurpfile graphify_summary "${REPORT_DIR}/graphify/summary.json" \
    '.adapters += [
      {
	        adapter_id: $graphify_summary[0].adapter_id,
	        evidence_class: $graphify_summary[0].evidence_class,
	        status_boundary: $graphify_summary[0].status_boundary,
	        scored_benchmark: $graphify_summary[0].scored_benchmark,
	        materialization: $graphify_summary[0].materialization,
	        report: $graphify_summary[0].report
      }
    ]' "${REPORT_DIR}/summary.json" >"${REPORT_DIR}/summary.json.tmp"
  mv "${REPORT_DIR}/summary.json.tmp" "${REPORT_DIR}/summary.json"
fi

echo "Live real-world adapter reports:"
echo "  ${REPORT_DIR}/elf-report.json"
echo "  ${REPORT_DIR}/elf-report.md"
echo "  ${REPORT_DIR}/qmd-report.json"
echo "  ${REPORT_DIR}/qmd-report.md"
if [[ -f "${REPORT_DIR}/ragflow/summary.json" ]]; then
  echo "  ${REPORT_DIR}/ragflow/ragflow-report.json"
  echo "  ${REPORT_DIR}/ragflow/ragflow-report.md"
  echo "  ${REPORT_DIR}/ragflow/summary.json"
fi
if [[ -f "${REPORT_DIR}/lightrag/summary.json" ]]; then
  echo "  ${REPORT_DIR}/lightrag/lightrag-report.json"
  echo "  ${REPORT_DIR}/lightrag/lightrag-report.md"
  echo "  ${REPORT_DIR}/lightrag/summary.json"
fi
if [[ -f "${REPORT_DIR}/graphrag/summary.json" ]]; then
  echo "  ${REPORT_DIR}/graphrag/graphrag-report.json"
  echo "  ${REPORT_DIR}/graphrag/graphrag-report.md"
  echo "  ${REPORT_DIR}/graphrag/graphrag-smoke.json"
  echo "  ${REPORT_DIR}/graphrag/summary.json"
fi
if [[ -f "${REPORT_DIR}/graphiti-zep/summary.json" ]]; then
  echo "  ${REPORT_DIR}/graphiti-zep/graphiti-zep-report.json"
  echo "  ${REPORT_DIR}/graphiti-zep/graphiti-zep-report.md"
  echo "  ${REPORT_DIR}/graphiti-zep/graphiti-zep-smoke.json"
  echo "  ${REPORT_DIR}/graphiti-zep/summary.json"
fi
if [[ -f "${REPORT_DIR}/graphify/summary.json" ]]; then
  echo "  ${REPORT_DIR}/graphify/graphify-report.json"
  echo "  ${REPORT_DIR}/graphify/graphify-report.md"
  echo "  ${REPORT_DIR}/graphify/graphify-smoke.json"
  echo "  ${REPORT_DIR}/graphify/summary.json"
fi
echo "  ${REPORT_DIR}/summary.json"
