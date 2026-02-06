# Integration Testing (Memory Retrieval)

Purpose: Provide a repeatable E2E test for memory ingestion, indexing, and retrieval.
Name: This flow is the E2E test in `docs/guide/testing.md`.

## When to use

- After adding or changing memory ingestion, ranking, or storage behavior.
- Before shipping changes that affect retrieval quality or service wiring.

## Preconditions

- Postgres is running and reachable.
- Qdrant is running and reachable.
- You have a config file with valid storage and provider settings.
- You can create and drop a dedicated database named `elf_e2e`.

Note: Use the existing collection configured in your `elf.toml`. Do not create a new collection for this flow. Keep test data isolated by tenant, project, and agent identifiers, then clean it up after the run.
Note: Qdrant exposes a REST API (default: 6333) and a gRPC API (default: 6334). The `storage.qdrant.url` field is the gRPC base URL. In this repository's local setup, REST is commonly mapped to port 51889 and gRPC to port 51890.
Note: The local Postgres instance in this repository typically runs on port `51888`. Adjust the DSN if your setup differs.

## Step 1: Prepare a dedicated integration config

Create a dedicated config file for integration tests (for example, `tmp/elf.integration.toml`) and point it to your running services. If `tmp/elf.integration.toml` already exists at the repository root, reuse it and update the DSN and service URLs as needed. Use `api_base` for provider endpoints.

```toml
[service]
admin_bind = "127.0.0.1:51891"
http_bind  = "127.0.0.1:51892"
mcp_bind   = "127.0.0.1:51893"
log_level  = "info"

[storage.postgres]
dsn            = "postgres://postgres:postgres@127.0.0.1:51888/elf_e2e"
pool_max_conns = 10

[storage.qdrant]
collection = "mem_notes_v1"
url        = "http://127.0.0.1:51890"
vector_dim = 4096

[providers.embedding]
api_base        = "https://provider.example/v1"
api_key         = "REPLACE_ME"
model           = "embedding-model"
path            = "/embeddings"
provider_id     = "provider-id"
dimensions      = 4096
timeout_ms      = 20000

default_headers = {}

[providers.rerank]
api_base        = "https://provider.example/v1"
api_key         = "REPLACE_ME"
model           = "rerank-model"
path            = "/rerank"
provider_id     = "provider-id"
timeout_ms      = 20000

default_headers = {}

[providers.llm_extractor]
api_base        = "https://provider.example/v1"
api_key         = "REPLACE_ME"
model           = "llm-model"
path            = "/chat/completions"
provider_id     = "provider-id"
temperature     = 0.1
timeout_ms      = 30000

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
# If empty, uses providers.embedding.model.
tokenizer_repo = ""

[search.expansion]
mode             = "dynamic"
max_queries      = 4
include_original = true

[search.dynamic]
min_candidates = 10
min_top_score  = 0.12

[search.prefilter]
max_candidates = 0

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
```

## Step 2: Start the worker and API

From the repository root:

```bash
cargo run -p elf-worker -- --config tmp/elf.integration.toml
```

In a second terminal:

```bash
cargo run -p elf-api -- --config tmp/elf.integration.toml
```

## Step 3: Add test notes

Use a dedicated tenant, project, and agent to isolate test data.

```bash
curl -sS http://127.0.0.1:51892/v1/memory/add_note \
  -H 'content-type: application/json' \
  -d '{
    "tenant_id": "it-tenant",
    "project_id": "it-project",
    "agent_id": "it-agent",
    "scope": "project_shared",
    "notes": [
      {
        "type": "fact",
        "key": "embeddings_storage",
        "text": "Embeddings are stored in Postgres and indexed in Qdrant.",
        "importance": 0.7,
        "confidence": 0.9,
        "ttl_days": 180,
        "source_ref": {"run": "integration-test"}
      },
      {
        "type": "fact",
        "key": "rerank_order",
        "text": "Search uses reranking after hybrid retrieval.",
        "importance": 0.7,
        "confidence": 0.9,
        "ttl_days": 180,
        "source_ref": {"run": "integration-test"}
      }
    ]
  }'
```

Record the returned `note_id` values from `results[].note_id`. These are required for the evaluation dataset and cleanup.

Note: Requests reject CJK content. Use English-only text and keys.

## Step 4: Create the evaluation dataset

Create `tmp/eval.json` with expected note IDs from the add-note call.

```json
{
  "name": "integration",
  "defaults": {
    "tenant_id": "it-tenant",
    "project_id": "it-project",
    "agent_id": "it-agent",
    "read_profile": "all_scopes",
    "top_k": 12,
    "candidate_k": 60
  },
  "queries": [
    {
      "id": "q-1",
      "query": "Where are embeddings stored?",
      "expected_note_ids": ["NOTE_ID_1"]
    },
    {
      "id": "q-2",
      "query": "How does ranking work in search?",
      "expected_note_ids": ["NOTE_ID_2"]
    }
  ]
}
```

## Step 5: Run the evaluation

```bash
cargo run -p elf-eval -- --config tmp/elf.integration.toml --dataset tmp/eval.json
```

Review the JSON output for recall, precision, and latency metrics.

## Acceptance criteria

Use these criteria as a starting point. Adjust thresholds based on your provider and workload.

Required (integration smoke):
- The evaluation completes without errors.
- Each query returns at least one expected note in `top_k`.

Recommended (quality signal):
- Mean recall@k across queries is at least 0.8.
- p95 latency is within 2x of the last known baseline for the same environment.

## Step 6: Clean up test notes

Use the returned note IDs from Step 3.

```bash
curl -sS http://127.0.0.1:51892/v1/memory/delete \
  -H 'content-type: application/json' \
  -d '{
    "tenant_id": "it-tenant",
    "project_id": "it-project",
    "agent_id": "it-agent",
    "note_id": "NOTE_ID_1"
  }'

curl -sS http://127.0.0.1:51892/v1/memory/delete \
  -H 'content-type: application/json' \
  -d '{
    "tenant_id": "it-tenant",
    "project_id": "it-project",
    "agent_id": "it-agent",
    "note_id": "NOTE_ID_2"
  }'
```

## Troubleshooting

- If results do not appear immediately, wait a few seconds for the outbox worker to index, then re-run the evaluation.
- If Qdrant connectivity warnings appear, verify the configured `storage.qdrant.url` and that the service is reachable.
