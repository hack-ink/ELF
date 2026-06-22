---
type: Spec
title: "ELF Memory Service v2.0 Specification"
description: "Define the ELF Memory Service v2.0 contract, invariants, and storage model."
resource: docs/spec/system_elf_memory_service_v2.md
status: active
authority: normative
owner: spec
last_verified: 2026-06-22
tags:
  - docs
  - spec
source_refs: []
code_refs: []
related: []
drift_watch:
  - docs/spec/system_elf_memory_service_v2.md
---
# ELF Memory Service v2.0 Specification

Purpose: Define the ELF Memory Service v2.0 contract, invariants, and storage model.
Status: normative
Read this when: You are implementing, validating, or reviewing the core ELF memory service behavior.
Not this document: Operator runbooks, local setup steps, or work-item triage workflows.
Defines: ELF Memory Service v2.0 API semantics, ingestion boundaries, and storage invariants.

Description: ELF means Evidence-linked fact memory for agents.

Audience: Implementation LLM or engineer agent.
Language: English only.
Contract: English-only API inputs and outputs. Reject non-English input at the API boundary.
Implementation target: Rust is recommended. The spec is language agnostic.

Core idea:
- Postgres with pgvector is the only source of truth for notes, chunk embeddings, audit history, and the indexing outbox.
- Note-level embeddings are derived pooled vectors for update and duplicate checks.
- Qdrant is a derived index for candidate retrieval only. Qdrant must be rebuildable from Postgres vectors without calling the embedding API.
- Two write APIs have hard semantic differences:
  - add_note is deterministic and must not call any LLM.
  - add_event is LLM-driven extraction and must bind evidence for every stored note.

Core vs Extensions:
- ELF Core is the high-trust, facts-first memory service defined by this specification.
  - It owns: notes/events ingestion semantics, scopes/sharing, search, auditability, and the English gate.
  - It must remain simple, deterministic where specified, and operable without any optional components.
- ELF Extensions are optional capability modules that may evolve independently without changing Core semantics.
  - Extensions must not weaken Core invariants or introduce hidden dependencies into Core flows.
  - Extensions should integrate via stable contracts (e.g., versioned source_ref pointers and bounded excerpt hydration).
  - Example extension (future): an Evidence Store / Doc Platform used for long-form evidence storage and progressive loading.

Multi-tenant namespace:
- tenant_id, project_id, agent_id, scope, read_profile.

Optional future work:
- Graph memory backend is defined in Postgres in `system_graph_memory_postgres_v1.md` and kept aligned with this specification.

============================================================
0. INVARIANTS (MUST HOLD)
============================================================
I1. Postgres with pgvector is the only source of truth for:
    - memory notes
    - scoped core memory blocks and attachments
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
    - Any API input that fails the English gate (defined below) must be rejected with HTTP 422.
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
- chunking.tokenizer_repo must be present and non-empty.

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
collection = "mem_notes_v2"
docs_collection = "doc_chunks_v1"
vector_dim = <REQUIRED_INT>

[providers.embedding]
provider_id = "<REQUIRED_ID>"
api_base = "<REQUIRED_URL>"
api_key = "<REQUIRED_NON_EMPTY>"
path = "<REQUIRED_PATH>"
model = "<REQUIRED_MODEL>"
dimensions = <REQUIRED_INT>
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

[memory.policy]

[[memory.policy.rules]]
note_type = "fact|plan|preference|constraint|decision|profile"
scope = "agent_private|project_shared|org_shared"
min_confidence = <OPTIONAL_FLOAT>
min_importance = <OPTIONAL_FLOAT>

[chunking]
enabled = true
max_tokens = <REQUIRED_INT>
overlap_tokens = <REQUIRED_INT>
tokenizer_repo = "<REQUIRED_NON_EMPTY_STRING>"

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

[search.explain]
retention_days = <REQUIRED_INT>
capture_candidates = <REQUIRED_BOOL>
candidate_retention_days = <REQUIRED_INT>
write_mode = "outbox|inline"

[search.recursive]
enabled = <REQUIRED_BOOL>
max_depth = <REQUIRED_INT>
max_children_per_node = <REQUIRED_INT>
max_nodes_per_scope = <REQUIRED_INT>
max_total_nodes = <REQUIRED_INT>

[search.graph_context]
enabled = <REQUIRED_BOOL>
max_facts_per_item = <REQUIRED_INT>
max_evidence_notes_per_fact = <REQUIRED_INT>

[ranking]
recency_tau_days = 60
tie_breaker_weight = 0.1

[ranking.deterministic]
enabled = <REQUIRED_BOOL>

[ranking.deterministic.lexical]
enabled = <REQUIRED_BOOL>
weight = <REQUIRED_FLOAT>
min_ratio = <REQUIRED_FLOAT>
max_query_terms = <REQUIRED_INT>
max_text_terms = <REQUIRED_INT>

[ranking.deterministic.hits]
enabled = <REQUIRED_BOOL>
weight = <REQUIRED_FLOAT>
half_saturation = <REQUIRED_FLOAT>
last_hit_tau_days = <REQUIRED_FLOAT>

[ranking.deterministic.decay]
enabled = <REQUIRED_BOOL>
weight = <REQUIRED_FLOAT>
tau_days = <REQUIRED_FLOAT>

[ranking.blend]
enabled = <REQUIRED_BOOL>
rerank_normalization = "<REQUIRED_STRING>"
retrieval_normalization = "<REQUIRED_STRING>"

[[ranking.blend.segments]]
max_retrieval_rank = <REQUIRED_INT>
retrieval_weight = <REQUIRED_FLOAT>

[ranking.diversity]
enabled = <REQUIRED_BOOL>
sim_threshold = <REQUIRED_FLOAT>
mmr_lambda = <REQUIRED_FLOAT>
max_skips = <REQUIRED_INT>

[ranking.retrieval_sources]
fusion_weight = <REQUIRED_FLOAT>
structured_field_weight = <REQUIRED_FLOAT>
fusion_priority = <REQUIRED_INT>
structured_field_priority = <REQUIRED_INT>

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
reject_non_english = true
redact_secrets_on_write = true
# Evidence rules for add_event
evidence_min_quotes = 1
evidence_max_quotes = 2
evidence_max_quote_chars = 320
auth_mode = "off|static_keys"
# Must exist. Empty array is allowed only when auth_mode = "off".
auth_keys = []

# Required when auth_mode = "static_keys"; replace auth_keys = [] with one or more entries.
# [[security.auth_keys]]
# token_id = "<REQUIRED_ID>"
# token = "<REQUIRED_NON_EMPTY>"
# tenant_id = "<REQUIRED_ID>"
# project_id = "<REQUIRED_ID>"
# agent_id = "<REQUIRED_ID>"
# read_profile = "private_only|private_plus_project|all_scopes"
# role = "user|admin|super_admin"

[context]
# Optional. Context metadata used to disambiguate retrieval across projects and scopes.
#
# project_descriptions keys:
# - "<tenant_id>:<project_id>" (recommended)
# - "<project_id>" (fallback)
project_descriptions = { "<OPTIONAL_KEY>" = "<OPTIONAL_STRING>" }
# scope_descriptions keys are scope labels, e.g. "project_shared".
scope_descriptions = { "<SCOPE>" = "<OPTIONAL_STRING>" }
# Optional. Additive score boost applied when query tokens match a scope description.
# Must be a finite number in the range 0.0-1.0. When greater than zero, scope_descriptions must be present.
scope_boost_weight = <OPTIONAL_FLOAT>

[mcp]
# Optional. Used by elf-mcp to attach required context headers when forwarding to elf-api.
# This section is required when running elf-mcp.
tenant_id = "<REQUIRED_ID>"
project_id = "<REQUIRED_ID>"
agent_id = "<REQUIRED_ID>"
read_profile = "private_only|private_plus_project|all_scopes"

============================================================
2. CLI AND CONFIG LOADING
============================================================
- elf-api, elf-worker, and elf-mcp are separate binaries.
- Each binary requires a config path via --config or -c.
- Startup must fail with a clear error if any required config field is missing.
- security.reject_non_english must be true. Startup must fail if it is false.

============================================================
3. ENGLISH GATE (ENGLISH-ONLY BOUNDARY)
============================================================
Policy:
- ELF is English-only. All externally supplied text fields must be English.
- Translation or multilingual retrieval is out of scope and must be handled upstream.

English gate algorithm (normative):
1) Normalize:
   - Apply Unicode NFKC normalization.
   - Reject if the normalized text contains control characters or zero-width/invisible
     characters (implementation-defined denylist).
2) Script gate (hard reject):
   - Reject if any codepoint is in a disallowed script.
   - Normative allowlist:
     - Allow: Latin, Common, Inherited.
     - Reject: any other script (e.g., Han, Hiragana, Katakana, Hangul, Cyrillic, Arabic).
3) Language identification gate (LID) (conditional reject):
   - Only apply LID to natural-language fields (note text, query, doc text). Do not
     apply LID to structured identifiers (urls, ids, keys) to avoid false rejects.
   - Only apply LID when the input is sufficiently long and letter-dense
     (implementation-defined thresholds).
   - If LID classifies the text as NOT English with confidence >= threshold, reject.
   - If LID is low-confidence/unknown, do not reject (to avoid false positives).

Fields to check:
- add_note: notes[].text, notes[].key (optional), source_ref string fields if any
- add_event: messages[].content
- search: query

Error response:
HTTP 422
{
  "error_code": "NON_ENGLISH_INPUT",
  "message": "Non-English input detected; upstream must canonicalize to English before calling ELF.",
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

4.4 source_ref (evidence pointer)
- source_ref is an optional, versioned pointer to supporting evidence for a stored note.
- Core requirement: ELF Core stores and returns source_ref as an opaque JSON object. Core does not interpret or dereference it.
- When source_ref is provided, it MUST be a JSON object and not a primitive value.
- Extensions requirement: ELF Extensions may define resolvers that can dereference source_ref into bounded excerpts for progressive loading.
- source_ref must be JSON-serializable, ASCII-safe, and stable over time.

Recommended shape (informative):
{
  "schema": "source_ref/v1",
  "resolver": "string",
  "ref": { "...": "resolver-specific" },
  "state": { "...": "optional snapshot/version info" },
  "locator": { "...": "optional in-source excerpt selector(s)" },
  "hashes": { "...": "optional integrity checks" },
  "hints": { "...": "optional debug/UX fields" }
}

Defined resolvers:
- `elf_doc_ext/v1`: Doc Extension v1 document pointer resolver. Defined in `docs/spec/system_source_ref_doc_pointer_v1.md`.

Resolver tiers (informative):
- reproducible: dereference is stable and replayable given (ref + state) (example: fs_git with a commit SHA).
- best_effort: dereference may change over time (example: external conversation thread id); resolvers should expose whether excerpt verification succeeded.

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
- final_score real not null
- explain jsonb not null

Indexes:
- idx_search_trace_items_trace: (trace_id, rank)
- idx_search_trace_items_note: (note_id)

5.10 search_trace_stages (stage-level retrieval trajectory)
- stage_id uuid primary key
- trace_id uuid not null references search_traces(trace_id) on delete cascade
- stage_order int not null
- stage_name text not null
- stage_payload jsonb not null
- created_at timestamptz not null

Indexes:
- idx_search_trace_stages_trace_order: (trace_id, stage_order)
- idx_search_trace_stages_trace_name: (trace_id, stage_name)

5.11 search_trace_stage_items (per-stage item metrics)
- id uuid primary key
- stage_id uuid not null references search_trace_stages(stage_id) on delete cascade
- item_id uuid null
- note_id uuid null
- chunk_id uuid null
- metrics jsonb not null

Indexes:
- idx_search_trace_stage_items_stage_item: (stage_id, item_id)

5.12 search_trace_outbox (async trace persistence)
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

5.13 llm_cache (LLM response cache)
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

5.14 memory_ingest_decisions (ingest policy audit)
- decision_id uuid primary key
- tenant_id text not null
- project_id text not null
- agent_id text not null
- scope text not null
- pipeline text not null
- note_type text not null
- note_key text null
- note_id uuid null
- note_version_id uuid null
- base_decision text not null
- policy_decision text not null
- note_op text not null
- reason_code text null
- details jsonb not null
- ts timestamptz not null

Indexing:
- idx_memory_ingest_decisions_tenant_scope_pipeline: (tenant_id, project_id, agent_id, scope, pipeline, ts)
- idx_memory_ingest_decisions_note_version_id: (note_version_id)

details must include:
- similarity_best
- key_match
- matched_dup
- dup_sim_threshold
- update_sim_threshold
- confidence
- importance
- structured_present
- graph_present
- policy_rule
- min_confidence
- min_importance
- write_policy_audits (add_note: single object, add_event: array of message audits, optional)

5.15 core_memory_blocks (authoritative always-attached context blocks)
- block_id uuid primary key
- tenant_id text not null
- project_id text not null
- agent_id text not null
- scope text not null
- key text not null
- title text not null
- content text not null
- source_ref jsonb not null
- status text not null
- created_at timestamptz not null
- updated_at timestamptz not null

Rules:
- Core blocks are small read-only operating context, separate from archival note search.
- Core blocks must not be indexed into Qdrant or returned by archival search unless a future explicit contract says so.
- source_ref must be a JSON object and is returned with block readback.
- scope, write permission, English gate, auth, and shared-grant rules apply.

Indexes:
- uq_core_memory_blocks_active_key: (tenant_id, project_id, agent_id, scope, key) WHERE status = 'active'
- idx_core_memory_blocks_scope_status: (tenant_id, project_id, scope, status)

5.16 core_memory_block_attachments (explicit block attachment)
- attachment_id uuid primary key
- block_id uuid not null references core_memory_blocks(block_id) on delete cascade
- tenant_id text not null
- project_id text not null
- agent_id text not null
- read_profile text not null
- attached_by_agent_id text not null
- attached_at timestamptz not null
- detached_by_agent_id text null
- detached_at timestamptz null

Rules:
- Active attachment is exact to tenant_id, project_id, agent_id, read_profile, and block_id.
- Attachment does not bypass scope access. Readback still applies read_profile scope resolution,
  private-owner checks, shared grants, and block status.
- Detached rows remain as audit evidence.

Indexes:
- uq_core_memory_block_attachments_active:
  (tenant_id, project_id, agent_id, read_profile, block_id) WHERE detached_at IS NULL
- idx_core_memory_block_attachments_read:
  (tenant_id, project_id, agent_id, read_profile, detached_at)
- idx_core_memory_block_attachments_block: (block_id, detached_at)

5.17 core_memory_block_events (append-only block audit)
- event_id uuid primary key
- block_id uuid not null references core_memory_blocks(block_id) on delete cascade
- attachment_id uuid null references core_memory_block_attachments(attachment_id) on delete set null
- tenant_id text not null
- project_id text not null
- actor_agent_id text not null
- event_type text not null
- target_agent_id text null
- read_profile text null
- prev_snapshot jsonb null
- new_snapshot jsonb null
- reason text not null
- ts timestamptz not null

event_type values:
- block_created
- block_updated
- attachment_added
- attachment_removed

Rules:
- Every block create/update and attachment add/remove writes one event.
- Block readback may include audit history for returned blocks.

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
- Each input message MAY include optional write_policy for per-message redact/exclude policy.
- Must enforce max_notes_per_add_event on the server.
- Must apply WriteGate and UpdateResolver after extraction.
- Should support dry_run to return candidates without persisting.

MUST NOT:
- Must not store notes lacking evidence or failing evidence substring checks.
- Must not store raw full logs as memory notes.
 - If evidence.quote is not a verbatim substring of the cited message, return REJECTED with reason_code REJECT_EVIDENCE_MISMATCH.
 - If write_policy is present and evidence mismatch is a byproduct of transformed content, return REJECTED with reason_code REJECT_WRITE_POLICY_MISMATCH.

8.3 Policy decision pipeline (both add_note and add_event)
Stage-1 (base decision) is computed from resolver outcome + side-effect presence:
- Add -> remember
- Update -> update
- None + (structured_present || graph_present) -> update
- None + (!structured_present && !graph_present) -> ignore

Stage-2 (policy stage) evaluates `memory.policy` rules and may only:
- keep base decision remember/update
- or downgrade remember/update -> ignore when thresholds fail

Decision taxonomy:
- remember
- update
- ignore
- reject

When policy downgrades to ignore:
- `memory_notes` must not be inserted/updated/deleted
- `memory_note_fields` must not be written
- graph memory rows must not be written
- indexing/search outbox rows must not be written
- only an audit row must be written via `memory_ingest_decisions`

Ignore reason codes:
- `IGNORE_DUPLICATE`: base=ignore and duplicate match was detected (`metadata.matched_dup = true`)
- `IGNORE_POLICY_THRESHOLD`: base=remember/update and policy stage threshold/guard downgraded to ignore

============================================================
9. WRITEGATE (SERVER SIDE, ALWAYS ON)
============================================================
Reject a note if any of the following are true:
- The note contains non-English input (fails the English gate).
- The type is not in the 6-type allowlist.
- The scope is not allowed or write not allowed.
- The text length is greater than max_note_chars.
- Secrets or PII are detected (regex and heuristics).
- The text is empty or whitespace only.

On rejection:
- op = REJECTED
- reason_code is one of:
  REJECT_NON_ENGLISH, REJECT_TOO_LONG, REJECT_SECRET, REJECT_INVALID_TYPE,
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
- Worker leases available jobs, inserts search_traces, search_trace_items, search_trace_stages, and search_trace_stage_items, then marks DONE.
- On failure, status = FAILED, attempts += 1, last_error set, available_at = now + backoff(attempts).
- Failures must not affect the original search response.

Periodic cleanup:
- Worker deletes expired search_traces (search_trace_items/search_trace_stages/search_trace_stage_items cascade).
- Worker deletes expired llm_cache rows.

============================================================
13. SEARCH PIPELINE (ONLINE)
============================================================
Input:
- tenant_id, project_id, agent_id
- read_profile
- query (English only)
- mode (`quick_find` or `planned_search`) - required
- optional top_k, candidate_k, filter, record_hits

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
     and the expansion cache schema version (hardcoded), plus max_queries and include_original.
   - If search.cache.enabled and a non-expired cache entry exists, use cached queries.
   - On cache miss, call the LLM expansion prompt and receive queries[].
     - Deduplicate, drop any non-English variants (English gate), and cap at max_queries.
     - Ensure original query is present when include_original = true.
   - If search.cache.enabled and payload size is within max_payload_bytes (when set),
     store the expanded queries with TTL = expansion_ttl_days.
5) Resolve optional project context description:
   - If context.project_descriptions is present, look up by key "tenant_id:project_id".
   - If not found, try key "project_id" as a fallback.
6) For each query, embed -> query_vec (embedding API).
   - Dense embedding input is:
     - query, or
     - query + "\n\nProject context:\n" + project_context_description (when present).
   - BM25 input remains the raw query text (no context suffix).
7) For each query, run Qdrant fusion query candidate_k with payload filters (dense + bm25):
   tenant_id, project_id, status = active (best-effort), and scope filters:
   - If scope = agent_private, require agent_id match.
   - Otherwise scope in allowed_scopes.
   If filter is present, do not push filter criteria into Qdrant.
8) Fuse all query results with RRF to produce candidate chunk_ids.
9) Prefilter (optional): if max_candidates > 0 and max_candidates < candidate_k,
   keep only top max_candidates by fusion score.
10) Fetch authoritative notes from Postgres by note_id and re-apply consistency checks:
   status = active, not expired, scope allowed, and if scope = agent_private then agent_id must match.
11) If filter is present, apply service-side candidate filtering using the authoritative note metadata:
    - effective_candidate_k = min(MAX_CANDIDATE_K, requested_candidate_k * 3), then clamp to >= top_k.
    - The filter is evaluated after candidate retrieval and consistency checks.
    - The filter is not pushed down to Qdrant or SQL.
12) Fetch chunk metadata for candidate chunks and immediate neighbors from memory_note_chunks.
13) Stitch snippets from chunk text (chunk + neighbors).
14) Rerank once using the original query, with cache support:
    - Build a rerank cache key from: query (trimmed), provider_id, model, rerank cache schema version (hardcoded),
      and the candidate signature [(chunk_id, note_updated_at)...].
    - If search.cache.enabled and a cache entry exists that matches the candidate signature,
      reuse cached scores.
    - On cache miss, call the rerank provider:
      scores = rerank(original_query, docs = [snippet ...]).
    - If search.cache.enabled and payload size is within max_payload_bytes (when set),
      store the rerank scores with TTL = rerank_ttl_days.
15) Tie-break:
    base = (1 + 0.6 * importance) * exp(-age_days / recency_tau_days)
    final = rerank_score + tie_breaker_weight * base
16) Optional scope context boost:
    - If context.scope_boost_weight > 0 and context.scope_descriptions contains scope labels,
      apply an additive boost to items in that scope based on query token matches.
    - Token matching uses case-insensitive ASCII alphanumeric tokens (length >= 2).
    - boost = scope_boost_weight * (matched_token_count / query_token_count).
17) Aggregate by note using top-1 chunk score, then sort and take top_k.
18) Update hits (optional, when record_hits is true):
    hit_count++, last_hit_at, memory_hits insert with chunk_id.
19) Build search trace payload with trace_id and per-item result_handle, then enqueue
    search_trace_outbox (best-effort; failures do not fail the search).
    - expires_at = now + search.explain.retention_days.
20) Return results.

Cache notes:
- Cache key material is serialized as JSON and hashed with BLAKE3 (256-bit hex).
- Cache read/write failures are treated as misses and must not fail the search request.

============================================================
14. ADMIN HTTP API (DEBUGGING)
============================================================
Base: http://{service.admin_bind}

Note: Admin endpoints are intended for localhost use only. They are not exposed on the public bind.

Authentication:
- security.auth_mode = "off": no auth header is required.
- security.auth_mode = "static_keys": admin requests must include `Authorization: Bearer <token>`.
- In `static_keys` mode, the matched `security.auth_keys` entry must have `admin = true` for admin endpoints.

Request correlation:
- `X-ELF-Request-Id` is optional on admin endpoints.
- If omitted, elf-api generates a new UUID.
- Response includes `X-ELF-Request-Id` header and `request_id` in JSON responses.

GET /viewer

Behavior:
- Serves the local read-only web viewer from the admin bind only.
- Must not be mounted on the public HTTP bind by default.
- The viewer uses admin-bind same-origin requests and only calls read-only endpoints.
- In `static_keys` mode, the viewer page may load without credentials, but data requests still require an admin bearer token.

Admin read-only session mirror:
- POST /v2/admin/searches
- GET /v2/admin/searches/{search_id}
- GET /v2/admin/searches/{search_id}/timeline
- POST /v2/admin/searches/{search_id}/notes

Behavior:
- These endpoints mirror the public progressive search session endpoints for local admin viewer use.
- They are read-only with respect to notes; detail hydration must default to `record_hits = false` when the viewer calls it.
- They require the same context headers as the public session endpoints, plus admin authentication when `security.auth_mode = "static_keys"`.

Admin read-only note mirror:
- GET /v2/admin/notes
- GET /v2/admin/notes/{note_id}

Behavior:
- These endpoints mirror the public note list/detail reads for local admin viewer use.
- Note metadata that includes `created_at`, `hit_count`, and `last_hit_at` is available through `GET /v2/admin/notes/{note_id}/provenance`.

Admin core memory block management:
- POST /v2/admin/core-blocks
- POST /v2/admin/core-blocks/{block_id}/attachments
- DELETE /v2/admin/core-blocks/attachments/{attachment_id}

Behavior:
- These endpoints create/update core blocks and attach/detach them for exact tenant/project/agent/read_profile readback.
- Core blocks are read-only to normal public callers; public callers only read attached blocks.
- Mutations write append-only `core_memory_block_events`.
- Core blocks are not note-search hits and do not write Qdrant points, search sessions, search traces, or note outbox rows.

Admin consolidation proposal review:
- POST /v2/admin/consolidation/runs
- GET /v2/admin/consolidation/runs
- GET /v2/admin/consolidation/runs/{run_id}
- GET /v2/admin/consolidation/proposals
- GET /v2/admin/consolidation/proposals/{proposal_id}
- POST /v2/admin/consolidation/proposals/{proposal_id}/review
- GET /v2/admin/dreaming/review-queue

Behavior:
- These endpoints expose fixture-driven or manually supplied consolidation runs and
  reviewable derived proposals.
- Creating a consolidation run enqueues a deterministic `consolidation_run_jobs`
  worker job and returns `job_id`; the worker materializes supplied proposal payloads
  into `consolidation_proposals`.
- Proposal payloads must follow `elf.consolidation/v1`, carry source refs and
  snapshots, and may include unsupported-claim flags, contradiction markers, and
  staleness markers for reviewer inspection.
- Review action values are `approve`, `apply`, `discard`, and `defer`.
- `apply` records an approval transition before the applied transition when a proposal
  starts from `proposed`.
- For `create_derived_note` and `update_derived_note`, `apply` promotes the reviewed
  proposal into an approved operational memory note. The promoted note source ref
  must use `elf.memory_promotion/v1`, include the proposal source refs and review
  actor/timestamp, write `memory_note_versions`, enqueue `UPSERT`, and update the
  proposal `target_ref` to `elf.memory_record_ref/v1` with `kind = "note"`.
- Every review action writes append-only review audit events returned by proposal
  detail readback.
- `GET /v2/admin/dreaming/review-queue` exposes
  `elf.dreaming_review_queue/v1`, a read-only policy view over consolidation
  proposals for Dreaming variants such as memory summaries, proactive briefs,
  scheduled memories, tags, duplicate merges, page rebuilds, memory promotions,
  graph facts, and corrections.
- Dreaming queue items must include source refs, affected refs, confidence,
  unsupported-claim lint, diff, policy, and review audit. The queue must report
  `source_mutation_allowed = false`; low-risk derived organization auto-apply is
  limited to approved tag or duplicate-merge candidates with no lint or source
  mutation request.
- These endpoints must not call LLM, embedding, rerank, or external provider adapters.
- They must not mutate raw source notes, docs, events, traces, graph facts, or search
  traces. Applied note proposals may create or update approved operational memory
  records only through the explicit review path above.

Admin memory correction loop:
- POST /v2/admin/notes/{note_id}/corrections

Body:
{
  "action": "supersede|delete|restore",
  "reason": "non-empty reviewer or policy reason",
  "source_ref": { "...": "non-empty source or review reference" },
  "restore_version_id": "uuid|null"
}

Behavior:
- `supersede` sets the note status to `deprecated`, writes a `DEPRECATE`
  `memory_note_versions` row, stores `elf.memory_correction/v1` source-ref evidence,
  and enqueues an indexing `DELETE` so normal recall suppresses the note.
- `delete` sets the note status to `deleted`, writes a `DELETE`
  `memory_note_versions` row, stores `elf.memory_correction/v1` source-ref evidence,
  and enqueues an indexing `DELETE`.
- `restore` restores the latest prior active snapshot from a `DELETE` or `DEPRECATE`
  version, or the supplied `restore_version_id`, writes a `RESTORE`
  `memory_note_versions` row, stores `elf.memory_correction/v1` source-ref evidence,
  and enqueues an indexing `UPSERT`.
- Correction actions require a non-empty reason and non-empty JSON object
  `source_ref`. They do not mutate raw source notes, docs, events, traces, graph
  facts, or source pointers.
- Normal recall remains active-only; `deprecated` and `deleted` notes are visible
  through provenance/history or explicit non-active list filters, not ordinary search.

Recall/debug panel:
- POST /v2/recall-debug/panel
- POST /v2/admin/recall-debug/panel

Behavior:
- The endpoints return `elf.recall_debug_panel/v1`, a read-only cross-layer panel
  over Memory Note trace bundles, Source Library document search, Knowledge Workspace
  page search, graph reports, and Dreaming review queue proposals.
- The public route is the agent-facing recall/debug API. The admin route is an
  operator mirror over the same service read model.
- Each row must expose selection state, authority layer, freshness state, source refs
  or source snapshots, score/rank where available, stage reason, evidence class, and
  replay command or deterministic artifact path when available.
- Responses must include `recall_trace` with schema `elf.recall_trace/v1`: a compact
  deterministic projection over selected, dropped, stale, blocked, and not-requested
  context for agent and fixture/report assertions.
- Missing anchors must be represented as `not_requested` layers. The panel must not
  collapse not-requested, incomplete, blocked, or wrong-result layers into a broad
  pass claim.
- Requested layer failures must be represented as blocked layer evidence, so one
  unavailable readback surface does not hide the other layer states.
- The detailed contract is defined in `system_recall_debug_panel_v1.md`.

Admin derived knowledge pages:
- POST /v2/admin/knowledge/pages/rebuild
- GET /v2/admin/knowledge/pages
- POST /v2/admin/knowledge/pages/search
- GET /v2/admin/knowledge/pages/{page_id}
- POST /v2/admin/knowledge/pages/{page_id}/lint

Behavior:
- These endpoints expose deterministic rebuild, list/detail readback, and stale-source
  lint for derived knowledge pages. The search endpoint exposes derived page section
  snippets with visible citations, source coverage, lint summary, trust state, and
  repair/rebuild guidance.
- Page payloads must follow `elf.knowledge_page/v1`, preserve section citations, and
  write normalized source refs for lint.
- Pages are derived and rebuildable; rebuilding or linting a page must not mutate
  authoritative notes, event audits, graph facts, consolidation proposals, docs,
  traces, or source pointers.
- Page snippets are not authoritative note search hits and must be labeled as derived
  knowledge page snippets wherever surfaced.
- The detailed contract is defined in `system_knowledge_pages_v1.md`.

Admin reviewable memory summary readback:

Behavior:
- Memory summary readback is a derived, reviewable artifact surface, not
  authoritative note search and not a hidden note rewrite path.
- Summary entries must follow `elf.memory_summary/v1`, carry source refs, freshness or
  validity metadata, and inclusion/downgrade/exclusion rationale for top-of-mind,
  background, stale, superseded, tombstoned, and derived project-profile entries.
- Stale, superseded, or tombstoned entries must not be returned as current
  top-of-mind facts.
- Derived project-profile entries must either cite source refs or carry explicit
  unsupported-claim flags when excluded.
- Memory summaries must not call provider adapters, mutate authoritative source notes,
  create Qdrant points, create search sessions, or record note hits in v1 contract
  validation.
- The detailed contract is defined in `system_memory_summary_v1.md`.

POST /v2/admin/qdrant/rebuild

Behavior:
- Rebuild the Qdrant chunk index from Postgres chunk vectors.
- Must not call the embedding API.
- Qdrant is derived and can be dropped and recreated at any time.

Response:
{
  "rebuilt_count": 0,
  "missing_vector_count": 0,
  "error_count": 0
}

POST /v2/admin/searches/raw

Headers:
- X-ELF-Tenant-Id (required)
- X-ELF-Project-Id (required)
- X-ELF-Agent-Id (required)
- X-ELF-Read-Profile (required): private_only|private_plus_project|all_scopes

Body:
{
  "query": "English-only",
  "mode": "quick_find",
  "top_k": 12,
  "candidate_k": 60,
  "payload_level": "l0",
  "filter": {
    "schema": "search_filter_expr/v1",
    "expr": {
      "op": "gte",
      "field": "importance",
      "value": 0.5
    }
  }
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
      "type": "fact|plan|preference|constraint|decision|profile",
      "key": null,
      "scope": "agent_private|project_shared|org_shared",
      "importance": 0.0,
      "confidence": 0.0,
      "updated_at": "...",
      "expires_at": "...|null",
      "final_score": 0.0,
      "source_ref": { ... },
      "explain": {
        "match": {
          "matched_terms": ["..."],
          "matched_fields": ["text", "key"]
        },
        "ranking": {
          "schema": "search_ranking_explain/v2",
          "policy_id": "ranking_v2:...",
          "final_score": 0.0,
          "terms": [
            { "name": "blend.retrieval", "value": 0.0 },
            { "name": "blend.rerank", "value": 0.0 },
            { "name": "tie_breaker", "value": 0.0 },
            { "name": "context.scope_boost", "value": 0.0 },
            { "name": "deterministic.lexical_bonus", "value": 0.0 },
            { "name": "deterministic.hit_boost", "value": 0.0 },
            { "name": "deterministic.decay_penalty", "value": 0.0 }
          ]
        },
        "relation_context": [
          {
            "fact_id": "uuid",
            "scope": "project_shared",
            "subject": { "canonical": "string", "kind": "person|concept|null" },
            "predicate": "string",
            "object": {
              "entity": { "canonical": "string", "kind": "person|concept|null" },
              "value": null
            },
            "valid_from": "...",
            "valid_to": null,
            "temporal_status": "current|historical|future",
            "evidence_note_ids": ["uuid", "uuid"]
          }
        ]
        }
      }
    }
  ]
}

Notes:
- `relation_context` is omitted unless `search.graph_context.enabled` is true.
- When present, relation context is evidence-bound and bounded by `search.graph_context.max_facts_per_item` and
  `search.graph_context.max_evidence_notes_per_fact`.
- `relation_context.temporal_status` is derived from the graph fact validity window at the search read timestamp.
  Historical facts may be returned when they are evidence-linked to a selected note; they must be labeled
  `historical` instead of being presented as current.
- It is included wherever `SearchExplain` is returned, including admin trace surfaces (`/v2/admin/traces/*` and
  `/v2/admin/trace-items/*`), in addition to search responses.
- Admin trace endpoints validate `tenant_id` + `project_id` only for access control. They are intended for
  project-scoped operations and do not require the requesting `agent_id` to match the stored trace owner.
- This endpoint is intended for debugging and evaluation. It returns chunk-level items and explain components.
- The public search endpoint returns a compact note-level index view.

GET /v2/admin/traces/recent

Headers:
- X-ELF-Tenant-Id (required)
- X-ELF-Project-Id (required)
- X-ELF-Agent-Id (required)

Query:
- limit (optional): default `50`, max `200`.
- cursor_created_at (optional, RFC3339): timestamp cursor value.
- cursor_trace_id (optional, uuid): cursor trace id.
- agent_id (optional): filter traces by creator.
- read_profile (optional): filter by read_profile.
- created_after (optional, RFC3339): strict lower bound on `created_at`.
- created_before (optional, RFC3339): strict upper bound on `created_at`.

Requirements:
- `cursor_created_at` and `cursor_trace_id` must be provided together or omitted together.

Response:
{
  "schema": "elf.recent_traces/v1",
  "traces": [
    {
      "trace_id": "uuid",
      "tenant_id": "string",
      "project_id": "string",
      "agent_id": "string",
      "read_profile": "private_only|private_plus_project|all_scopes",
      "query": "string",
      "created_at": "..."
    }
  ],
  "next_cursor": {
    "created_at": "...",
    "trace_id": "uuid"
  } | null
}

Ordering:
- `created_at DESC`, then `trace_id DESC`.
- The page cursor for the next page uses `(created_at, trace_id) < cursor`.

GET /v2/admin/traces/{trace_id}/bundle

Headers:
- X-ELF-Tenant-Id (required)
- X-ELF-Project-Id (required)
- X-ELF-Agent-Id (required)

Query:
- mode: `bounded` (default) or `full`.
- stage_items_limit (optional): max items per trajectory stage.
- candidates_limit (optional): max candidate count for `candidates`.

Response:
{
  "schema": "elf.trace_bundle/v1",
  "generated_at": "...",
  "trace": { ... },
  "items": [ ... ],
  "trajectory_summary": {
    "schema": "search_retrieval_trajectory/v1",
    "stages": [ ... ]
  } | null,
  "stages": [ ... ],
  "candidates": [ ... ] | null
}
- `stage_items_limit`: `64` in `bounded` mode (cap `256`), `256` in `full` mode.
- `candidates_limit`: `0` in `bounded` mode (no candidates), `200` in `full` mode.
- Candidate snapshot is decoded to `TraceReplayCandidate`.
- `candidates` is omitted as `null` when not requested.

GET /v2/admin/traces/{trace_id}

Headers:
- X-ELF-Tenant-Id (required)
- X-ELF-Project-Id (required)
- X-ELF-Agent-Id (required)

Response:
{
  "trace": { ... },
  "items": [ ... ],
  "trajectory_summary": {
    "schema": "search_retrieval_trajectory/v1",
    "stages": [ ... ]
  }
}
`items[*].explain` follows the same `SearchExplain` schema as search responses (including optional `relation_context`).

GET /v2/admin/trajectories/{trace_id}

Headers:
- X-ELF-Tenant-Id (required)
- X-ELF-Project-Id (required)
- X-ELF-Agent-Id (required)

Response:
{
  "trace": { ... },
  "trajectory": {
    "schema": "search_retrieval_trajectory/v1",
    "stages": [ ... ]
  },
  "stages": [
    {
      "stage_order": 1,
      "stage_name": "rewrite.expansion",
      "stage_payload": { ... },
      "items": [ ... ]
    }
  ]
}

GET /v2/admin/trace-items/{item_id}

Headers:
- X-ELF-Tenant-Id (required)
- X-ELF-Project-Id (required)
- X-ELF-Agent-Id (required)

Response:
{
  "trace": { ... },
  "item": { ... },
  "trajectory": {
    "schema": "search_retrieval_trajectory/v1",
    "stages": [ ... ]
  }
}
`item.explain` follows the same `SearchExplain` schema as search responses (including optional `relation_context`).

GET /v2/admin/graph/predicates?scope=...

Headers:
- X-ELF-Tenant-Id (required)
- X-ELF-Project-Id (required)
- X-ELF-Agent-Id (required)

Query:
- scope (optional): tenant_project|project|global|all (default: all)

Response:
{
  "predicates": [
    {
      "predicate_id": "uuid",
      "scope_key": "string",
      "tenant_id": "string|null",
      "project_id": "string|null",
      "canonical": "string",
      "canonical_norm": "string",
      "cardinality": "single|multi",
      "status": "pending|active|deprecated",
      "created_at": "...",
      "updated_at": "..."
    }
  ]
}

PATCH /v2/admin/graph/predicates/{predicate_id}

Headers:
- X-ELF-Tenant-Id (required)
- X-ELF-Project-Id (required)
- X-ELF-Agent-Id (required)

Body:
{
  "status": "pending|active|deprecated|null",
  "cardinality": "single|multi|null"
}

Behavior:
- At least one of status or cardinality is required.
- Allowed status transitions: pending->active, pending->deprecated, active->deprecated.
- Deprecated predicates cannot be modified (409).
- Global predicates are immutable (403).
- Note: Global predicate mutations remain follow-up work and are not covered by this contract.

Response:
{
  "predicate_id": "uuid",
  "scope_key": "string",
  "tenant_id": "string|null",
  "project_id": "string|null",
  "canonical": "string",
  "canonical_norm": "string",
  "cardinality": "single|multi",
  "status": "pending|active|deprecated",
  "created_at": "...",
  "updated_at": "..."
}

POST /v2/admin/graph/predicates/{predicate_id}/aliases

Headers:
- X-ELF-Tenant-Id (required)
- X-ELF-Project-Id (required)
- X-ELF-Agent-Id (required)

Body:
{
  "alias": "string"
}

Behavior:
- alias must be non-empty.
- Deprecated predicates cannot be modified (409).
- Global predicates are immutable (403).
- Note: Global predicate mutations remain follow-up work and are not covered by this contract.

Response:
{
  "predicate_id": "uuid",
  "aliases": [
    {
      "alias_id": "uuid",
      "predicate_id": "uuid",
      "scope_key": "string",
      "alias": "string",
      "alias_norm": "string",
      "created_at": "..."
    }
  ]
}

GET /v2/admin/graph/predicates/{predicate_id}/aliases

Headers:
- X-ELF-Tenant-Id (required)
- X-ELF-Project-Id (required)
- X-ELF-Agent-Id (required)

Response:
{
  "predicate_id": "uuid",
  "aliases": [
    {
      "alias_id": "uuid",
      "predicate_id": "uuid",
      "scope_key": "string",
      "alias": "string",
      "alias_norm": "string",
      "created_at": "..."
    }
  ]
}

GET /v2/admin/notes/{note_id}/provenance

Headers:
- X-ELF-Tenant-Id (required)
- X-ELF-Project-Id (required)
- X-ELF-Agent-Id (required)

Path:
- note_id: uuid

Response:
{
  "schema": "elf.note_provenance_bundle/v1",
  "note": { ... },
  "ingest_decisions": [...],
  "note_versions": [...],
  "indexing_outbox": [...],
  "recent_traces": [...]
}

GET /v2/admin/notes/{note_id}/history

Headers:
- X-ELF-Tenant-Id (required)
- X-ELF-Project-Id (required)
- X-ELF-Agent-Id (required)

Path:
- note_id: uuid

Response:
{
  "schema": "elf.memory_history/v1",
  "note_id": "uuid",
  "events": [
    {
      "event_id": "string",
      "event_type": "add|update|ignore|reject|expire|delete|derived|applied|superseded|restored|defer|invalidated|related",
      "subject_type": "note",
      "note_id": "uuid",
      "source_table": "string",
      "source_id": "uuid|null",
      "related_note_version_id": "uuid|null",
      "related_decision_id": "uuid|null",
      "related_proposal_id": "uuid|null",
      "actor": "string|null",
      "op": "string|null",
      "reason_code": "string|null",
      "summary": "string",
      "details": { ... },
      "ts": "..."
    }
  ]
}

Notes:
- History events are a chronological read-only projection over durable source tables.
- Ingest decisions that produce note versions should set `note_version_id` so history
  can link the decision to the resulting note version.
- Derived, applied, rejected, and deferred events come from consolidation proposals
  and review events that reference the note in `source_refs` or the proposal
  `target_ref`.
- Superseded, deleted, and restored events come from correction-backed note version
  rows.

============================================================
15. HTTP API (PUBLIC)
============================================================
Base: http://{service.http_bind}

All /v2 endpoints except GET /health require context headers:
- X-ELF-Tenant-Id (required)
- X-ELF-Project-Id (required)
- X-ELF-Agent-Id (required)

Request correlation:
- `X-ELF-Request-Id` is optional on public endpoints.
- If omitted, elf-api generates a new UUID.
- Response includes `X-ELF-Request-Id` header and `request_id` in JSON responses.

Search creation and graph query endpoints also require:
- X-ELF-Read-Profile (required): private_only|private_plus_project|all_scopes

Header rules:
- Headers must be valid UTF-8 strings.
- Headers must be non-empty and at most 128 characters.
- Headers must pass the English identifier gate (no non-Latin scripts, no zero-width/control characters).

Authentication:
- security.auth_mode = "off": no auth header is required.
- security.auth_mode = "static_keys": requests must include `Authorization: Bearer <token>`, matched against `security.auth_keys`.

POST /v2/notes/ingest

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Body:
{
  "scope": "agent_private|project_shared|org_shared",
  "notes": [
    {
      "type": "preference|constraint|decision|profile|fact|plan",
      "key": "string|null",
      "text": "English-only sentence",
      "importance": 0.0,
      "confidence": 0.0,
      "ttl_days": 180,
      "write_policy": "optional",
      "structured": {
        "summary": "string|null",
        "facts": "string[]|null",
        "concepts": "string[]|null",
        "entities": [
          {
            "canonical": "string|null",
            "kind": "string|null",
            "aliases": "string[]|null"
          }
        ]|null,
        "relations": [
          {
            "subject": {
              "canonical": "string|null",
              "kind": "string|null",
              "aliases": "string[]|null"
            },
            "predicate": "string",
            "object": {
              "entity": {
                "canonical": "string|null",
                "kind": "string|null",
                "aliases": "string[]|null"
              }|null,
              "value": "string|null"
            },
            "valid_from": "ISO8601 datetime|null",
            "valid_to": "ISO8601 datetime|null"
          }
        ]|null
      }|null,
      "source_ref": { ... }
    }
  ]
}

Notes:
- Exactly one of object.entity and object.value must be non-null.

Response:
{
  "results": [
    {
      "note_id": "uuid|null",
      "op": "ADD|UPDATE|NONE|DELETE|REJECTED",
      "policy_decision": "remember|update|ignore|reject",
      "reason_code": "optional",
      "field_path": "optional"
    }
  ]
}

Notes:
- This endpoint is deterministic and must not call any LLM.

POST /v2/events/ingest

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Body:
{
  "scope": "optional-scope",
  "dry_run": false,
  "ingestion_profile": {
    "id": "default",
    "version": 1
  },
  "messages": [
    {
      "role": "user|assistant|tool",
      "content": "English-only",
      "ts": "optional",
      "msg_id": "optional",
      "write_policy": "optional"
    }
  ]
}

Response:
{
  "ingestion_profile": {
    "id": "string",
    "version": 1
  },
  "extracted": { ...extractor output... },
  "results": [
    {
      "note_id": "uuid|null",
      "op": "ADD|UPDATE|NONE|DELETE|REJECTED",
      "policy_decision": "remember|update|ignore|reject",
      "reason_code": "optional",
      "reason": "optional",
      "field_path": "optional",
      "write_policy_audits": [
        {
          "exclusions": [{ "start": 0, "end": 4 }],
          "redactions": [{ "span": { "start": 0, "end": 4 }, "replacement": "***" }]
        }
      ]
    }
  ]
}

Notes:
- reason_code values include writegate rejection codes, REJECT_EVIDENCE_MISMATCH, and REJECT_WRITE_POLICY_MISMATCH.
- `ingestion_profile.id` is required when profile override is provided, and when `version` is omitted, latest version for that id is used.
- If `ingestion_profile` is omitted, the tenant/project default profile is used.

GET /v2/admin/events/ingestion-profiles

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Response:
{
  "profiles": [
    {
      "profile_id": "string",
      "version": 1,
      "created_at": "...",
      "created_by": "agent_id"
    }
  ]
}

POST /v2/admin/events/ingestion-profiles

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Body:
{
  "profile_id": "string",
  "version": 1,
  "profile": {},
  "created_by": "agent_id"
}

Response:
{
  "profile_id": "string",
  "version": 1,
  "profile": { ... },
  "created_at": "...",
  "created_by": "agent_id"
}

GET /v2/admin/events/ingestion-profiles/{profile_id}?version=1

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Query:
- version (optional)

Response:
{
  "profile_id": "string",
  "version": 1,
  "profile": { ... },
  "created_at": "...",
  "created_by": "agent_id"
}

GET /v2/admin/events/ingestion-profiles/{profile_id}/versions

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Response:
{
  "profiles": [
    {
      "profile_id": "string",
      "version": 1,
      "created_at": "...",
      "created_by": "agent_id"
    }
  ]
}

GET /v2/admin/events/ingestion-profiles/default

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Response:
{
  "profile_id": "string",
  "version": 1,
  "updated_at": "..."
}

PUT /v2/admin/events/ingestion-profiles/default

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Body:
{
  "profile_id": "string",
  "version": 1
}

Response:
{
  "profile_id": "string",
  "version": 1,
  "updated_at": "..."
}

POST /v2/graph/query

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id
- X-ELF-Read-Profile

Body:
{
  "subject": { "entity_id": "uuid" } | { "surface": "string" },
  "predicate": { "predicate_id": "uuid" } | { "surface": "string" } | null,
  "scopes": ["agent_private|project_shared|org_shared"] | null,
  "as_of": "RFC3339 datetime|null",
  "limit": 50,
  "explain": false
}

Response:
{
  "as_of": "...",
  "subject": {
    "entity_id": "uuid",
    "canonical": "string",
    "kind": "string|null"
  },
  "predicate": {
    "predicate_id": "uuid",
    "canonical": "string"
  } | null,
  "scopes": ["agent_private|project_shared|org_shared"],
  "truncated": false,
  "facts": [
    {
      "fact_id": "uuid",
      "scope": "agent_private|project_shared|org_shared",
      "actor": "agent_id",
      "predicate": "string",
      "predicate_id": "uuid|null",
      "valid_from": "...",
      "valid_to": "...|null",
      "temporal_status": "current|historical|future",
      "object": {
        "entity": {
          "entity_id": "uuid",
          "canonical": "string",
          "kind": "string|null"
        } | null,
        "value": "string|null"
      },
      "evidence_note_ids": ["uuid"]
    }
  ],
  "explain": {
    "schema": "elf.graph_query/v1",
    "as_of": "...",
    "requested_limit": 50,
    "allowed_scopes": ["..."],
    "effective_scopes": ["..."],
    "queried_rows": 51,
    "returned_rows": 50,
    "truncated": true
  } | null
}

Notes:
- `subject` is required and accepts exactly one lookup shape: `entity_id` or `surface`.
- `predicate` is optional; when omitted, matching facts across predicates are eligible.
- `X-ELF-Read-Profile` is required and gates readable scopes via `[scopes.read_profiles]`.
- `scopes` is optional. If omitted, the endpoint uses all scopes allowed by `read_profile`. If provided, each scope must be allowed by `read_profile`.
- Shared scopes still apply grant checks; unreadable shared facts are not returned.
- `limit` defaults to 50 and must be in the range 1..200.
- `truncated = true` means additional facts matched but were clipped by `limit`.
- `evidence_note_ids` is ordered by evidence creation time and capped to 16 IDs per fact.
- `explain` defaults to false; when true, response includes `explain.schema = "elf.graph_query/v1"`.

GET /v2/core-blocks

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id
- X-ELF-Read-Profile

Response:
{
  "schema": "elf.core_memory_blocks/v1",
  "tenant_id": "string",
  "project_id": "string",
  "agent_id": "string",
  "read_profile": "private_only|private_plus_project|all_scopes",
  "items": [
    {
      "block_id": "uuid",
      "attachment_id": "uuid",
      "tenant_id": "string",
      "project_id": "string",
      "agent_id": "block-owner-agent",
      "scope": "agent_private|project_shared|org_shared",
      "key": "string",
      "title": "string",
      "content": "small English operating context",
      "source_ref": { ... },
      "status": "active",
      "updated_at": "...",
      "attached_at": "...",
      "attached_by_agent_id": "string",
      "audit_history": [ ... ]
    }
  ]
}

Notes:
- This endpoint is not archival search. It does not embed, rerank, search Qdrant,
  create a search session, or record note hits.
- A block is returned only when it has an active attachment for the exact
  tenant/project/agent/read_profile and the block is readable under that read_profile's
  scopes and shared grants.

GET /v2/entity-memory

Headers:
- X-ELF-Tenant-Id (required)
- X-ELF-Project-Id (required)
- X-ELF-Agent-Id (required)
- X-ELF-Read-Profile (required)

Query:
- entity_id: uuid, optional.
- entity_surface: string, optional canonical or alias surface.
- Exactly one of entity_id or entity_surface is required.

Response:
{
  "schema": "elf.entity_memory_view/v1",
  "tenant_id": "string",
  "project_id": "string",
  "agent_id": "requesting-agent",
  "read_profile": "private_only|private_plus_project|all_scopes",
  "as_of": "...",
  "entity": {
    "entity_id": "uuid",
    "canonical": "Alice",
    "kind": "person|null",
    "surfaces": ["Alice", "Alicia"]
  },
  "summary": {
    "current_count": 0,
    "stale_count": 0,
    "superseded_count": 0,
    "tombstoned_count": 0,
    "top_of_mind_count": 0,
    "background_count": 0,
    "core_block_count": 0,
    "archival_note_count": 0
  },
  "items": [
    {
      "source": "core_block|archival_note",
      "lifecycle": "current|stale|superseded|tombstoned",
      "read_bucket": "top_of_mind|background",
      "scope": "agent_private|project_shared|org_shared",
      "agent_id": "source-owner-agent",
      "note_id": "uuid|null",
      "block_id": "uuid|null",
      "attachment_id": "uuid|null",
      "note_type": "string|null",
      "key": "string|null",
      "title": "string|null",
      "text": "string",
      "importance": 0.0,
      "confidence": 0.0,
      "source_ref": { ... },
      "updated_at": "...",
      "expires_at": "...|null",
      "relations": [
        {
          "fact_id": "uuid",
          "predicate": "prefers",
          "scope": "agent_private|project_shared|org_shared",
          "actor": "fact-owner-agent",
          "valid_from": "...",
          "valid_to": "...|null",
          "temporal_status": "current|historical|future"
        }
      ]
    }
  ]
}

Behavior:
- The endpoint resolves a graph entity by id or canonical/alias surface within the request tenant/project.
- It returns graph evidence notes linked through `graph_facts` and `graph_fact_evidence`, including stale, superseded, and tombstoned lifecycle buckets for authority readback.
- It also returns attached core blocks for the exact tenant/project/agent/read_profile when block key/title/content/source_ref mentions the canonical entity or one of its aliases.
- Read access is still governed by read_profile scopes and shared grants. `agent_private` rows are visible only to their owning agent.
- Core blocks are classified as `current` and `top_of_mind`; archival notes are `top_of_mind` only when they are current and importance is at least 0.8.
- This endpoint is read-only. It does not embed, rerank, mutate notes or blocks, create search sessions, write Qdrant points, or record note hits.

POST /v2/searches

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id
- X-ELF-Read-Profile

Body:
{
  "mode": "quick_find",
  "query": "English-only",
  "top_k": 12,
  "candidate_k": 60,
  "payload_level": "l0",
  "filter": {
    "schema": "search_filter_expr/v1",
    "expr": {
      "op": "and",
      "args": [
        { "op": "eq", "field": "scope", "value": "project_shared" },
        { "op": "gte", "field": "importance", "value": 0.5 }
      ]
    }
  }
}

Response:
{
  "mode": "quick_find",
  "trace_id": "uuid",
  "search_id": "uuid",
  "expires_at": "...",
  "trajectory_summary": {
    "schema": "search_retrieval_trajectory/v1",
    "stages": [ ... ]
  } | null,
  "items": [
    {
      "note_id": "uuid",
      "type": "...",
      "key": null,
      "scope": "...",
      "importance": 0.0,
      "confidence": 0.0,
      "updated_at": "...",
      "expires_at": "...|null",
      "final_score": 0.0,
      "summary": "..."
    }
  ]
}

Notes:
- This endpoint creates a search session and returns a compact note index view.
- `trajectory_summary` is optional and includes staged retrieval trajectory metadata via `search_retrieval_trajectory/v1`, with `stages` only containing summary-level stats per stage (e.g., counts/timing); it intentionally excludes full stage internals.
- `mode` is required and controls how much planning/latency tradeoff the query uses: `quick_find` for lower-latency paths, `planned_search` for planning-focused retrieval.
- `query_plan` is included only when `mode` is `planned_search`.
- record_hits is always false for this endpoint.
- `payload_level` is optional and defaults to `l0`.
- This endpoint does not return full note text; use `/v2/searches/{search_id}/notes` for progressive note hydration.

GET /v2/searches/{search_id}?top_k=12&touch=true

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Query parameters:
- top_k (optional): Override the number of items returned.
- touch (optional, default true): When true, extend the search session TTL.
- payload_level (optional, default l0): Accepted for request parity with note-detail shaping.

Response: Same as POST /v2/searches.

GET /v2/searches/{search_id}/timeline?group_by=day

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Query parameters:
- group_by (optional, default day): day|none
- payload_level (optional, default l0): if `group_by` is omitted, this endpoint defaults to `none` for l0 and `day` for other levels.

Response:
{
  "search_id": "uuid",
  "expires_at": "...",
  "groups": [
    { "date": "YYYY-MM-DD|all", "items": [ ... ] }
  ]
}

Notes:
- This endpoint touches the search session and extends its TTL.

POST /v2/searches/{search_id}/notes

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Body:
{
  "note_ids": ["uuid"],
  "payload_level": "l0",
  "record_hits": true
}

Response:
{
  "search_id": "uuid",
  "expires_at": "...",
  "results": [
    {
      "note_id": "uuid",
      "note": { ...full note... },
      "error": null
    }
  ]
}

Notes:
- record_hits defaults to true when omitted.
- This endpoint touches the search session and extends its TTL.

Payload-level semantics for search note details:

| payload_level | `searches/{search_id}/notes`.text | `searches/{search_id}/notes`.structured | `searches/{search_id}/notes`.source_ref | `/admin/searches/raw`.source_ref |
| --- | --- | --- | --- | --- |
| l0 | compact summary (bounded by `max_note_chars`) | `null` | `{}` | `{}` |
| l1 | compact summary (structured summary if available, else compact text) | object | `{}` | `{}` |
| l2 | full text | object | full object | full object |

Notes:
- Omitted `payload_level` defaults to `l0` on both `/v2/searches/{search_id}/notes` and `/v2/admin/searches/raw`.

GET /v2/notes?scope=project_shared&status=active&type=fact

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Notes:
- If scope is omitted, agent_private notes are excluded.
- If scope is agent_private, the calling agent_id is required and enforced.

GET /v2/notes/{note_id}

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

PATCH /v2/notes/{note_id}

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Body:
{
  "text": "optional",
  "importance": 0.0,
  "confidence": 0.0,
  "ttl_days": 180
}

Response:
{
  "note_id": "uuid",
  "op": "ADD|UPDATE|NONE|DELETE|REJECTED",
  "reason_code": "optional"
}

DELETE /v2/notes/{note_id}

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Response:
{
  "note_id": "uuid",
  "op": "ADD|UPDATE|NONE|DELETE|REJECTED"
}

Notes:
- Shared scopes (`project_shared`, `org_shared`) are not implicitly readable by other agents.
- Access to a shared note requires an explicit `memory_space_grants` entry for the requesting agent/project.
- `team_shared` is the public API alias for internal `project_shared`.

POST /v2/notes/{note_id}/publish

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Body:
{
  "space": "team_shared|org_shared"
}

Response:
{
  "note_id": "uuid",
  "space": "team_shared|org_shared"
}

Behavior:
- Publishing a private note to `team_shared` changes visibility to shared scope and creates a project-wide grant so all agents in the same project can read the note when requested explicitly from shared scope.

POST /v2/notes/{note_id}/unpublish

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Body:
{
  "space": "team_shared|org_shared"
}

Response:
{
  "note_id": "uuid",
  "space": "agent_private"
}

GET /v2/spaces/{space}/grants

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Path:
- space: team_shared|org_shared

Response:
{
  "grants": [
    {
      "space": "team_shared|org_shared",
      "grantee_kind": "project|agent",
      "grantee_agent_id": null,
      "granted_by_agent_id": "agent_id",
      "granted_at": "..."
    }
  ]
}

POST /v2/spaces/{space}/grants

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Path:
- space: team_shared|org_shared

Body:
{
  "grantee_kind": "project|agent",
  "grantee_agent_id": "optional-agent-id"
}

Response:
{
  "space": "team_shared|org_shared",
  "grantee_kind": "project|agent",
  "grantee_agent_id": null,
  "granted": true
}

POST /v2/spaces/{space}/grants/revoke

Headers:
- X-ELF-Tenant-Id, X-ELF-Project-Id, X-ELF-Agent-Id

Path:
- space: team_shared|org_shared

Body:
{
  "grantee_kind": "project|agent",
  "grantee_agent_id": "optional-agent-id"
}

Response:
{
  "revoked": true
}

GET /health

Error body:
{
  "error_code": "NON_ENGLISH_INPUT|SCOPE_DENIED|INVALID_REQUEST|INTERNAL_ERROR",
  "message": "Human readable string.",
  "fields": ["$.headers.X-ELF-Tenant-Id", "$.notes[0].text"]
}

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
- Each query must be English only and must not contain any non-English text.
- Each query must be a single sentence.
- Include the original query unless INCLUDE_ORIGINAL is false.

System prompt (Expansion):
"You are a query expansion engine for a memory retrieval system.
Output must be valid JSON only and must match the provided schema exactly.
Generate short English-only query variations that preserve the original intent.
Do not include any non-English text. Do not add explanations or extra fields."

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
- Streamable HTTP MCP server that forwards tool calls to the public HTTP API.
- elf-mcp reads the optional [mcp] config section and attaches these headers on every request:
  - X-ELF-Tenant-Id
  - X-ELF-Project-Id
  - X-ELF-Agent-Id
  - X-ELF-Read-Profile (server-configured from mcp.read_profile; not client-controlled)
- Tools map 1:1 to v2 endpoints:
  - elf_notes_ingest -> POST /v2/notes/ingest
  - elf_events_ingest -> POST /v2/events/ingest
  - elf_core_blocks_get -> GET /v2/core-blocks
  - elf_entity_memory_get -> GET /v2/entity-memory
  - elf_graph_query -> POST /v2/graph/query
  - elf_searches_create -> POST /v2/searches
  - elf_searches_get -> GET /v2/searches/{search_id}
  - elf_searches_timeline -> GET /v2/searches/{search_id}/timeline
  - elf_searches_notes -> POST /v2/searches/{search_id}/notes
  - elf_docs_put -> POST /v2/docs
  - elf_docs_get -> GET /v2/docs/{doc_id}
  - elf_docs_search_l0 -> POST /v2/docs/search/l0
  - elf_docs_excerpts_get -> POST /v2/docs/excerpts
  - elf_notes_list -> GET /v2/notes
  - elf_notes_get -> GET /v2/notes/{note_id}
  - elf_notes_patch -> PATCH /v2/notes/{note_id}
  - elf_notes_delete -> DELETE /v2/notes/{note_id}
  - elf_notes_publish -> POST /v2/notes/{note_id}/publish
  - elf_notes_unpublish -> POST /v2/notes/{note_id}/unpublish
  - elf_space_grants_list -> GET /v2/spaces/{space}/grants
  - elf_space_grant_upsert -> POST /v2/spaces/{space}/grants
  - elf_space_grant_revoke -> POST /v2/spaces/{space}/grants/revoke
  - elf_admin_events_ingestion_profiles_list -> GET /v2/admin/events/ingestion-profiles
  - elf_admin_events_ingestion_profiles_create -> POST /v2/admin/events/ingestion-profiles
  - elf_admin_events_ingestion_profile_get -> GET /v2/admin/events/ingestion-profiles/{profile_id}
  - elf_admin_events_ingestion_profile_versions_list -> GET /v2/admin/events/ingestion-profiles/{profile_id}/versions
  - elf_admin_events_ingestion_profile_default_get -> GET /v2/admin/events/ingestion-profiles/default
  - elf_admin_events_ingestion_profile_default_set -> PUT /v2/admin/events/ingestion-profiles/default
  - elf_admin_traces_recent_list -> GET /v2/admin/traces/recent
  - elf_admin_trace_get -> GET /v2/admin/traces/{trace_id}
  - elf_admin_trajectory_get -> GET /v2/admin/trajectories/{trace_id}
  - elf_admin_trace_item_get -> GET /v2/admin/trace-items/{item_id}
  - elf_admin_trace_bundle_get -> GET /v2/admin/traces/{trace_id}/bundle
  - elf_recall_debug_panel -> POST /v2/recall-debug/panel
  - elf_admin_note_provenance_get -> GET /v2/admin/notes/{note_id}/provenance
  - elf_admin_memory_history_get -> GET /v2/admin/notes/{note_id}/history
- The MCP server must contain zero business logic or policy.
- All policy remains in elf-api and elf-service.

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
      "structured": {
        "summary": "string|null",
        "facts": "string[]|null",
        "concepts": "string[]|null",
        "entities": [
          {
            "canonical": "string|null",
            "kind": "string|null",
            "aliases": "string[]|null"
          }
        ]|null,
        "relations": [
          {
            "subject": {
              "canonical": "string|null",
              "kind": "string|null",
              "aliases": "string[]|null"
            },
            "predicate": "string",
            "object": {
              "entity": {
                "canonical": "string|null",
                "kind": "string|null",
                "aliases": "string[]|null"
              }|null,
              "value": "string|null"
            },
            "valid_from": "ISO8601 datetime|null",
            "valid_to": "ISO8601 datetime|null"
          }
        ]|null
      }|null,
      "scope_suggestion": "agent_private|project_shared|org_shared|null",
      "evidence": [
        { "message_index": number, "quote": "string" }
      ],
      "reason": "string"
    }
  ]
}

Notes:
- Exactly one of object.entity and object.value must be non-null.

Hard rules:
- notes.length <= MAX_NOTES
- text must be English-only (must pass the English gate)
- each note must be one sentence
- evidence must be 1..2 quotes
- each evidence.quote must be a verbatim substring of messages[message_index].content
- when write_policy is provided on a source message, evidence checks run after policy transforms
- do not store secrets or PII

System prompt (Extractor):
"You are a memory extraction engine for an agent memory system.
Output must be valid JSON only and must match the provided schema exactly.
Extract at most MAX_NOTES high-signal, cross-session reusable memory notes from the given messages.
Each note must be one English sentence and must not contain any non-English text.
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
- Any input that fails the English gate (Section 3) in add_note, add_event, or search
  returns HTTP 422 with a JSONPath-like field path.
C. Evidence binding:
- If extractor evidence.quote is not a substring -> REJECTED with REJECT_EVIDENCE_MISMATCH.
- If mismatch is introduced when requested message write_policy transforms content -> REJECTED with REJECT_WRITE_POLICY_MISMATCH.
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
20. OUT OF SCOPE (v2.0)
============================================================
- Translation or multilingual retrieval (handled by upstream agents).
- Graph memory backend (reserved for later).
- Public internet exposure and auth (localhost only in v2.0).
