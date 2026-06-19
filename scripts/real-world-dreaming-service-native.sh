#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ELF_DREAMING_SERVICE_NATIVE_REPORT_DIR:-${ROOT_DIR}/tmp/real-world-memory/service-native-dreaming}"
FIXTURE_ROOT="${ELF_DREAMING_SERVICE_NATIVE_FIXTURES:-${ROOT_DIR}/apps/elf-eval/fixtures/real_world_memory}"
INPUT_FIXTURE_DIR="${REPORT_DIR}/input-fixtures"

if [[ ! -f "/.dockerenv" && "${ELF_DREAMING_SERVICE_NATIVE_ALLOW_HOST:-0}" != "1" ]]; then
  echo "Refusing to run service-native Dreaming readback outside Docker. Use cargo make real-world-memory-service-native-dreaming." >&2
  exit 1
fi

for cmd in bash cargo jq; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing ${cmd} in service-native Dreaming readback runner." >&2
    exit 1
  fi
done

mkdir -p "${REPORT_DIR}"
rm -rf "${INPUT_FIXTURE_DIR}" \
  "${REPORT_DIR:?}/elf-fixtures" \
  "${REPORT_DIR:?}/elf-materialization.json" \
  "${REPORT_DIR:?}/report.json" \
  "${REPORT_DIR:?}/report.md" \
  "${REPORT_DIR:?}/summary.json"

mkdir -p "${INPUT_FIXTURE_DIR}"
cp -R "${FIXTURE_ROOT}/memory_summary" "${INPUT_FIXTURE_DIR}/memory_summary"
cp -R "${FIXTURE_ROOT}/proactive_brief" "${INPUT_FIXTURE_DIR}/proactive_brief"
cp -R "${FIXTURE_ROOT}/scheduled_memory" "${INPUT_FIXTURE_DIR}/scheduled_memory"

cd "${ROOT_DIR}"

cargo run -p elf-eval --bin real_world_live_adapter -- elf \
  --fixtures "${INPUT_FIXTURE_DIR}" \
  --out-fixtures "${REPORT_DIR}/elf-fixtures" \
  --evidence-out "${REPORT_DIR}/elf-materialization.json" \
  --config config/local/elf.docker.toml \
  --adapter-id elf_service_native_dreaming

cargo run -p elf-eval --bin real_world_job_benchmark -- run \
  --fixtures "${REPORT_DIR}/elf-fixtures" \
  --out "${REPORT_DIR}/report.json" \
  --run-id real-world-memory-service-native-dreaming \
  --adapter-id elf_service_native_dreaming \
  --adapter-name "ELF service-native Dreaming readback adapter" \
  --adapter-behavior service_native_dreaming_readback \
  --adapter-storage-status pass \
  --adapter-runtime-status pass \
  --adapter-notes "Materialized through ElfService add_note/list/search readback for memory_summary, proactive_brief, and scheduled_memory fixtures. Private/provider blockers remain typed non-pass records under XY-930."

cargo run -p elf-eval --bin real_world_job_benchmark -- publish \
  --report "${REPORT_DIR}/report.json" \
  --out "${REPORT_DIR}/report.md"

jq -n \
  --slurpfile materialization "${REPORT_DIR}/elf-materialization.json" \
  --slurpfile report "${REPORT_DIR}/report.json" \
  '{
    schema: "elf.service_native_dreaming_readback_sweep/v1",
    generated_at: (now | todateiso8601),
    fixture_dir: (env.ELF_DREAMING_SERVICE_NATIVE_FIXTURES // "apps/elf-eval/fixtures/real_world_memory"),
    artifact_dir: (env.ELF_DREAMING_SERVICE_NATIVE_REPORT_DIR // "tmp/real-world-memory/service-native-dreaming"),
    adapter: {
      adapter_id: "elf_service_native_dreaming",
      evidence_class: "service_native_readback",
      materialization: $materialization[0],
      report: {
        json: "tmp/real-world-memory/service-native-dreaming/report.json",
        markdown: "tmp/real-world-memory/service-native-dreaming/report.md",
        summary: $report[0].summary,
        suites: $report[0].suites
      }
    },
    comparison_boundary: {
      baseline: "XY-955 fixture-backed Dreaming outputs",
      judgment_rule: "improved only when service-native readback scores source-linked artifacts without stale, tombstoned, unsupported, untraced, or source-mutation violations",
      private_provider_boundary: "XY-930 remains blocked unless operator-owned manifest and explicit provider setup exist"
    }
  }' >"${REPORT_DIR}/summary.json"

echo "Service-native Dreaming readback reports:"
echo "  ${REPORT_DIR}/elf-materialization.json"
echo "  ${REPORT_DIR}/report.json"
echo "  ${REPORT_DIR}/report.md"
echo "  ${REPORT_DIR}/summary.json"
