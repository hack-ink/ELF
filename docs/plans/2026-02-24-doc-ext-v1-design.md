# Doc Extension v1 (Evidence Store) — Design

**Status:** Approved (v1 scope locked)

## Goal

Provide an ELF Extension for long-form evidence storage and retrieval that:

- Stores English-only documents in Postgres (source of truth).
- Builds a derived Qdrant index for retrieval (dense + BM25).
- Supports progressive disclosure (L0 discovery; L1/L2 bounded excerpts).
- Returns verifiable excerpts (selectors + hashes + verified flag), enabling facts-first workflows.

## Non-goals (v1)

- No public library (tenant_public or cross-tenant global public). Tracked separately (deferred).
- No translation or multilingual retrieval.
- No LLM query expansion for doc search.
- No heavy reranking or “full search platform” feature set (analytics, entity extraction, etc.).

## Core vs Extension boundary

- **ELF Core** remains facts-first memory (short notes; advanced retrieval; expansion/fusion/rerank as needed).
- **Doc Extension v1** is an evidence store with minimal retrieval and bounded hydration.
  - Search exists only as `docs_search_l0` for discovery/backfill/debug.
  - All “real evidence reading” happens via `docs_excerpts_get`.

## Scope model (tenant-internal only)

Doc uses the same scope labels as Core memory:

- `agent_private`
- `project_shared` (aka `team_shared` externally)
- `org_shared` (stored under reserved `project_id = "__org__"`)

Shared visibility is controlled via explicit grants. The v1 implementation reuses the existing shared-grants semantics (project/agent grants) so that:

- `project_shared` supports intra-project sharing.
- `org_shared` supports intra-tenant, cross-project sharing.

## English-only boundary

All Doc text inputs must satisfy the English gate (Core policy). Doc v1 does not translate.

## Storage: Postgres (SoT)

### Entities

- **Document**
  - `doc_id` (uuid)
  - `tenant_id`, `project_id`, `agent_id`, `scope`
  - `title` (optional)
  - `source_ref` (optional json)
  - `content` (text)
  - `content_hash` (blake3 hex of raw UTF-8 bytes)
  - `content_bytes` (bytes length)
  - timestamps, status

- **Chunk**
  - `chunk_id` (uuid)
  - `doc_id` (fk)
  - `chunk_index` (0..)
  - `start_offset`, `end_offset` (byte offsets in UTF-8 `content`)
  - `chunk_text` (text)
  - `chunk_hash` (blake3)

- **Chunk embedding (SoT for rebuild)**
  - `chunk_id`, `embedding_version`, `embedding_dim`
  - `vec` (pgvector vector(VECTOR_DIM))

### Limits (defaults; configurable)

- `docs_put.max_doc_bytes = 4 MiB` (2^22)
- Chunking:
  - `target_bytes = 2048`
  - `overlap_bytes = 256`
  - `max_chunks_per_doc = 4096` (2^12)
- Excerpts:
  - `L1.max_bytes = 8 KiB` (2^13)
  - `L2.max_bytes = 32 KiB` (2^15)
- Search:
  - `docs_search_l0.top_k_max = 32` (2^5)

## Derived index: Qdrant

Doc Extension v1 uses a dedicated Qdrant collection for doc chunks.

- Point id = `chunk_id`
- Vectors:
  - `dense`: float32 embedding vector
  - `bm25`: `Document(text, model="qdrant/bm25")`
- Payload includes: `doc_id`, `chunk_id`, `chunk_index`, offsets, `tenant_id`, `project_id`, `agent_id`, `scope`, `status`, `updated_at`, `embedding_version`, `content_hash`, `chunk_hash`

This supports deterministic, model-free lexical retrieval (BM25) without storing SPLADE-like sparse vectors.

## Indexing consistency: transactional outbox

Doc ingestion enqueues indexing jobs in Postgres (outbox) in the same transaction as document persistence.

Worker processes doc outbox jobs (at-least-once):

- `UPSERT`: embed chunk text, store embedding in PG, upsert point to Qdrant doc collection
- `DELETE`: delete points by doc_id or chunk_ids

All operations must be idempotent.

## Retrieval & progressive disclosure

### L0: discovery (`docs_search_l0`)

Inputs:
- `query` (English-only)
- filters: `scope`, (optional) `status`, `doc_type` (future), time bounds (future)
- `top_k` (<= 32)
- `candidate_k` (<= 1024)

Behavior:
- Embed query text (dense)
- Run Qdrant fusion query: dense prefetch + bm25 prefetch; final query fusion = RRF
- Return L0 items: pointers + tiny preview snippet + minimal metadata
- Do not return large excerpts

### L1/L2: hydration (`docs_excerpts_get`)

Inputs:
- `doc_id`
- selector:
  - `chunk_id` and optional local offsets, or
  - `TextQuoteSelector` (exact + prefix + suffix), and optional `TextPositionSelector` (start/end)
- `level = L1|L2`

Behavior:
- Load authoritative `content` (PG)
- Resolve selector:
  - Prefer TextQuoteSelector match; fallback to TextPositionSelector when provided
- Extract bounded window:
  - L1: <= 8 KiB
  - L2: <= 32 KiB
- Return excerpt + verification signals (below)

## Verification contract (v1)

Every excerpt response must include:

- `locator` (the selector used / resolved)
- `hashes` (at least `content_hash` and `excerpt_hash`, blake3 hex)
- `verified: bool`
- `verification_errors: []`

Rules:
- If selector resolution fails or hashes mismatch: `verified=false`.
- Agents should treat `verified=false` excerpts as best-effort and avoid using them as hard evidence.

Cryptographic signing may be added later; v1 requires hash+selector verification only.

## API & MCP surface

HTTP (Extension endpoints):

- `POST /v2/docs` → `docs_put`
- `GET /v2/docs/{doc_id}` → `docs_get` (metadata-first)
- `POST /v2/docs/search/l0` → `docs_search_l0`
- `POST /v2/docs/excerpts` → `docs_excerpts_get`

MCP (single surface via `elf-mcp`):

- `elf_docs_put`
- `elf_docs_get`
- `elf_docs_search_l0`
- `elf_docs_excerpts_get`

If Doc Extension is disabled/unconfigured, tools must fail closed with explicit, stable error codes.
