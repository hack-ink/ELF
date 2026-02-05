# ELF Memory Service v1.0 Specification

Description: ELF means Evidence-linked fact memory for agents.

Audience: Implementation LLM or engineer agent.
Language: English only. No CJK characters are allowed anywhere in this document.
Contract: English-only API inputs and outputs. Reject any CJK at the API boundary.
Implementation target: Rust is recommended. The spec is language agnostic.

Core idea:
- Postgres with pgvector is the only source of truth for notes, chunk embeddings, audit history, and the indexing outbox.
- Note-level embeddings are derived pooled vectors for update and duplicate checks.
- Qdrant is a derived index for candidate retrieval only. Qdrant must be rebuildable from Postgres vectors without calling the embedding API.
- Two write APIs have hard semantic differences:
  - add_note is deterministic and must not call any LLM.
  - add_event is LLM-driven extraction and must bind evidence for every stored note.

Multi-tenant namespace:
- tenant_id, project_id, agent_id, scope, read_profile.

Optional future work:
- Graph memory backend (Neo4j) is reserved and out of scope for v1.0.

============================================================
0. INVARIANTS (MUST HOLD)
============================================================
I1. Postgres with pgvector is the only source of truth for:
    - memory notes
    - chunk embedding vectors
    - chunk metadata
    - pooled note embeddings (derived)
    - audit and version history
    - hit logs (optional)
    - indexing outbox jobs
I2. Qdrant is derived and rebuildable:
    - Qdrant may be dropped and recreated at any time.
    - Qdrant must be rebuildable from Postgres vectors without calling the embedding API.
I3. Online retrieval:
    - Qdrant returns candidate chunk_ids.
    - Postgres returns authoritative notes and re-validates status, TTL, and scope.
I4. English-only contract:
    - Any API input containing CJK must be rejected with HTTP 422.
    - Upstream agents must canonicalize to English before calling ELF.
I5. add_note must not call any LLM under any circumstance.
I6. add_event must call the LLM extractor and must bind evidence with verbatim substring checks.

============================================================
1. CONFIGURATION (TOML)
============================================================
File: elf.toml

Rules:
- The config file path is required and must be provided with --config or -c.
- No default values are allowed in code. Every field below must be present in elf.toml unless explicitly marked optional.
- No environment variables are allowed for configuration. All values are stored in elf.toml.
- Provider api_key values must be present and non-empty.
- providers.embedding.dimensions must match storage.qdrant.vector_dim.
- chunking.enabled must be true.
- chunking.max_tokens must be greater than zero.
- chunking.overlap_tokens must be less than chunking.max_tokens.
- chunking.tokenizer_repo may be empty or omitted to inherit providers.embedding.model.

Template (all values required):

[service]
http_bind = "<REQUIRED_HOST:PORT>"
mcp_bind = "<REQUIRED_HOST:PORT>"
admin_bind = "<REQUIRED_HOST:PORT>"
log_level = "<REQUIRED_LOG_LEVEL>"

[storage.postgres]
dsn = "<REQUIRED_POSTGRES_DSN>"
pool_max_conns = <REQUIRED_INT>

[storage.qdrant]
url = "<REQUIRED_URL>"
collection = "mem_notes_v1"
vector_dim = <REQUIRED_INT>

[providers.embedding]
provider_id = "<REQUIRED_ID>"
api_base = "<REQUIRED_URL>"
api_key = "<REQUIRED_NON_EMPTY>"
path = "<REQUIRED_PATH>"
model = "<REQUIRED_MODEL>"
dimensions = "<REQUIRED_INT>"
timeout_ms = <REQUIRED_INT>
# Must exist. Empty map is allowed.
default_headers = {}

[providers.rerank]
provider_id = "<REQUIRED_ID>"
api_base = "<REQUIRED_URL>"
api_key = "<REQUIRED_NON_EMPTY>"
path = "<REQUIRED_PATH>"
model = "<REQUIRED_MODEL>"
timeout_ms = <REQUIRED_INT>
# Must exist. Empty map is allowed.
default_headers = {}

[providers.llm_extractor]
provider_id = "<REQUIRED_ID>"
api_base = "<REQUIRED_URL>"
api_key = "<REQUIRED_NON_EMPTY>"
path = "<REQUIRED_PATH>"
model = "<REQUIRED_MODEL>"
temperature = <REQUIRED_FLOAT>
timeout_ms = <REQUIRED_INT>
# Must exist. Empty map is allowed.
default_headers = {}

[scopes]
allowed = ["agent_private", "project_shared", "org_shared"]

[scopes.read_profiles]
private_only = ["agent_private"]
private_plus_project = ["agent_private", "project_shared"]
all_scopes = ["agent_private", "project_shared", "org_shared"]

[scopes.precedence]
agent_private = 30
project_shared = 20
org_shared = 10

[scopes.write_allowed]
agent_private = true
project_shared = true
org_shared = true

[memory]
max_notes_per_add_event = 3
max_note_chars = 240
# Similarity thresholds
dup_sim_threshold = 0.92
update_sim_threshold = 0.85
# Retrieval sizes
candidate_k = 60
top_k = 12

[chunking]
enabled = true
max_tokens = <REQUIRED_INT>
overlap_tokens = <REQUIRED_INT>
# Optional. Empty or omitted uses providers.embedding.model.
tokenizer_repo = "<OPTIONAL_STRING>"

[search.expansion]
mode = "off|always|dynamic"
max_queries = <REQUIRED_INT>
include_original = <REQUIRED_BOOL>

[search.dynamic]
min_candidates = <REQUIRED_INT>
min_top_score = <REQUIRED_FLOAT>

[search.prefilter]
max_candidates = <REQUIRED_INT>

[search.cache]
enabled = <REQUIRED_BOOL>
expansion_ttl_days = <REQUIRED_INT>
rerank_ttl_days = <REQUIRED_INT>
# Optional. Omit to disable payload size limits.
max_payload_bytes = <OPTIONAL_INT>
expansion_version = "<REQUIRED_NON_EMPTY>"
rerank_version = "<REQUIRED_NON_EMPTY>"

[search.explain]
retention_days = <REQUIRED_INT>

[ranking]
recency_tau_days = 60
tie_breaker_weight = 0.1

[lifecycle.ttl_days]
plan = 14
fact = 180
preference = 0
constraint = 0
decision = 0
profile = 0

[lifecycle]
purge_deleted_after_days = 30
purge_deprecated_after_days = 180

[security]
bind_localhost_only = true
reject_cjk = true
redact_secrets_on_write = true
# Evidence rules for add_event
evidence_min_quotes = 1
evidence_max_quotes = 2
evidence_max_quote_chars = 320

============================================================
2. CLI AND CONFIG LOADING
============================================================
- elf-api, elf-worker, and elf-mcp are separate binaries.
- Each binary requires a config path via --config or -c.
- Startup must fail with a clear error if any required config field is missing.
- security.reject_cjk must be true. Startup must fail if it is false.

============================================================
3. ENGLISH-ONLY BOUNDARY
============================================================
Definition:
- CJK detection is the presence of any codepoint in the following Unicode blocks:
  - CJK Unified Ideographs
  - CJK Symbols and Punctuation
  - Hiragana
  - Katakana
  - Hangul

Policy:
- If security.reject_cjk is true, any CJK in any string field listed below must return HTTP 422.

Fields to check:
- add_note: notes[].text, notes[].key (optional), source_ref string fields if any
- add_event: messages[].content
- search: query

Error response:
HTTP 422
{
  "error_code": "NON_ENGLISH_INPUT",
  "message": "CJK detected; upstream must canonicalize to English before calling ELF.",
  "fields": ["$.messages[2].content", "$.notes[0].text"]
}

============================================================
4. DOMAIN MODEL
============================================================
4.1 Memory types (exactly 6)
- preference
- constraint
- decision
- profile
- fact
- plan

4.2 Canonical note
- A note is a short English sentence and must be <= max_note_chars.
- Format is not enforced. Recommended prefixes for consistency:
  "Preference: ...", "Constraint: ...", "Decision: ...", "Profile: ...", "Fact: ...", "Plan: ..."

4.3 Keys
- key is optional but strongly recommended for stable updates.
- key examples: preferred_language, no_secrets_policy, architecture_sot, project_workflow, long_term_goal.

============================================================
5. POSTGRES SCHEMA (SOURCE OF TRUTH + PGVECTOR)
============================================================
Startup must:
- CREATE EXTENSION IF NOT EXISTS vector;
- Execute sql/init.sql.

Schema location:
- All schema and index DDL must live under sql/ and be orchestrated by sql/init.sql.
- sql/init.sql must be idempotent and include the per-table files in dependency order.

5.1 memory_notes (authoritative notes)
Columns:
- note_id uuid primary key
- tenant_id text not null
- project_id text not null
- agent_id text not null
- scope text not null
- type text not null
- key text null
- text text not null
- importance real not null
- confidence real not null
- status text not null
- created_at timestamptz not null
- updated_at timestamptz not null
- expires_at timestamptz null
- embedding_version text not null
- source_ref jsonb not null
- hit_count bigint not null default 0
- last_hit_at timestamptz null

Indexes (minimum):
- idx_notes_scope_status: (tenant_id, project_id, scope, status)
- idx_notes_key: (tenant_id, project_id, agent_id, scope, type, key) WHERE key IS NOT NULL
- idx_notes_expires: (expires_at)

5.2 memory_note_chunks (chunk metadata)
Columns:
- chunk_id uuid primary key
- note_id uuid not null references memory_notes(note_id) on delete cascade
- chunk_index int not null
- start_offset int not null
- end_offset int not null
- text text not null
- embedding_version text not null
- created_at timestamptz not null default now()

Indexes (minimum):
- idx_note_chunks_note: (note_id)
- idx_note_chunks_note_index: (note_id, chunk_index)

5.3 note_chunk_embeddings (source of truth vectors; pgvector)
- chunk_id uuid references memory_note_chunks(chunk_id) on delete cascade
- embedding_version text not null
- embedding_dim int not null
- vec vector(<vector_dim>) not null
- created_at timestamptz not null default now()
primary key(chunk_id, embedding_version)

Rules:
- Every memory_note_chunks row must have a corresponding note_chunk_embeddings row for its embedding_version.
- Chunk embeddings are the source of truth for retrieval and rebuild.

5.4 note_embeddings (derived pooled vectors; pgvector)
- note_id uuid references memory_notes(note_id) on delete cascade
- embedding_version text not null
- embedding_dim int not null
- vec vector(<vector_dim>) not null
- created_at timestamptz not null default now()
primary key(note_id, embedding_version)

Rules:
- note_embeddings is derived by mean pooling chunk embeddings for (note_id, embedding_version).
- note_embeddings must be refreshed whenever chunk embeddings change.

5.5 memory_note_versions (append-only audit)
- version_id uuid primary key
- note_id uuid not null
- op text not null
- prev_snapshot jsonb null
- new_snapshot jsonb null
- reason text not null
- actor text not null
- ts timestamptz not null default now()

5.6 memory_hits (optional)
- hit_id uuid primary key
- note_id uuid not null
- chunk_id uuid null
- query_hash text not null
- rank int not null
- final_score real not null
- ts timestamptz not null default now()

5.7 indexing_outbox (guaranteed indexing)
- outbox_id uuid primary key
- note_id uuid not null
- op text not null
- embedding_version text not null
- status text not null
- attempts int not null default 0
- last_error text null
- available_at timestamptz not null default now()
- created_at timestamptz not null default now()
- updated_at timestamptz not null default now()

Indexes:
- idx_outbox_status_available: (status, available_at)
- idx_outbox_note_op_status: (note_id, op, status)

5.8 search_traces (search explainability)
- trace_id uuid primary key
- tenant_id text not null
- project_id text not null
- agent_id text not null
- read_profile text not null
- query text not null
- expansion_mode text not null
- expanded_queries jsonb not null
- allowed_scopes jsonb not null
- candidate_count int not null
- top_k int not null
- config_snapshot jsonb not null
- trace_version int not null
- created_at timestamptz not null
- expires_at timestamptz not null

Indexes:
- idx_search_traces_expires: (expires_at)
- idx_search_traces_context: (tenant_id, project_id, created_at)

5.9 search_trace_items (per-result explain data)
- item_id uuid primary key
- trace_id uuid not null references search_traces(trace_id) on delete cascade
- note_id uuid not null
- chunk_id uuid null
- rank int not null
- retrieval_score real null
- retrieval_rank int null
- rerank_score real not null
- tie_breaker_score real not null
- final_score real not null
- boosts jsonb not null
- matched_terms jsonb not null
- matched_fields jsonb not null

Indexes:
- idx_search_trace_items_trace: (trace_id, rank)
- idx_search_trace_items_note: (note_id)

5.10 search_trace_outbox (async trace persistence)
- outbox_id uuid primary key
- trace_id uuid not null
- status text not null
- attempts int not null default 0
- last_error text null
- available_at timestamptz not null default now()
- payload jsonb not null
- created_at timestamptz not null default now()
- updated_at timestamptz not null default now()

Indexes:
- idx_trace_outbox_status_available: (status, available_at)
- idx_trace_outbox_trace_status: (trace_id, status)

5.11 llm_cache (LLM response cache)
- cache_id uuid primary key
- cache_kind text not null
- cache_key text not null
- payload jsonb not null
- created_at timestamptz not null
- last_accessed_at timestamptz not null
- expires_at timestamptz not null
- hit_count bigint not null default 0

Indexes:
- idx_llm_cache_key: (cache_kind, cache_key) unique
- idx_llm_cache_expires: (expires_at)

============================================================
6. QDRANT COLLECTION (DERIVED INDEX ONLY)
============================================================
- Collection: storage.qdrant.collection
- Dense vector: named `dense` with size storage.qdrant.vector_dim (cosine distance).
- Sparse vector: named `bm25` with `idf` modifier and model `qdrant/bm25`.
- Point id: chunk_id (string UUID)
- Payload fields (minimum):
  note_id, chunk_id, chunk_index, start_offset, end_offset,
  tenant_id, project_id, agent_id, scope, type, key, status,
  updated_at, expires_at, importance, confidence, embedding_version
- Chunk text is not stored in Qdrant payload.

IMPORTANT:
- Qdrant may be stale. Postgres is authoritative.

============================================================
7. PROVIDER ADAPTERS (HTTP)
============================================================
7.1 EmbeddingProvider
Function:
- embed(texts[]) -> vectors[][]

Contract:
- Output vector count equals input text count.
- Each vector length equals vector_dim.

Implementation:
- POST {api_base}{path}
  { "model": model, "input": [texts...], "dimensions": dimensions }
- Send Authorization: Bearer <api_key>.
- Merge default_headers into the request.
- Map response to float32[D].

embedding_version:
- "<provider_id>:<model>:<vector_dim>"

7.2 RerankProvider
Function:
- rerank(query, docs[]) -> scores[]

Contract:
- Scores are aligned to docs indexes.

Implementation:
- POST {api_base}{path}
  { "model": model, "query": "...", "documents": ["..."] }
- Send Authorization: Bearer <api_key>.
- Merge default_headers into the request.
- Map response into aligned float[] (some providers return indexes).

7.3 LLM Extractor Provider
Function:
- extract(messages[]) -> JSON notes

Contract:
- Strict JSON output.
- If response_format is available, use it.
- Otherwise enforce JSON-only with at most 2 retries.

Implementation:
- POST {api_base}{path}
  { "model": model, "temperature": temperature, "messages": [...] }
- Send Authorization: Bearer <api_key>.
- Merge default_headers into the request.

============================================================
8. API SEMANTICS: add_note vs add_event (HARD DIFFERENCES)
============================================================
8.1 add_note (deterministic write)
MUST:
- Must not call any LLM.
- Must treat input notes as authoritative content with no rewriting.
- Must apply WriteGate, UpdateResolver, persistence, and indexing outbox.
- Must return per-note op result: ADD, UPDATE, NONE, or REJECTED with reason_code.

MUST NOT:
- Must not infer missing type, scope, or key beyond validation defaults.
- Must not generate new text.

8.2 add_event (LLM extraction write)
MUST:
- Must call the LLM extractor exactly once per request.
- Must require evidence binding for each candidate note.
- Must enforce max_notes_per_add_event on the server.
- Must apply WriteGate and UpdateResolver after extraction.
- Should support dry_run to return candidates without persisting.

MUST NOT:
- Must not store notes lacking evidence or failing evidence substring checks.
- Must not store raw full logs as memory notes.
 - If evidence.quote is not a verbatim substring of the cited message, return REJECTED with reason_code REJECT_EVIDENCE_MISMATCH.

============================================================
9. WRITEGATE (SERVER SIDE, ALWAYS ON)
============================================================
Reject a note if any of the following are true:
- The note contains CJK.
- The type is not in the 6-type allowlist.
- The scope is not allowed or write not allowed.
- The text length is greater than max_note_chars.
- Secrets or PII are detected (regex and heuristics).
- The text is empty or whitespace only.

On rejection:
- op = REJECTED
- reason_code is one of:
  REJECT_CJK, REJECT_TOO_LONG, REJECT_SECRET, REJECT_INVALID_TYPE,
  REJECT_SCOPE_DENIED, REJECT_EMPTY

============================================================
10. UPDATE RESOLVER (IN-PLACE UPDATE, STABLE note_id)
============================================================
Resolution namespace group:
(tenant_id, project_id, agent_id, scope, type)

Order:
1) Key-based:
   - If key is not null and an active note exists with the same key:
     -> UPDATE in place (same note_id).
2) Similarity-based (when key is null):
   - Compute embedding for incoming text.
   - Compare cosine similarity vs existing active notes in the group using Postgres-stored vec.
   - If sim >= dup_sim_threshold -> NONE.
   - Else if sim >= update_sim_threshold -> UPDATE best match in place.
   - Else -> ADD new note_id.

On UPDATE:
- Preserve note_id.
- Write memory_note_versions with prev and new snapshots.
- Update memory_notes.text, updated_at, expires_at, source_ref, confidence, importance.
- Enqueue outbox UPSERT.

============================================================
11. TTL AND LIFECYCLE
============================================================
TTL assignment on write:
- If request.ttl_days is provided and > 0 -> expires_at = now + ttl_days.
- Else if lifecycle.ttl_days[type] > 0 -> expires_at = now + ttl_days[type].
- Else expires_at = NULL.

GC job (daily):
- If status = deleted and deleted age > purge_deleted_after_days -> hard purge row (cascade).
- If status = deprecated and last_hit_at older than purge_deprecated_after_days -> delete or purge.
- If expires_at < now -> set status = deleted + version row + outbox DELETE.

============================================================
12. PERSISTENCE AND INDEXING (SOURCE OF TRUTH FIRST + OUTBOX)
============================================================
For every ADD, UPDATE, DEPRECATE, or DELETE, the Postgres transaction must:
- Update memory_notes.
- Write memory_note_versions.
- Insert indexing_outbox (UPSERT or DELETE) as PENDING.
- Commit.

After commit:
- Best-effort inline outbox processing may run.
- Correctness is guaranteed by the background worker.

Worker rules:
- For UPSERT:
  - Fetch memory_notes row.
  - If not active or expired -> mark outbox DONE and skip indexing.
  - Split note text into sentence-aware chunks.
  - Upsert memory_note_chunks rows for (note_id, chunk_index).
  - Call embedding API for chunk text and upsert note_chunk_embeddings.
  - Compute pooled note vector by mean pooling chunk embeddings and upsert note_embeddings.
  - Upsert one Qdrant point per chunk with dense and bm25 vectors plus payload.
  - Mark outbox DONE.
- For DELETE:
  - Delete Qdrant points by note_id filter (ignore not found).
  - Mark DONE.
- Failures:
  - status = FAILED, attempts += 1, available_at = now + backoff(attempts).

Search trace outbox (best-effort):
- Search enqueues trace payloads into search_trace_outbox with status = PENDING.
- Worker leases available jobs, inserts search_traces and search_trace_items, then marks DONE.
- On failure, status = FAILED, attempts += 1, last_error set, available_at = now + backoff(attempts).
- Failures must not affect the original search response.

Periodic cleanup:
- Worker deletes expired search_traces (search_trace_items cascade).
- Worker deletes expired llm_cache rows.

============================================================
13. SEARCH PIPELINE (ONLINE)
============================================================
Input:
- tenant_id, project_id, agent_id
- read_profile
- query (English only)
- optional top_k, candidate_k, record_hits

Config:
- search.expansion.mode = off|always|dynamic
- search.expansion.max_queries
- search.expansion.include_original (default true)
- search.dynamic.min_candidates
- search.dynamic.min_top_score
- search.prefilter.max_candidates (0 or >= candidate_k means no prefilter)
- search.cache.enabled
- search.cache.expansion_ttl_days
- search.cache.rerank_ttl_days
- search.cache.max_payload_bytes (optional)
- search.cache.expansion_version
- search.cache.rerank_version
- search.explain.retention_days

Steps:
1) English-only boundary check.
2) Resolve allowed_scopes = scopes.read_profiles[read_profile].
3) Resolve expansion mode:
   - off: use only original query.
   - always: expand with LLM.
   - dynamic: run a baseline hybrid search for the original query, then expand if
     candidate_count < min_candidates OR top1_fusion_score < min_top_score.
4) If expansion is enabled, resolve expanded queries with cache support.
   - Build an expansion cache key from: query (trimmed), provider_id, model, temperature,
     expansion_version, max_queries, include_original.
   - If search.cache.enabled and a non-expired cache entry exists, use cached queries.
   - On cache miss, call the LLM expansion prompt and receive queries[].
     - Deduplicate, strip CJK, and cap at max_queries.
     - Ensure original query is present when include_original = true.
   - If search.cache.enabled and payload size is within max_payload_bytes (when set),
     store the expanded queries with TTL = expansion_ttl_days.
5) For each query, embed -> query_vec (embedding API).
6) For each query, run Qdrant fusion query candidate_k with payload filters (dense + bm25):
   tenant_id, project_id, status = active (best-effort), and scope filters:
   - If scope = agent_private, require agent_id match.
   - Otherwise scope in allowed_scopes.
7) Fuse all query results with RRF to produce candidate chunk_ids.
8) Prefilter (optional): if max_candidates > 0 and max_candidates < candidate_k,
   keep only top max_candidates by fusion score.
9) Fetch authoritative notes from Postgres by note_id and re-apply filters:
   status = active, not expired, scope allowed, and if scope = agent_private then agent_id must match.
10) Fetch chunk metadata for candidate chunks and immediate neighbors from memory_note_chunks.
11) Stitch snippets from chunk text (chunk + neighbors).
12) Rerank once using the original query, with cache support:
    - Build a rerank cache key from: query (trimmed), provider_id, model, rerank_version,
      and the candidate signature [(chunk_id, note_updated_at)...].
    - If search.cache.enabled and a cache entry exists that matches the candidate signature,
      reuse cached scores.
    - On cache miss, call the rerank provider:
      scores = rerank(original_query, docs = [snippet ...]).
    - If search.cache.enabled and payload size is within max_payload_bytes (when set),
      store the rerank scores with TTL = rerank_ttl_days.
13) Tie-break:
    base = (1 + 0.6 * importance) * exp(-age_days / recency_tau_days)
    final = rerank_score + tie_breaker_weight * base
14) Aggregate by note using top-1 chunk score, then sort and take top_k.
15) Update hits (optional, when record_hits is true):
    hit_count++, last_hit_at, memory_hits insert with chunk_id.
16) Build search trace payload with trace_id and per-item result_handle, then enqueue
    search_trace_outbox (best-effort; failures do not fail the search).
    - expires_at = now + search.explain.retention_days.
17) Return results.

Cache notes:
- Cache key material is serialized as JSON and hashed with BLAKE3 (256-bit hex).
- Cache read/write failures are treated as misses and must not fail the search request.

============================================================
14. ADMIN: REBUILD QDRANT FROM POSTGRES (NO EMBED API)
============================================================
Endpoint (localhost only):
POST /v1/admin/rebuild_qdrant

Behavior:
- Scan memory_note_chunks joined to memory_notes where status = active and not expired.
- For each chunk:
  - Load vec from note_chunk_embeddings (chunk_id, embedding_version).
  - Upsert Qdrant point with chunk vectors and payload.
- Must not call the embedding API.

Report:
- rebuilt_count
- missing_vector_count (notes without vec)
- error_count

============================================================
15. HTTP API (PUBLIC)
============================================================
Base: service.http_bind

POST /v1/memory/add_note
Body:
{
  "tenant_id": "...",
  "project_id": "...",
  "agent_id": "...",
  "scope": "agent_private|project_shared|org_shared",
  "notes": [
    {
      "type": "preference|constraint|decision|profile|fact|plan",
      "key": "string|null",
      "text": "English-only sentence",
      "importance": 0.0-1.0,
      "confidence": 0.0-1.0,
      "ttl_days": number|null,
      "source_ref": { ... }
    }
  ]
}
Response:
{
  "results": [
    { "note_id": "uuid", "op": "ADD|UPDATE|NONE|REJECTED", "reason_code": "..." }
  ]
}

POST /v1/memory/add_event
Body:
{
  "tenant_id": "...",
  "project_id": "...",
  "agent_id": "...",
  "scope": "optional-scope",
  "dry_run": false,
  "messages": [
    { "role": "user|assistant|tool", "content": "English-only", "ts": "optional", "msg_id": "optional" }
  ]
}
Response:
{
  "extracted": [ ...extractor output... ],
  "results": [
    { "note_id": "uuid|null", "op": "ADD|UPDATE|NONE|REJECTED", "reason_code": "...", "reason": "..." }
  ]
}
Notes:
- reason_code values include WriteGate rejection codes and REJECT_EVIDENCE_MISMATCH.

POST /v1/memory/search
Body:
{
  "tenant_id": "...",
  "project_id": "...",
  "agent_id": "...",
  "read_profile": "private_only|private_plus_project|all_scopes",
  "query": "English-only",
  "top_k": 12,
  "candidate_k": 60,
  "record_hits": false
}
Response:
{
  "trace_id": "uuid",
  "items": [
    {
      "result_handle": "uuid",
      "note_id": "uuid",
      "chunk_id": "uuid",
      "chunk_index": 0,
      "start_offset": 0,
      "end_offset": 0,
      "snippet": "...",
      "type": "...",
      "key": null,
      "scope": "...",
      "importance": 0.0,
      "confidence": 0.0,
      "updated_at": "...",
      "expires_at": "...|null",
      "final_score": 0.0,
      "source_ref": { ... },
      "explain": {
        "retrieval_score": 0.0|null,
        "retrieval_rank": 1|null,
        "rerank_score": 0.0,
        "tie_breaker_score": 0.0,
        "final_score": 0.0,
        "boosts": [{"name": "recency_importance", "score": 0.0}],
        "matched_terms": ["..."],
        "matched_fields": ["text","key"]
      }
    }
  ]
}
Notes:
- result_handle is a stable handle for search explain.
- record_hits defaults to false when omitted.

GET /v1/memory/search/explain?result_handle=...
Response:
{
  "trace": {
    "trace_id": "uuid",
    "tenant_id": "...",
    "project_id": "...",
    "agent_id": "...",
    "read_profile": "...",
    "query": "...",
    "expansion_mode": "off|always|dynamic",
    "expanded_queries": ["..."],
    "allowed_scopes": ["..."],
    "candidate_count": 0,
    "top_k": 0,
    "config_snapshot": { ... },
    "trace_version": 1,
    "created_at": "..."
  },
  "item": {
    "result_handle": "uuid",
    "note_id": "uuid",
    "chunk_id": "uuid",
    "rank": 1,
    "explain": {
      "retrieval_score": 0.0|null,
      "retrieval_rank": 1|null,
      "rerank_score": 0.0,
      "tie_breaker_score": 0.0,
      "final_score": 0.0,
      "boosts": [{"name": "recency_importance", "score": 0.0}],
      "matched_terms": ["..."],
      "matched_fields": ["text","key"]
    }
  }
}
Notes:
- If result_handle is unknown or the trace has not been persisted yet, return INVALID_REQUEST.

GET /v1/memory/notes/{note_id}
Response:
{
  "note_id": "uuid",
  "tenant_id": "...",
  "project_id": "...",
  "agent_id": "...",
  "scope": "...",
  "type": "...",
  "key": null,
  "text": "...",
  "importance": 0.0,
  "confidence": 0.0,
  "status": "...",
  "updated_at": "...",
  "expires_at": "...|null",
  "source_ref": { ... }
}

GET /v1/memory/list?tenant_id=...&project_id=...&scope=...&status=...&type=...&agent_id=...
Notes:
- If scope = agent_private, agent_id is required.
- If scope is omitted, agent_private notes are excluded.

POST /v1/memory/update
Body:
{
  "tenant_id": "...",
  "project_id": "...",
  "agent_id": "...",
  "note_id": "uuid",
  "text": "optional",
  "importance": 0.0-1.0 optional,
  "confidence": 0.0-1.0 optional,
  "ttl_days": number|null
}
Notes:
- If ttl_days is omitted, expires_at remains unchanged.
- If ttl_days <= 0, apply default TTL rules for the note type.
Response:
{
  "note_id": "uuid",
  "op": "UPDATE|NONE|REJECTED",
  "reason_code": "optional"
}

POST /v1/memory/delete
Body:
{
  "tenant_id": "...",
  "project_id": "...",
  "agent_id": "...",
  "note_id": "uuid"
}
Response:
{
  "note_id": "uuid",
  "op": "DELETE|NONE"
}
GET /health

Error codes (common):
- NON_ENGLISH_INPUT (422)
- SCOPE_DENIED (403)
- INVALID_REQUEST (400)
- INVALID_REQUEST (400)
- INTERNAL_ERROR (500)

============================================================
16. LLM QUERY EXPANSION PROMPT (search) - APPENDIX
============================================================
LLM output must be JSON only and match the schema below.

Schema:
{
  "queries": ["string", "..."]
}

Hard rules:
- queries.length <= MAX_QUERIES
- Each query must be English only and must not contain any CJK characters.
- Each query must be a single sentence.
- Include the original query unless INCLUDE_ORIGINAL is false.

System prompt (Expansion):
"You are a query expansion engine for a memory retrieval system.
Output must be valid JSON only and must match the provided schema exactly.
Generate short English-only query variations that preserve the original intent.
Do not include any CJK characters. Do not add explanations or extra fields."

User prompt template:
"Return JSON matching this exact schema:
<SCHEMA_JSON>
Constraints:
- MAX_QUERIES = <MAX_QUERIES>
- INCLUDE_ORIGINAL = <INCLUDE_ORIGINAL>
Original query:
<QUERY>"

============================================================
17. MCP ADAPTER (SEPARATE PROCESS)
============================================================
- Separate binary: elf-mcp.
- Streamable HTTP MCP server.
- Tools map 1:1 to HTTP endpoints:
  memory_add_note, memory_add_event, memory_search, memory_list, memory_update, memory_delete.
- The MCP server must contain zero business logic or policy.
- All policy remains in elf-api.

============================================================
18. LLM EXTRACTOR PROMPT (add_event) - APPENDIX
============================================================
LLM output must be JSON only and match the schema below.

Schema:
{
  "notes": [
    {
      "type": "preference|constraint|decision|profile|fact|plan",
      "key": "string|null",
      "text": "English-only sentence <= MAX_NOTE_CHARS",
      "importance": 0.0,
      "confidence": 0.0,
      "ttl_days": number|null,
      "scope_suggestion": "agent_private|project_shared|org_shared|null",
      "evidence": [
        { "message_index": number, "quote": "string" }
      ],
      "reason": "string"
    }
  ]
}

Hard rules:
- notes.length <= MAX_NOTES
- text must contain no CJK
- each note must be one sentence
- evidence must be 1..2 quotes
- each evidence.quote must be a verbatim substring of messages[message_index].content
- do not store secrets or PII

System prompt (Extractor):
"You are a memory extraction engine for an agent memory system.
Output must be valid JSON only and must match the provided schema exactly.
Extract at most MAX_NOTES high-signal, cross-session reusable memory notes from the given messages.
Each note must be one English sentence and must not contain any CJK characters.
Preserve numbers, dates, percentages, currency amounts, tickers, URLs, and code snippets exactly.
Never store secrets or PII: API keys, tokens, private keys, seed phrases, passwords, bank IDs, personal addresses.
For every note, provide 1 to 2 evidence quotes copied verbatim from the input messages and include the message_index.
If you cannot provide verbatim evidence, omit the note.
If content is ephemeral or not useful long-term, return an empty notes array."

User prompt template:
"Return JSON matching this exact schema:
<SCHEMA_JSON>
Constraints:
- MAX_NOTES = <MAX_NOTES>
- MAX_NOTE_CHARS = <MAX_NOTE_CHARS>
Here are the messages as JSON:
<MESSAGES_JSON>"

============================================================
19. TESTS AND ACCEPTANCE CRITERIA
============================================================
A. add_note does not call LLM:
- Instrument LLM client call count. It must remain 0 during add_note tests.
B. English-only boundary:
- Any CJK in add_note, add_event, or search returns HTTP 422 with field path.
C. Evidence binding:
- If extractor evidence.quote is not a substring -> REJECTED with REJECT_EVIDENCE_MISMATCH.
D. Rebuild:
- Drop Qdrant collection, recreate, call /admin/rebuild_qdrant.
- Must succeed without calling embedding API.
E. Source of truth vectors:
- For every active chunk, note_chunk_embeddings row exists and vec dim matches config.
- note_embeddings exists for active notes as derived pooled vectors.
F. Idempotency:
- add_note same payload twice -> second op = NONE.
G. Outbox eventual consistency:
- Simulate embedding provider outage.
- Outbox goes FAILED and later retries to DONE after provider recovers.

============================================================
20. OUT OF SCOPE (v1.0)
============================================================
- Translation or multilingual retrieval (handled by upstream agents).
- Graph memory backend (reserved for later).
- Public internet exposure and auth (localhost only in v1.0).
