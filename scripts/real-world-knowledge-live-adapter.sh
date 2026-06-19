#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ELF_KNOWLEDGE_LIVE_REPORT_DIR:-${ROOT_DIR}/tmp/real-world-memory/live-knowledge}"
FIXTURE_DIR="${ELF_KNOWLEDGE_LIVE_FIXTURES:-${ROOT_DIR}/apps/elf-eval/fixtures/real_world_memory/knowledge}"

if [[ ! -f "/.dockerenv" && "${ELF_KNOWLEDGE_LIVE_ALLOW_HOST:-0}" != "1" ]]; then
  echo "Refusing to run live knowledge adapter outside Docker. Use cargo make real-world-memory-live-knowledge." >&2
  exit 1
fi

for cmd in bash cargo jq; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing ${cmd} in live knowledge runner." >&2
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
  --run-id real-world-memory-live-knowledge \
  --adapter-id elf_live_real_world \
  --adapter-name "ELF live knowledge-page service adapter" \
  --adapter-behavior live_real_world_adapter \
  --adapter-storage-status pass \
  --adapter-runtime-status pass \
  --adapter-notes "Materialized by real_world_live_adapter through ElfService knowledge_page_rebuild, knowledge_page_lint, and knowledge_pages_search across the encoded knowledge_compilation fixture pack; pages remain derived benchmark artifacts, not authoritative storage."

cargo run -p elf-eval --bin real_world_job_benchmark -- publish \
  --report "${REPORT_DIR}/elf-report.json" \
  --out "${REPORT_DIR}/elf-report.md"

jq -n \
  --slurpfile materialization "${REPORT_DIR}/elf-materialization.json" \
  --slurpfile report "${REPORT_DIR}/elf-report.json" \
  '{
    schema: "elf.real_world_knowledge_live_adapter_sweep/v1",
    generated_at: (now | todateiso8601),
    fixture_dir: (env.ELF_KNOWLEDGE_LIVE_FIXTURES // "apps/elf-eval/fixtures/real_world_memory/knowledge"),
    artifact_dir: (env.ELF_KNOWLEDGE_LIVE_REPORT_DIR // "tmp/real-world-memory/live-knowledge"),
    adapter: {
      adapter_id: "elf_live_real_world",
      evidence_class: "live_real_world",
      materialization: $materialization[0],
      report: {
        json: "tmp/real-world-memory/live-knowledge/elf-report.json",
        markdown: "tmp/real-world-memory/live-knowledge/elf-report.md",
        summary: $report[0].summary,
        suites: $report[0].suites
      }
    },
    comparison_boundary: {
      baseline: "fixture-backed knowledge_compilation pages plus graph/RAG representative typed non-pass coverage",
      judgment_rule: "improved only when service-native rebuild/lint/search materialization preserves citations, stale-source lint, unsupported-section flags, rebuild metadata, and source-of-truth boundaries",
      competitor_boundary: "llm-wiki, gbrain, GraphRAG, RAGFlow, LightRAG, and graphify remain separate comparison targets unless a contained adapter emits comparable source ids, page sections, citation mappings, lint findings, and typed statuses"
    }
  }' >"${REPORT_DIR}/summary.json"

echo "Live knowledge reports:"
echo "  ${REPORT_DIR}/elf-materialization.json"
echo "  ${REPORT_DIR}/elf-report.json"
echo "  ${REPORT_DIR}/elf-report.md"
echo "  ${REPORT_DIR}/summary.json"
