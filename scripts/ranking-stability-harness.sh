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

for cmd in curl psql taplo; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing ${cmd}." >&2
    exit 1
  fi
done

RUN_ID="${ELF_HARNESS_RUN_ID:-"$(date +%s)-$$"}"

DB_NAME="${ELF_HARNESS_DB_NAME:-elf_stability}"
QDRANT_COLLECTION="${ELF_HARNESS_COLLECTION:-elf_stability_${RUN_ID}}"
VECTOR_DIM="${ELF_HARNESS_VECTOR_DIM:-4096}"

NOISE_STD="${ELF_HARNESS_NOISE_STD:-0.08}"
RUNS_PER_QUERY="${ELF_HARNESS_RUNS_PER_QUERY:-8}"
TOP_K="${ELF_HARNESS_TOP_K:-10}"
CANDIDATE_K="${ELF_HARNESS_CANDIDATE_K:-60}"
TARGET_TOP_K="${ELF_HARNESS_TARGET_TOP_K:-10}"

if [[ "${DB_NAME}" != elf_* ]]; then
  echo "ELF_HARNESS_DB_NAME must start with elf_ to avoid deleting real data." >&2
  exit 1
fi
if [[ "${QDRANT_COLLECTION}" != elf_* ]]; then
  echo "ELF_HARNESS_COLLECTION must start with elf_ to avoid deleting real data." >&2
  exit 1
fi

HTTP_BIND="${ELF_HARNESS_HTTP_BIND:-127.0.0.1:18189}"
ADMIN_BIND="${ELF_HARNESS_ADMIN_BIND:-127.0.0.1:18190}"
MCP_BIND="${ELF_HARNESS_MCP_BIND:-127.0.0.1:18191}"
HTTP_BASE="http://${HTTP_BIND}"

PG_DSN_BASE="${ELF_PG_DSN%/*}"
PG_DSN="${PG_DSN_BASE}/${DB_NAME}"

CFG_BASE="${ROOT_DIR}/tmp/elf.stability.base.toml"
CFG_DET="${ROOT_DIR}/tmp/elf.stability.det.toml"
DATASET="${ROOT_DIR}/tmp/elf.stability.dataset.json"
OUT_JSON="${ROOT_DIR}/tmp/elf.stability.out.json"
WORKER_LOG="${ROOT_DIR}/tmp/elf.stability.worker.log"
API_LOG="${ROOT_DIR}/tmp/elf.stability.api.log"

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

VECTOR_DIM_TOML="$(echo "${VECTOR_DIM}" | perl -pe '1 while s/^([0-9]+)([0-9]{3})/$1_$2/')"

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
model       = "local-token-overlap-noisy@${NOISE_STD}"
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
enabled            = false
expansion_ttl_days = 7
rerank_ttl_days    = 7

[search.explain]
retention_days = 2

[ranking]
recency_tau_days   = 0
tie_breaker_weight = 0.0

[ranking.blend]
enabled = false

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

cp "${CFG_BASE}" "${CFG_DET}"
cat >>"${CFG_DET}" <<TOML

[ranking.deterministic]
enabled = true

[ranking.deterministic.hits]
enabled         = true
weight          = 1.25
half_saturation = 1.0
last_hit_tau_days = 30.0
TOML

taplo fmt "${CFG_BASE}" "${CFG_DET}" >/dev/null 2>&1

echo "Building harness binaries."
(cd "${ROOT_DIR}" && cargo build -p elf-worker -p elf-api -p elf-eval >/dev/null)

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

TENANT_ID="stability-tenant-${RUN_ID}"
PROJECT_ID="stability-project-${RUN_ID}"
AGENT_ID="stability-agent-${RUN_ID}"

NOTE_PAYLOAD="$(
  "${JSON_TOOL}" -n --arg run "ranking-stability-harness" --arg scope "agent_private" --arg query "deployment steps" --argjson count "${CANDIDATE_K}" '{
    scope: $scope,
    notes: [range(1; $count + 1) as $i | {
      type: "fact",
      key: ("stability_" + ($i|tostring)),
      text: ("Deployment steps for service. " + $query + ". Candidate " + ($i|tostring) + "."),
      importance: 0.2,
      confidence: 0.9,
      ttl_days: 180,
      source_ref: {run: $run}
    }]
  }'
)"

echo "Ingesting ${CANDIDATE_K} notes."
NOTE_IDS_RAW="$(
  curl -sS "${HTTP_BASE}/v2/notes/ingest" \
    -H 'content-type: application/json' \
    -H "X-ELF-Tenant-Id: ${TENANT_ID}" \
    -H "X-ELF-Project-Id: ${PROJECT_ID}" \
    -H "X-ELF-Agent-Id: ${AGENT_ID}" \
    -d "${NOTE_PAYLOAD}" | "${JSON_TOOL}" -r '.results[].note_id'
)"
mapfile -t NOTE_IDS <<<"${NOTE_IDS_RAW}"

if [[ "${#NOTE_IDS[@]}" -lt 10 ]]; then
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
for id in "${NOTE_IDS[@]}"; do
  if ! wait_for_outbox_done "${id}"; then
    echo "Timed out waiting for note to index. Check logs: ${WORKER_LOG}." >&2
    exit 1
  fi
done

TARGET_IDS=("${NOTE_IDS[@]:0:${TARGET_TOP_K}}")

echo "Boosting hit_count for the first ${TARGET_TOP_K} notes to create a stable target set."
TARGET_LIST="$(
  printf "%s\n" "${TARGET_IDS[@]}" | "${JSON_TOOL}" -R -s -c 'split("\n")[:-1]'
)"
TARGET_ARRAY_SQL="{"
for id in "${TARGET_IDS[@]}"; do
  TARGET_ARRAY_SQL+="${id},"
done
TARGET_ARRAY_SQL="${TARGET_ARRAY_SQL%,}}"
psql "${PG_DSN}" -v ON_ERROR_STOP=1 -c \
  "UPDATE memory_notes SET hit_count = 100, last_hit_at = now() WHERE note_id = ANY ('${TARGET_ARRAY_SQL}'::uuid[]);" \
  >/dev/null

cat >"${DATASET}" <<JSON
{
  "name": "ranking-stability-harness",
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
      "query": "deployment steps",
      "expected_note_ids": ${TARGET_LIST}
    }
  ]
}
JSON

echo "Running eval compare (runs_per_query=${RUNS_PER_QUERY})."
(cd "${ROOT_DIR}" && cargo run -q -p elf-eval -- --config-a "${CFG_BASE}" --config-b "${CFG_DET}" --dataset "${DATASET}" --runs-per-query "${RUNS_PER_QUERY}") \
  | awk 'BEGIN { started = 0 } /^\{/ { started = 1 } { if (started) print }' \
  >"${OUT_JSON}"

SET_CHURN_A="$("${JSON_TOOL}" -r '.summary_a.stability.avg_set_churn_at_k' "${OUT_JSON}")"
SET_CHURN_B="$("${JSON_TOOL}" -r '.summary_b.stability.avg_set_churn_at_k' "${OUT_JSON}")"
POS_CHURN_A="$("${JSON_TOOL}" -r '.summary_a.stability.avg_positional_churn_at_k' "${OUT_JSON}")"
POS_CHURN_B="$("${JSON_TOOL}" -r '.summary_b.stability.avg_positional_churn_at_k' "${OUT_JSON}")"

echo "Results (lower churn is better):"
echo "A (deterministic off) set_churn@k=${SET_CHURN_A} positional_churn@k=${POS_CHURN_A}"
echo "B (deterministic on)  set_churn@k=${SET_CHURN_B} positional_churn@k=${POS_CHURN_B}"
echo "Output: ${OUT_JSON}"

awk -v a="${SET_CHURN_A}" -v b="${SET_CHURN_B}" 'BEGIN { exit !(b <= a + 1e-9) }' || {
  echo "Expected deterministic ranking to reduce churn, but set churn did not improve." >&2
  exit 1
}
