# Chunked Embeddings (Chunk-First Retrieval) Design

**Goal:** Deliver a chunk-first retrieval architecture that maximizes recall and precision while keeping indexing and updates efficient.

**Context:** The system currently embeds entire notes and indexes them as single Qdrant points. This design shifts the retrieval unit to chunks and makes chunk embeddings the primary source of truth for search, while preserving a pooled note vector for fast update and duplicate detection.

## Decisions

- Chunk-first retrieval is the default end-to-end behavior.
- Chunking is sentence-aware with token limits and overlap.
- Chunk size defaults: `max_tokens = 512`, `overlap_tokens = 128`.
- Chunk IDs are deterministic from `(note_id, chunk_index)`.
- Both dense and BM25 vectors are chunk-level.
- Search returns chunk items with snippets only; full notes are fetched separately.
- Rerank is chunk-level; note aggregation uses top-1 chunk score.
- Adjacent stitching uses the top chunk plus its immediate neighbors.
- Chunk text is stored in Postgres; Qdrant payload contains only minimal metadata.
- `note_embeddings` becomes a derived pooled vector from chunk embeddings.
- Tokenizer auto-downloads from Hugging Face; default repo inherits from `providers.embedding.model`.

## Architecture Summary

Chunk embeddings are the primary retrieval unit. Each note is split into sentence-aware chunks, embedded in the worker, and stored in two tables: chunk metadata (`memory_note_chunks`) and chunk vectors (`note_chunk_embeddings`). Qdrant stores one point per chunk for dense and BM25 search. Search operates on chunk points, reranks chunk snippets, then aggregates by note using the top-1 chunk score. A pooled note vector (mean of chunk vectors) is stored in `note_embeddings` to keep update and duplicate detection efficient and deterministic without extra embedding calls.

## Data Flow

**Write path**
- `add_note` and `add_event` insert a single row into `memory_notes` and enqueue `indexing_outbox`.
- The worker loads the note, validates status and TTL, and splits text into chunks.
- The worker persists chunk rows and chunk embeddings, computes a pooled note vector, and updates `note_embeddings`.
- The worker upserts one Qdrant point per chunk with dense and BM25 vectors.

**Search path**
- Qdrant returns chunk candidates for dense + BM25 fusion queries.
- Candidates are revalidated against Postgres note status, TTL, and scope.
- Rerank runs on stitched snippet text (chunk + neighbors).
- Results aggregate by note using the top-1 chunk score.
- API response returns chunk items with snippet and offsets.

## Schema Changes

Add two new tables under `sql/tables/` and include them in `sql/init.sql` in dependency order.

**memory_note_chunks**
- `chunk_id uuid primary key`
- `note_id uuid not null references memory_notes(note_id) on delete cascade`
- `chunk_index int not null`
- `start_offset int not null`
- `end_offset int not null`
- `text text not null`
- `embedding_version text not null`
- `created_at timestamptz not null default now()`

Indexes:
- `(note_id, chunk_index)`
- `(note_id)`

**note_chunk_embeddings**
- `chunk_id uuid not null references memory_note_chunks(chunk_id) on delete cascade`
- `embedding_version text not null`
- `embedding_dim int not null`
- `vec vector(<VECTOR_DIM>) not null`
- `created_at timestamptz not null default now()`

Primary key: `(chunk_id, embedding_version)`

**Existing tables**
- `note_embeddings` is retained but redefined as derived pooled vectors.
- `search_trace_items` should add `chunk_id` for explainability.
- `memory_hits` should add `chunk_id` to reflect chunk-level results.

## Configuration

Add a `chunking` section to `elf.toml`:

- `chunking.enabled` (bool, default true)
- `chunking.max_tokens` (u32, default 512)
- `chunking.overlap_tokens` (u32, default 128)
- `chunking.tokenizer_repo` (optional). If empty, use `providers.embedding.model`.

Validation rules:
- `max_tokens > 0`
- `overlap_tokens < max_tokens`
- The tokenizer repo must resolve to a Hugging Face model name.

Tokenizer loading:
- Use `tokenizers` with the `http` feature.
- Load with `Tokenizer::from_pretrained(<repo>, None)` and cache in worker state.

## Chunking Algorithm

1. Split text into sentence segments using `unicode-segmentation`.
2. Accumulate sentences until adding the next would exceed `max_tokens`.
3. Create a chunk, then carry `overlap_tokens` from the previous chunk tail.
4. If a single sentence exceeds `max_tokens`, fall back to fixed token windows with overlap.

Chunk IDs are deterministic from `(note_id, chunk_index)` to ensure idempotent rebuilds.

## Qdrant Payload

Each chunk point includes the following payload fields:
- `note_id`, `chunk_id`, `chunk_index`
- `start_offset`, `end_offset`
- `tenant_id`, `project_id`, `agent_id`, `scope`, `status`
- `type`, `key`, `updated_at`, `expires_at`, `importance`, `confidence`
- `embedding_version`

Chunk text is not stored in Qdrant payload.

## API Changes

Search is chunk-first:
- `POST /v2/searches` returns chunk items and snippets.
- Snippets are stitched from the top chunk plus immediate neighbors.
- Full notes are fetched separately via `POST /v2/searches/{search_id}/notes` or `GET /v2/notes/{note_id}`.

Search explain:
- `GET /v2/admin/trace-items/{item_id}` returns per-item explain data, including `chunk_id` alongside scores.

## Rebuild and Indexing

- Rebuild Qdrant from `note_chunk_embeddings` and chunk text only.
- No embedding API calls during rebuild.
- Deletions remove all chunk points by filtering on `note_id`.

## Error Handling

- Indexing failures are retried via `indexing_outbox` with backoff.
- Tokenizer download failure should stop the worker with a clear error.
- Qdrant failures are logged and retried; Postgres remains authoritative.

## Testing

Add tests to cover:
- Sentence-aware chunking with token limits and overlap.
- Deterministic chunk IDs.
- Chunk embeddings persisted and pooled note vectors derived without extra embedding calls.
- Qdrant point count matches chunk count.
- Search returns chunk items with correct offsets and snippet stitching.
- Rebuild uses chunk embeddings and does not call the embedding provider.

## Spec Updates

Update `docs/spec/system_elf_memory_service_v2.md` to reflect:
- Chunk embeddings as the source-of-truth vectors.
- `note_embeddings` as derived pooled vectors.
- New tables and search explain fields.

## Open Questions

None. The design is ready for implementation.
