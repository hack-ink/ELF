#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ELF_CONSOLIDATION_LIVE_REPORT_DIR:-${ROOT_DIR}/tmp/real-world-memory/live-consolidation}"
FIXTURE_DIR="${ELF_CONSOLIDATION_LIVE_FIXTURES:-${ROOT_DIR}/apps/elf-eval/fixtures/real_world_memory/consolidation}"

if [[ ! -f "/.dockerenv" && "${ELF_CONSOLIDATION_LIVE_ALLOW_HOST:-0}" != "1" ]]; then
  echo "Refusing to run live consolidation adapter outside Docker. Use cargo make real-world-memory-live-consolidation." >&2
  exit 1
fi

for cmd in bash cargo jq; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing ${cmd} in live consolidation runner." >&2
    exit 1
  fi
done

mkdir -p "${REPORT_DIR}"
rm -rf "${REPORT_DIR:?}/elf-fixtures" \
  "${REPORT_DIR:?}/elf-materialization.json" \
  "${REPORT_DIR:?}/elf-report.json" \
  "${REPORT_DIR:?}/elf-report.md" \
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
  --run-id real-world-memory-live-consolidation \
  --adapter-id elf_live_real_world \
  --adapter-name "ELF live consolidation service adapter" \
  --adapter-behavior live_real_world_adapter \
  --adapter-storage-status pass \
  --adapter-runtime-status pass \
  --adapter-notes "Materialized by real_world_live_adapter through ElfService consolidation_run_create, worker proposal materialization, and apply/defer/discard review audit transitions; source notes remain immutable derived-output evidence."

cargo run -p elf-eval --bin real_world_job_benchmark -- publish \
  --report "${REPORT_DIR}/elf-report.json" \
  --out "${REPORT_DIR}/elf-report.md"

jq -n \
  --slurpfile materialization "${REPORT_DIR}/elf-materialization.json" \
  --slurpfile report "${REPORT_DIR}/elf-report.json" \
  '{
    schema: "elf.real_world_consolidation_live_adapter_sweep/v1",
    generated_at: (now | todateiso8601),
    fixture_dir: (env.ELF_CONSOLIDATION_LIVE_FIXTURES // "apps/elf-eval/fixtures/real_world_memory/consolidation"),
    artifact_dir: (env.ELF_CONSOLIDATION_LIVE_REPORT_DIR // "tmp/real-world-memory/live-consolidation"),
    adapter: {
      adapter_id: "elf_live_real_world",
      evidence_class: "live_real_world",
      materialization: $materialization[0],
      report: {
        json: "tmp/real-world-memory/live-consolidation/elf-report.json",
        markdown: "tmp/real-world-memory/live-consolidation/elf-report.md",
        summary: $report[0].summary,
        suites: $report[0].suites
      }
    }
  }' >"${REPORT_DIR}/summary.json"
