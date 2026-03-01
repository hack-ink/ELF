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
: "${ELF_QDRANT_HTTP_URL:?Set ELF_QDRANT_HTTP_URL to the Qdrant REST base URL, for example http://127.0.0.1:51889 (default: http://127.0.0.1:6333).}"

QDRANT_GRPC_URL="${ELF_QDRANT_GRPC_URL:-${ELF_QDRANT_URL:-}}"
if [[ -z "${QDRANT_GRPC_URL}" ]]; then
  echo "Set ELF_QDRANT_GRPC_URL to the Qdrant gRPC base URL, for example http://127.0.0.1:51890 (default: http://127.0.0.1:6334). Legacy alias ELF_QDRANT_URL is deprecated but still supported."
  exit 1
fi

if command -v jaq >/dev/null 2>&1; then
  JSON_TOOL="jaq"
elif command -v jq >/dev/null 2>&1; then
  JSON_TOOL="jq"
else
  echo "Missing jaq/jq. Install jaq (recommended) or jq." >&2
  exit 1
fi

for cmd in curl psql taplo; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing ${cmd}." >&2
    exit 1
  fi
done

RUN_ID="${ELF_HARNESS_RUN_ID:-"$(date +%s)-$$"}"

DB_NAME="${ELF_HARNESS_DB_NAME:-elf_consolidation}"
QDRANT_COLLECTION="${ELF_HARNESS_COLLECTION:-elf_harness_consolidation_${RUN_ID}}"
VECTOR_DIM="${ELF_HARNESS_VECTOR_DIM:-4096}"
TOP_K="${ELF_HARNESS_TOP_K:-3}"
CANDIDATE_K="${ELF_HARNESS_CANDIDATE_K:-30}"
TARGET_KEY="incident_merge_protocol"

if [[ ! "${DB_NAME}" =~ ^elf_ ]]; then
  echo "ELF_HARNESS_DB_NAME must start with elf_ to avoid deleting real data." >&2
  exit 1
fi
if [[ ! "${QDRANT_COLLECTION}" =~ ^elf_ ]]; then
  echo "ELF_HARNESS_COLLECTION must start with elf_ to avoid deleting real data." >&2
  exit 1
fi
if [[ ! "${VECTOR_DIM}" =~ ^[0-9]+$ ]] || [[ "${VECTOR_DIM}" -le 0 ]]; then
  echo "ELF_HARNESS_VECTOR_DIM must be a positive integer." >&2
  exit 1
fi

HTTP_BIND="${ELF_HARNESS_HTTP_BIND:-127.0.0.1:18389}"
ADMIN_BIND="${ELF_HARNESS_ADMIN_BIND:-127.0.0.1:18390}"
MCP_BIND="${ELF_HARNESS_MCP_BIND:-127.0.0.1:18391}"
HTTP_BASE="http://${HTTP_BIND}"

PG_DSN_BASE="${ELF_PG_DSN%/*}"
PG_DSN="${PG_DSN_BASE}/${DB_NAME}"

VECTOR_DIM_TOML="$(echo "${VECTOR_DIM}" | perl -pe '1 while s/^([0-9]+)([0-9]{3})/$1_$2/')"

CFG_BASE="${ROOT_DIR}/tmp/elf.consolidation.base.toml"
DATASET="${ROOT_DIR}/tmp/elf.consolidation.dataset.json"
OUT_BASE="${ROOT_DIR}/tmp/elf.consolidation.out.base.json"
OUT_AFTER="${ROOT_DIR}/tmp/elf.consolidation.out.after.json"
WORKER_LOG="${ROOT_DIR}/tmp/elf.consolidation.worker.log"
API_LOG="${ROOT_DIR}/tmp/elf.consolidation.api.log"

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

wait_for_outbox_done() {
  local note_id="$1"
  for _ in $(seq 1 120); do
    status="$(
      psql "${PG_DSN}" -tAc \
        "SELECT status FROM indexing_outbox WHERE note_id = '${note_id}' ORDER BY created_at DESC LIMIT 1;" \
        | tr -d '[:space:]'
    )"
    if [[ -z "${status}" ]] || [[ "${status}" == "DONE" ]]; then
      return 0
    fi
    sleep 0.5
  done
  return 1
}

run_eval() {
  local out_path="$1"
  (cd "${ROOT_DIR}" && cargo run -q -p elf-eval -- --config "${CFG_BASE}" --dataset "${DATASET}") \
    | awk 'BEGIN { started = 0 } /^\{/ { started = 1 } { if (started) print }' \
    >"${out_path}"
}

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
url        = "${QDRANT_GRPC_URL}"
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
candidate_k             = ${CANDIDATE_K}
dup_sim_threshold       = 0.92
max_note_chars          = 240
max_notes_per_add_event = 3
top_k                   = ${TOP_K}
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
capture_candidates = false
candidate_retention_days = 2
write_mode = "outbox"

[ranking]
recency_tau_days   = 60
tie_breaker_weight = 0.1

[ranking.deterministic]
enabled = false

[ranking.deterministic.lexical]
enabled         = false
max_query_terms = 16
max_text_terms  = 1024
min_ratio       = 0.3
weight          = 0.05

[ranking.deterministic.hits]
enabled           = false
half_saturation   = 8.0
last_hit_tau_days = 14.0
weight            = 0.05

[ranking.deterministic.decay]
enabled  = false
tau_days = 30.0
weight   = 0.05

[ranking.blend]
enabled                 = true
rerank_normalization    = "rank"
retrieval_normalization = "rank"

[[ranking.blend.segments]]
max_retrieval_rank = 3
retrieval_weight   = 0.8

[[ranking.blend.segments]]
max_retrieval_rank = 10
retrieval_weight   = 0.2

[[ranking.blend.segments]]
max_retrieval_rank = 1_000_000
retrieval_weight   = 0.2

[ranking.diversity]
enabled       = true
max_skips     = 64
mmr_lambda    = 0.7
sim_threshold = 0.88

[ranking.retrieval_sources]
fusion_priority           = 1
fusion_weight             = 1.0
structured_field_priority = 0
structured_field_weight   = 1.0

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
auth_mode                = "off"
auth_keys                = []
bind_localhost_only      = true
evidence_max_quote_chars = 320
evidence_max_quotes      = 2
evidence_min_quotes      = 1
redact_secrets_on_write  = true
reject_non_english       = true
TOML

taplo fmt "${CFG_BASE}" >/dev/null 2>&1

echo "Building harness binaries."
(cd "${ROOT_DIR}" && cargo build -p elf-worker -p elf-api -p elf-eval >/dev/null)

echo "Starting worker and API (logs: ${WORKER_LOG}, ${API_LOG})."
(cd "${ROOT_DIR}" && "${ROOT_DIR}/target/debug/elf-worker" --config "${CFG_BASE}" >"${WORKER_LOG}" 2>&1) &
WORKER_PID="$!"
(cd "${ROOT_DIR}" && "${ROOT_DIR}/target/debug/elf-api" --config "${CFG_BASE}" >"${API_LOG}" 2>&1) &
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

TENANT_ID="consolidation-tenant-${RUN_ID}"
PROJECT_ID="consolidation-project-${RUN_ID}"
AGENT_ID="consolidation-agent-${RUN_ID}"

echo "Ingesting duplicate policy notes (legacy/noisy) before consolidation."
DUP_NOTE_IDS_RAW="$(
  "${JSON_TOOL}" -n \
    --arg run "${RUN_ID}" \
    --arg key "${TARGET_KEY}" \
    --arg tenant "${TENANT_ID}" \
    --arg project "${PROJECT_ID}" \
    --arg agent "${AGENT_ID}" \
    '{
      tenant_id: $tenant,
      project_id: $project,
      agent_id: $agent,
      scope: "agent_private",
      notes: [
        {
          type: "fact",
          key: $key,
          text: "Incident merge protocol draft A: for every incident merge, consolidate duplicate notes with the same policy key and carry forward the newest canonical decision evidence.",
          importance: 0.95,
          confidence: 0.4,
          ttl_days: 180,
          source_ref: {run: $run, stage: "legacy-a"}
        },
        {
          type: "fact",
          key: $key,
          text: "Incident merge protocol draft B: consolidate duplicate incident notes, retain one canonical policy note, and remove stale duplicates after the merge checkpoint.",
          importance: 0.95,
          confidence: 0.4,
          ttl_days: 180,
          source_ref: {run: $run, stage: "legacy-b"}
        },
        {
          type: "fact",
          key: $key,
          text: "Incident merge protocol draft C: when duplicate memory notes exist for the same key, de-duplicate to one canonical incident policy and archive obsolete variants.",
          importance: 0.95,
          confidence: 0.4,
          ttl_days: 180,
          source_ref: {run: $run, stage: "legacy-c"}
        }
      ]
    }' \
  | curl -sS "${HTTP_BASE}/v2/notes/ingest" \
    -H 'content-type: application/json' \
    -H "X-ELF-Tenant-Id: ${TENANT_ID}" \
    -H "X-ELF-Project-Id: ${PROJECT_ID}" \
    -H "X-ELF-Agent-Id: ${AGENT_ID}" \
    -d @- \
    | "${JSON_TOOL}" -r '.results[].note_id'
)"

mapfile -t DUP_NOTE_IDS <<<"${DUP_NOTE_IDS_RAW}"

echo "Ingesting distractor notes."
DISTRACTOR_IDS_RAW="$(
  "${JSON_TOOL}" -n \
    --arg run "${RUN_ID}" \
    '{
      scope: "agent_private",
      notes: [range(1; 13) as $i | {
        type: "fact",
        key: ("distraction_" + ($i|tostring)),
        text: ("Unrelated backlog signal " + ($i|tostring) + "."),
        importance: 0.01,
        confidence: 0.5,
        ttl_days: 180,
        source_ref: {run: $run}
      }]
    }' \
  | curl -sS "${HTTP_BASE}/v2/notes/ingest" \
    -H 'content-type: application/json' \
    -H "X-ELF-Tenant-Id: ${TENANT_ID}" \
    -H "X-ELF-Project-Id: ${PROJECT_ID}" \
    -H "X-ELF-Agent-Id: ${AGENT_ID}" \
    -d @- \
    | "${JSON_TOOL}" -r '.results[].note_id'
)"

mapfile -t DISTRACTOR_IDS <<<"${DISTRACTOR_IDS_RAW}"

if [[ "${#DUP_NOTE_IDS[@]}" -lt 3 || "${#DISTRACTOR_IDS[@]}" -lt 8 ]]; then
  echo "Add-note failed. Check logs: ${API_LOG}." >&2
  exit 1
fi

echo "Waiting for indexing jobs to finish."
for id in "${DUP_NOTE_IDS[@]}" "${DISTRACTOR_IDS[@]}"; do
  if ! wait_for_outbox_done "${id}"; then
    echo "Timed out waiting for indexing. Check logs: ${WORKER_LOG}." >&2
    exit 1
  fi
done

cat >"${DATASET}" <<JSON
{
  "name": "incident-consolidation-harness",
  "defaults": {
    "tenant_id": "${TENANT_ID}",
    "project_id": "${PROJECT_ID}",
    "agent_id": "${AGENT_ID}",
    "read_profile": "all_scopes",
    "top_k": ${TOP_K},
    "candidate_k": ${CANDIDATE_K}
  },
  "queries": [
    {
      "id": "q-1",
      "query": "How do we consolidate duplicate incident notes into one canonical policy?",
      "expected_keys": ["${TARGET_KEY}"]
    }
  ]
}
JSON

run_eval "${OUT_BASE}"

BASE_RECALL="$("${JSON_TOOL}" -r '.summary.avg_recall_at_k' "${OUT_BASE}")"
BASE_CONTEXT="$("${JSON_TOOL}" -r '.summary.avg_retrieved_summary_chars' "${OUT_BASE}")"
BASE_KEYS="$("${JSON_TOOL}" -r '.queries[0].retrieved_keys | map(. // "") | join(",")' "${OUT_BASE}")"

echo "Consolidation step: deleting duplicate legacy notes and adding a canonical entry."
for id in "${DUP_NOTE_IDS[@]}"; do
  curl -sS -X DELETE "${HTTP_BASE}/v2/notes/${id}" \
    -H "X-ELF-Tenant-Id: ${TENANT_ID}" \
    -H "X-ELF-Project-Id: ${PROJECT_ID}" \
    -H "X-ELF-Agent-Id: ${AGENT_ID}" \
    >/dev/null
  if ! wait_for_outbox_done "${id}"; then
    echo "Timed out waiting for duplicate note to de-index. Check logs: ${WORKER_LOG}." >&2
    exit 1
  fi
done

STABLE_NOTE_ID="$(
  curl -sS "${HTTP_BASE}/v2/notes/ingest" \
    -H 'content-type: application/json' \
    -H "X-ELF-Tenant-Id: ${TENANT_ID}" \
    -H "X-ELF-Project-Id: ${PROJECT_ID}" \
    -H "X-ELF-Agent-Id: ${AGENT_ID}" \
    -d "{
      \"scope\": \"agent_private\",
      \"notes\": [
        {
          \"type\": \"fact\",
          \"key\": \"${TARGET_KEY}\",
          \"text\": \"Canonical incident merge protocol: keep one note per policy key and remove duplicates after merge.\",
          \"importance\": 0.9,
          \"confidence\": 0.98,
          \"ttl_days\": 180,
          \"source_ref\": {\"run\": \"${RUN_ID}\", \"stage\": \"consolidated\"}
        }
      ]
    }" | "${JSON_TOOL}" -r '.results[0].note_id'
)"

if [[ -z "${STABLE_NOTE_ID}" || "${STABLE_NOTE_ID}" == "null" ]]; then
  echo "Failed to ingest consolidated note." >&2
  exit 1
fi

if ! wait_for_outbox_done "${STABLE_NOTE_ID}"; then
  echo "Timed out waiting for consolidated note to index. Check logs: ${WORKER_LOG}." >&2
  exit 1
fi

run_eval "${OUT_AFTER}"

AFTER_RECALL="$("${JSON_TOOL}" -r '.summary.avg_recall_at_k' "${OUT_AFTER}")"
AFTER_CONTEXT="$("${JSON_TOOL}" -r '.summary.avg_retrieved_summary_chars' "${OUT_AFTER}")"
AFTER_KEYS="$("${JSON_TOOL}" -r '.queries[0].retrieved_keys | map(. // "") | join(",")' "${OUT_AFTER}")"

echo "Consolidation results:"
echo "baseline  recall@${TOP_K}=${BASE_RECALL} avg_retrieved_summary_chars=${BASE_CONTEXT}"
echo "baseline  top_keys=${BASE_KEYS}"
echo "after     recall@${TOP_K}=${AFTER_RECALL} avg_retrieved_summary_chars=${AFTER_CONTEXT}"
echo "after     top_keys=${AFTER_KEYS}"

if [[ "${AFTER_KEYS}" != *"${TARGET_KEY}"* ]]; then
  echo "Expected consolidated key ${TARGET_KEY} to remain retrievable after consolidation." >&2
  exit 1
fi

awk -v after="${AFTER_RECALL}" -v base="${BASE_RECALL}" 'BEGIN { exit !(after + 1e-9 >= base) }' || {
  echo "Expected recall to be preserved or improved after consolidation." >&2
  exit 1
}

awk -v after="${AFTER_CONTEXT}" -v base="${BASE_CONTEXT}" 'BEGIN { exit !(after <= base + 1e-9) }' || {
  echo "Expected avg_retrieved_summary_chars to decrease or stay flat after consolidation." >&2
  exit 1
}
