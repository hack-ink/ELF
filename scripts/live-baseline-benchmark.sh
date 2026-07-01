#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ELF_BASELINE_REPORT_DIR:-${ROOT_DIR}/tmp/live-baseline}"
WORK_DIR="${ELF_BASELINE_WORK_DIR:-/bench}"
REPOS_DIR="${WORK_DIR}/repos"
CORPUS_DIR="${WORK_DIR}/corpus"
HOME_DIR="${WORK_DIR}/home"
RECORDS="${REPORT_DIR}/project-records.jsonl"
REPORT="${REPORT_DIR}/live-baseline-report.json"
RUN_ID="${ELF_BASELINE_RUN_ID:-live-baseline-$(date +%Y%m%d%H%M%S)}"
PROJECT_FILTER="${ELF_BASELINE_PROJECTS:-all}"
CORPUS_PROFILE="${ELF_BASELINE_PROFILE:-smoke}"
SCALE_DOC_COUNT="${ELF_BASELINE_SCALE_DOCS:-120}"
STRESS_DOC_COUNT="${ELF_BASELINE_STRESS_DOCS:-480}"
BACKFILL_DOC_COUNT="${ELF_BASELINE_BACKFILL_DOCS:-2000}"
QUERY_TOP_K="${ELF_BASELINE_TOP_K:-10}"
CURRENT_PROJECT_STARTED_AT=""
PRODUCTION_SYNTHETIC_MANIFEST="${ROOT_DIR}/apps/elf-eval/fixtures/production_corpus/synthetic_coding_agent_manifest.json"
CORPUS_TRACK="generated_public"
CORPUS_PATH_DESCRIPTION="generated in Docker under /bench/corpus"
CORPUS_MANIFEST_ID=""

LIVE_BASELINE_LIB_DIR="${ROOT_DIR}/scripts/live-baseline"

source "${LIVE_BASELINE_LIB_DIR}/core.sh"
source "${LIVE_BASELINE_LIB_DIR}/openmemory.sh"
source "${LIVE_BASELINE_LIB_DIR}/corpus.sh"
source "${LIVE_BASELINE_LIB_DIR}/report.sh"
source "${LIVE_BASELINE_LIB_DIR}/runner.sh"
source "${LIVE_BASELINE_LIB_DIR}/projects/elf.sh"
source "${LIVE_BASELINE_LIB_DIR}/projects/agentmemory.sh"
source "${LIVE_BASELINE_LIB_DIR}/projects/qmd.sh"
source "${LIVE_BASELINE_LIB_DIR}/projects/memsearch.sh"
source "${LIVE_BASELINE_LIB_DIR}/projects/mem0.sh"
source "${LIVE_BASELINE_LIB_DIR}/projects/openviking.sh"
source "${LIVE_BASELINE_LIB_DIR}/projects/claude_mem.sh"

if [[ ! -f "/.dockerenv" && "${ELF_BASELINE_ALLOW_HOST:-0}" != "1" ]]; then
  echo "Refusing to run live baseline benchmark outside Docker. Use cargo make baseline-live-docker." >&2
  exit 1
fi

for cmd in bash cargo git jq node npm python3 rg timeout; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing ${cmd} in baseline runner." >&2
    exit 1
  fi
done
rm -rf "${WORK_DIR}"
mkdir -p "${REPORT_DIR}"
find "${REPORT_DIR}" -maxdepth 1 -type f -delete
mkdir -p "${REPOS_DIR}" "${CORPUS_DIR}" "${HOME_DIR}"
: >"${RECORDS}"

case "${CORPUS_PROFILE}" in
  production-synthetic | production-private)
    prepare_production_corpus
    ;;
  *)
    generate_corpus
    ;;
esac
DOCUMENT_COUNT="$(find "${CORPUS_DIR}" -maxdepth 1 -type f -name '*.md' | wc -l | tr -d ' ')"
QUERY_COUNT="$(jq '.queries | length' "${REPORT_DIR}/queries.json")"
run_project "ELF" project_elf
run_project "agentmemory" project_agentmemory
run_project "qmd" project_qmd
run_project "memsearch" project_memsearch
run_project "mem0" project_mem0
run_project "OpenViking" project_openviking
run_project "claude-mem" project_claude_mem
finish_report

jq . "${REPORT}"
echo "Live baseline report: ${REPORT}"

if [[ "${ELF_BASELINE_STRICT:-0}" == "1" ]]; then
  jq -e '.verdict == "pass"' "${REPORT}" >/dev/null
fi
