#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -f "${ROOT_DIR}/.env" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "${ROOT_DIR}/.env"
  set +a
fi

: "${ELF_PG_DSN:?Set ELF_PG_DSN to a Postgres DSN (usually .../postgres).}"
: "${ELF_QDRANT_URL:?Set ELF_QDRANT_URL to the Qdrant gRPC base URL, for example http://127.0.0.1:51890 (default: http://127.0.0.1:6334).}"
: "${ELF_QDRANT_HTTP_URL:?Set ELF_QDRANT_HTTP_URL to the Qdrant REST base URL, for example http://127.0.0.1:51889 (default: http://127.0.0.1:6333).}"

if command -v jaq >/dev/null 2>&1; then
  JSON_TOOL="jaq"
elif command -v jq >/dev/null 2>&1; then
  JSON_TOOL="jq"
else
  echo "Missing jaq/jq. Install jaq (recommended) or jq." >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "Missing curl." >&2
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "Missing psql." >&2
  exit 1
fi

if ! command -v taplo >/dev/null 2>&1; then
  echo "Missing taplo." >&2
  exit 1
fi

RUN_ID="${ELF_HARNESS_RUN_ID:-"$(date +%s)-$$"}"

DB_NAME="${ELF_HARNESS_DB_NAME:-elf_e2e}"
QDRANT_COLLECTION="${ELF_HARNESS_COLLECTION:-elf_harness_${RUN_ID}}"
VECTOR_DIM="${ELF_HARNESS_VECTOR_DIM:-4096}"

if [[ ! "${VECTOR_DIM}" =~ ^[0-9]+$ ]]; then
  echo "ELF_HARNESS_VECTOR_DIM must be an integer." >&2
  exit 1
fi

# Keep VECTOR_DIM numeric for JSON and SQL usage; use an underscore-formatted variant for TOML.
VECTOR_DIM_TOML="$(echo "${VECTOR_DIM}" | perl -pe '1 while s/^([0-9]+)([0-9]{3})/$1_$2/')"

if [[ "${DB_NAME}" != elf_* ]]; then
  echo "ELF_HARNESS_DB_NAME must start with elf_ to avoid deleting real data." >&2
  exit 1
fi

HTTP_BIND="${ELF_HARNESS_HTTP_BIND:-127.0.0.1:18089}"
ADMIN_BIND="${ELF_HARNESS_ADMIN_BIND:-127.0.0.1:18090}"
MCP_BIND="${ELF_HARNESS_MCP_BIND:-127.0.0.1:18091}"

HTTP_BASE="http://${HTTP_BIND}"

PG_DSN_BASE="${ELF_PG_DSN%/*}"
PG_DSN="${PG_DSN_BASE}/${DB_NAME}"

CFG_BASE="${ROOT_DIR}/tmp/elf.harness.base.toml"
CFG_CONTEXT="${ROOT_DIR}/tmp/elf.harness.context.toml"
DATASET="${ROOT_DIR}/tmp/elf.harness.dataset.json"
OUT_BASE="${ROOT_DIR}/tmp/elf.harness.out.base.json"
OUT_CONTEXT="${ROOT_DIR}/tmp/elf.harness.out.context.json"
WORKER_LOG="${ROOT_DIR}/tmp/elf.harness.worker.log"
API_LOG="${ROOT_DIR}/tmp/elf.harness.api.log"

if [[ "${QDRANT_COLLECTION}" != elf_harness_* ]]; then
  echo "ELF_HARNESS_COLLECTION must start with elf_harness_ to avoid deleting real data." >&2
  exit 1
fi

WORKER_PID=""
API_PID=""

cleanup() {
  set +e

  if [[ -n "${API_PID}" ]] && kill -0 "${API_PID}" >/dev/null 2>&1; then
    kill "${API_PID}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${WORKER_PID}" ]] && kill -0 "${WORKER_PID}" >/dev/null 2>&1; then
    kill "${WORKER_PID}" >/dev/null 2>&1 || true
  fi
  wait >/dev/null 2>&1 || true

  if [[ "${ELF_HARNESS_KEEP_COLLECTION:-0}" != "1" ]]; then
    curl -sS -X DELETE "${ELF_QDRANT_HTTP_URL}/collections/${QDRANT_COLLECTION}?wait=true" >/dev/null || true
  fi

  if [[ "${ELF_HARNESS_KEEP_DB:-0}" != "1" ]]; then
    psql "${ELF_PG_DSN}" -tAc \
      "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '${DB_NAME}' AND pid <> pg_backend_pid();" \
      >/dev/null 2>&1 || true
    psql "${ELF_PG_DSN}" -v ON_ERROR_STOP=1 -c "DROP DATABASE IF EXISTS ${DB_NAME};" >/dev/null 2>&1 || true
  fi
}

trap cleanup EXIT

echo "Recreating database ${DB_NAME}."
psql "${ELF_PG_DSN}" -v ON_ERROR_STOP=1 -c "DROP DATABASE IF EXISTS ${DB_NAME};" >/dev/null
psql "${ELF_PG_DSN}" -v ON_ERROR_STOP=1 -c "CREATE DATABASE ${DB_NAME};" >/dev/null

echo "Recreating Qdrant collection ${QDRANT_COLLECTION}."
curl -sS -X DELETE "${ELF_QDRANT_HTTP_URL}/collections/${QDRANT_COLLECTION}?wait=true" >/dev/null || true
(cd "${ROOT_DIR}" && ELF_QDRANT_COLLECTION="${QDRANT_COLLECTION}" ELF_QDRANT_VECTOR_DIM="${VECTOR_DIM}" ./qdrant/init.sh >/dev/null)

cat >"${CFG_BASE}" <<TOML
[service]
admin_bind = "${ADMIN_BIND}"
http_bind  = "${HTTP_BIND}"
log_level  = "info"
mcp_bind   = "${MCP_BIND}"

[storage.postgres]
dsn            = "${PG_DSN}"
pool_max_conns = 10

[storage.qdrant]
collection = "${QDRANT_COLLECTION}"
url        = "${ELF_QDRANT_URL}"
vector_dim = ${VECTOR_DIM_TOML}

[providers.embedding]
api_base    = "http://127.0.0.1"
api_key     = "local"
dimensions  = ${VECTOR_DIM_TOML}
model       = "local-hash"
path        = "/embeddings"
provider_id = "local"
timeout_ms  = 1_000

default_headers = {}

[providers.rerank]
api_base    = "http://127.0.0.1"
api_key     = "local"
model       = "local-token-overlap"
path        = "/rerank"
provider_id = "local"
timeout_ms  = 1_000

default_headers = {}

[providers.llm_extractor]
api_base      = "http://127.0.0.1"
api_key       = "local"
model         = "local-disabled"
path          = "/chat/completions"
provider_id   = "local"
temperature   = 0.0
timeout_ms    = 1_000

default_headers = {}

[scopes]
allowed = ["agent_private", "org_shared", "project_shared"]

[scopes.read_profiles]
all_scopes           = ["agent_private", "org_shared", "project_shared"]
private_only         = ["agent_private"]
private_plus_project = ["agent_private", "project_shared"]

[scopes.precedence]
agent_private  = 30
org_shared     = 10
project_shared = 20

[scopes.write_allowed]
agent_private  = true
org_shared     = true
project_shared = true

[memory]
candidate_k             = 60
dup_sim_threshold       = 0.92
max_note_chars          = 240
max_notes_per_add_event = 3
top_k                   = 12
update_sim_threshold    = 0.85

[chunking]
enabled        = true
max_tokens     = 512
overlap_tokens = 128
tokenizer_repo = "gpt2"

[search.expansion]
include_original = true
max_queries      = 4
mode             = "off"

[search.dynamic]
min_candidates = 10
min_top_score  = 0.12

[search.prefilter]
max_candidates = 0

[search.cache]
enabled           = false
expansion_ttl_days = 7
rerank_ttl_days    = 7

[search.explain]
retention_days = 7

[ranking]
recency_tau_days   = 60
tie_breaker_weight = 0.1

[lifecycle.ttl_days]
constraint = 0
decision   = 0
fact       = 180
plan       = 14
preference = 0
profile    = 0

[lifecycle]
purge_deleted_after_days    = 30
purge_deprecated_after_days = 180

[security]
bind_localhost_only      = true
evidence_max_quote_chars = 320
evidence_max_quotes      = 2
evidence_min_quotes      = 1
redact_secrets_on_write  = true
reject_cjk               = true
TOML

cp "${CFG_BASE}" "${CFG_CONTEXT}"
cat >>"${CFG_CONTEXT}" <<'TOML'

[context]
scope_boost_weight = 0.1

[context.scope_descriptions]
org_shared     = "Org-wide policies and shared operating context."
project_shared = "Project-specific deployment steps and runbooks."
TOML

taplo fmt "${CFG_BASE}" "${CFG_CONTEXT}" >/dev/null 2>&1

echo "Starting worker and API (logs: ${WORKER_LOG}, ${API_LOG})."
(cd "${ROOT_DIR}" && cargo run -p elf-worker -- --config "${CFG_BASE}" >"${WORKER_LOG}" 2>&1) &
WORKER_PID="$!"
(cd "${ROOT_DIR}" && cargo run -p elf-api -- --config "${CFG_BASE}" >"${API_LOG}" 2>&1) &
API_PID="$!"

echo "Waiting for API health check at ${HTTP_BASE}/health."
for _ in $(seq 1 120); do
  status="$(curl -s -o /dev/null -w '%{http_code}' "${HTTP_BASE}/health" 2>/dev/null || true)"
  if [[ "${status}" == "200" ]]; then
    break
  fi
  sleep 0.5
done

status="$(curl -s -o /dev/null -w '%{http_code}' "${HTTP_BASE}/health" 2>/dev/null || true)"
if [[ "${status}" != "200" ]]; then
  echo "API did not become healthy in time. Check logs: ${API_LOG}." >&2
  exit 1
fi
TENANT_ID="harness-tenant-${RUN_ID}"
PROJECT_ID="harness-project-${RUN_ID}"
AGENT_ID="harness-agent-${RUN_ID}"

echo "Adding confuser notes in org_shared and project_shared."
NOTE_ORG="$(
  curl -sS "${HTTP_BASE}/v2/notes/ingest" \
    -H 'content-type: application/json' \
    -H "X-ELF-Tenant-Id: ${TENANT_ID}" \
    -H "X-ELF-Project-Id: ${PROJECT_ID}" \
    -H "X-ELF-Agent-Id: ${AGENT_ID}" \
    -d "{
      \"scope\": \"org_shared\",
      \"notes\": [
        {
          \"type\": \"fact\",
          \"key\": \"deployment_steps\",
          \"text\": \"Deployment steps for service.\",
          \"importance\": 0.9,
          \"confidence\": 0.9,
          \"ttl_days\": 180,
          \"source_ref\": {\"run\": \"context-harness\"}
        }
      ]
    }" | "${JSON_TOOL}" -r '.results[0].note_id'
)"

NOTE_PROJECT="$(
  curl -sS "${HTTP_BASE}/v2/notes/ingest" \
    -H 'content-type: application/json' \
    -H "X-ELF-Tenant-Id: ${TENANT_ID}" \
    -H "X-ELF-Project-Id: ${PROJECT_ID}" \
    -H "X-ELF-Agent-Id: ${AGENT_ID}" \
    -d "{
      \"scope\": \"project_shared\",
      \"notes\": [
        {
          \"type\": \"fact\",
          \"key\": \"deployment_steps\",
          \"text\": \"Deployment steps for service.\",
          \"importance\": 0.0,
          \"confidence\": 0.9,
          \"ttl_days\": 180,
          \"source_ref\": {\"run\": \"context-harness\"}
        }
      ]
    }" | "${JSON_TOOL}" -r '.results[0].note_id'
)"

if [[ "${NOTE_ORG}" == "null" ]] || [[ "${NOTE_PROJECT}" == "null" ]]; then
  echo "Add-note failed. Check logs: ${API_LOG}." >&2
  exit 1
fi

wait_for_outbox_done() {
  local note_id="$1"
  for _ in $(seq 1 120); do
    status="$(
      psql "${PG_DSN}" -tAc \
        "SELECT status FROM indexing_outbox WHERE note_id = '${note_id}' ORDER BY created_at DESC LIMIT 1;" \
        | tr -d '[:space:]'
    )"
    if [[ "${status}" == "DONE" ]]; then
      return 0
    fi
    sleep 0.5
  done
  return 1
}

echo "Waiting for indexing jobs to finish."
if ! wait_for_outbox_done "${NOTE_ORG}"; then
  echo "Timed out waiting for org_shared note to index. Check logs: ${WORKER_LOG}." >&2
  exit 1
fi
if ! wait_for_outbox_done "${NOTE_PROJECT}"; then
  echo "Timed out waiting for project_shared note to index. Check logs: ${WORKER_LOG}." >&2
  exit 1
fi

cat >"${DATASET}" <<JSON
{
  "name": "context-misranking",
  "defaults": {
    "tenant_id": "${TENANT_ID}",
    "project_id": "${PROJECT_ID}",
    "agent_id": "${AGENT_ID}",
    "read_profile": "all_scopes",
    "top_k": 1,
    "candidate_k": 60
  },
  "queries": [
    {
      "id": "q-1",
      "query": "deployment steps",
      "expected_note_ids": ["${NOTE_PROJECT}"]
    }
  ]
}
JSON

run_eval() {
  local cfg_path="$1"
  local out_path="$2"
  (cd "${ROOT_DIR}" && cargo run -q -p elf-eval -- --config "${cfg_path}" --dataset "${DATASET}") \
    | awk 'BEGIN { started = 0 } /^\{/ { started = 1 } { if (started) print }' \
    >"${out_path}"
}

echo "Running baseline eval (no context)."
run_eval "${CFG_BASE}" "${OUT_BASE}"

echo "Running context eval (scope boost enabled)."
run_eval "${CFG_CONTEXT}" "${OUT_CONTEXT}"

RECALL_BASE="$("${JSON_TOOL}" -r '.summary.avg_recall_at_k' "${OUT_BASE}")"
TOP_BASE="$("${JSON_TOOL}" -r '.queries[0].retrieved_note_ids[0]' "${OUT_BASE}")"
RECALL_CONTEXT="$("${JSON_TOOL}" -r '.summary.avg_recall_at_k' "${OUT_CONTEXT}")"
TOP_CONTEXT="$("${JSON_TOOL}" -r '.queries[0].retrieved_note_ids[0]' "${OUT_CONTEXT}")"

echo "Results:"
echo "baseline  recall@1=${RECALL_BASE} top_note_id=${TOP_BASE}"
echo "context   recall@1=${RECALL_CONTEXT} top_note_id=${TOP_CONTEXT}"
echo "expected  note_id=${NOTE_PROJECT}"

echo "Cleaning up notes."
for id in "${NOTE_ORG}" "${NOTE_PROJECT}"; do
  curl -sS -X DELETE "${HTTP_BASE}/v2/notes/${id}" \
    -H "X-ELF-Tenant-Id: ${TENANT_ID}" \
    -H "X-ELF-Project-Id: ${PROJECT_ID}" \
    -H "X-ELF-Agent-Id: ${AGENT_ID}" \
    >/dev/null
done
