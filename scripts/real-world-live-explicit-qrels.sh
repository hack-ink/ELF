#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ELF_REAL_WORLD_LIVE_EXPLICIT_QRELS_REPORT_DIR:-${ROOT_DIR}/tmp/real-world-memory/live-explicit-qrels}"
SOURCE_FIXTURE_DIR="${ELF_REAL_WORLD_LIVE_EXPLICIT_QRELS_FIXTURES:-${ROOT_DIR}/apps/elf-eval/fixtures/real_world_memory}"
OPERATOR_SOURCE_FIXTURE_DIR="${ELF_REAL_WORLD_LIVE_EXPLICIT_QRELS_OPERATOR_DEBUG_FIXTURES:-${ROOT_DIR}/apps/elf-eval/fixtures/real_world_job/operator_debugging_ux}"
QREL_FIXTURE_DIR="${REPORT_DIR}/explicit-qrel-fixtures"
QREL_OPERATOR_FIXTURE_DIR="${REPORT_DIR}/explicit-qrel-operator-debug-fixtures"
LIVE_REPORT_DIR="${REPORT_DIR}/live-adapters"
LIVE_WORK_DIR="${ELF_REAL_WORLD_LIVE_EXPLICIT_QRELS_WORK_DIR:-/bench/real-world-live-explicit-qrels}"

if [[ ! -f "/.dockerenv" && "${ELF_REAL_WORLD_LIVE_ALLOW_HOST:-0}" != "1" ]]; then
  echo "Refusing to run live explicit-qrel adapters outside Docker. Use cargo make real-world-memory-live-explicit-qrels." >&2
  exit 1
fi

for cmd in bash jq python3; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing ${cmd} in live explicit-qrel runner." >&2
    exit 1
  fi
done

cd "${ROOT_DIR}"

rm -rf "${REPORT_DIR}"
mkdir -p "${REPORT_DIR}"

python3 scripts/materialize-explicit-qrels.py \
  --fixtures "${SOURCE_FIXTURE_DIR}" \
  --out-fixtures "${QREL_FIXTURE_DIR}" \
  --summary-out "${REPORT_DIR}/memory-materialization-summary.json" \
  --ranked-candidates-source none \
  --profile generated_public \
  --exclude-without-positive-qrels

python3 scripts/materialize-explicit-qrels.py \
  --fixtures "${OPERATOR_SOURCE_FIXTURE_DIR}" \
  --out-fixtures "${QREL_OPERATOR_FIXTURE_DIR}" \
  --summary-out "${REPORT_DIR}/operator-debug-materialization-summary.json" \
  --ranked-candidates-source none \
  --profile generated_public \
  --exclude-without-positive-qrels

ELF_REAL_WORLD_LIVE_REPORT_DIR="${LIVE_REPORT_DIR}" \
  ELF_REAL_WORLD_LIVE_FIXTURES="${QREL_FIXTURE_DIR}" \
  ELF_REAL_WORLD_OPERATOR_DEBUG_FIXTURES="${QREL_OPERATOR_FIXTURE_DIR}" \
  ELF_REAL_WORLD_LIVE_WORK_DIR="${LIVE_WORK_DIR}" \
  ELF_REAL_WORLD_LIVE_ELF_RUN_ID="real-world-memory-live-explicit-qrels-elf" \
  ELF_REAL_WORLD_LIVE_QMD_RUN_ID="real-world-memory-live-explicit-qrels-qmd" \
  ELF_REAL_WORLD_LIVE_COMBINED_RUN_ID="real-world-memory-live-elf-qmd-explicit-qrels-quantitative" \
  bash scripts/real-world-live-adapters.sh

jq -n \
  --slurpfile memory_summary "${REPORT_DIR}/memory-materialization-summary.json" \
  --slurpfile operator_summary "${REPORT_DIR}/operator-debug-materialization-summary.json" \
  --slurpfile live_summary "${LIVE_REPORT_DIR}/summary.json" \
  '{
    schema: "elf.real_world_live_explicit_qrels_sweep/v1",
    generated_at: (now | todateiso8601),
    artifact_dir: (env.ELF_REAL_WORLD_LIVE_EXPLICIT_QRELS_REPORT_DIR // "tmp/real-world-memory/live-explicit-qrels"),
    live_report_dir: "tmp/real-world-memory/live-explicit-qrels/live-adapters",
    materialization: {
      memory: $memory_summary[0],
      operator_debugging_ux: $operator_summary[0]
    },
    live_summary: $live_summary[0],
    boundary: "Input fixtures have deterministic explicit qrels, but ranked candidates are product-runtime traces from the live adapters. This improves qrel-source evidence only; leaderboard claims still require pass rows, full ranked coverage, held-out/leakage audit evidence, and paired significance."
  }' >"${REPORT_DIR}/summary.json"

echo "Live explicit-qrel adapter reports:"
echo "  ${REPORT_DIR}/memory-materialization-summary.json"
echo "  ${REPORT_DIR}/operator-debug-materialization-summary.json"
echo "  ${LIVE_REPORT_DIR}/elf-report.json"
echo "  ${LIVE_REPORT_DIR}/qmd-report.json"
echo "  ${LIVE_REPORT_DIR}/qmd-quantitative-product-manifest.json"
echo "  ${LIVE_REPORT_DIR}/elf-qmd-quantitative-report.json"
echo "  ${LIVE_REPORT_DIR}/elf-qmd-quantitative-report.md"
echo "  ${REPORT_DIR}/summary.json"
