# Doc Extension v1 (Evidence Store) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement Doc Extension v1: PG-backed document store + doc chunk indexing outbox + Qdrant derived index (dense + BM25) + L0/L1/L2 retrieval endpoints exposed via HTTP and MCP.

**Architecture:** Add new PG tables for docs/chunks/embeddings/outbox, add worker pipeline to index doc chunks into a dedicated Qdrant collection, implement minimal Doc APIs and MCP tools. Reuse existing scope + grants semantics and the existing outbox/worker patterns.

**Tech Stack:** Rust (axum, sqlx), Postgres (+ pgvector), Qdrant, MCP (elf-mcp).

---

### Task 1: Add Doc schema tables (PG)

**Files:**
- Create: `sql/tables/025_doc_documents.sql`
- Create: `sql/tables/026_doc_chunks.sql`
- Create: `sql/tables/027_doc_chunk_embeddings.sql`
- Create: `sql/tables/028_doc_indexing_outbox.sql`
- Modify: `sql/init.sql`
- Modify: `packages/elf-storage/src/schema.rs`

**Step 1: Create SQL tables**
- Define doc/chunk tables with scope checks, hash fields, and indexes for lookup by `(tenant_id, project_id, scope, status)`.
- Add chunk offsets and hashes.
- Add doc outbox table (chunk_id + op + embedding_version + retry fields).

**Step 2: Wire tables into schema renderer**
- Include `\\ir` entries in `sql/init.sql`.
- Add `include_str!` matches in `packages/elf-storage/src/schema.rs`.

**Step 3: Run formatting / tests**
- Run: `cargo make fmt`
- Run: `cargo make test`

**Step 4: Commit**
- Add all new SQL files + schema include changes.

---

### Task 2: Add storage models + queries for Doc (PG)

**Files:**
- Modify: `packages/elf-storage/src/models.rs`
- Create: `packages/elf-storage/src/docs.rs`
- Modify: `packages/elf-storage/src/lib.rs`

**Step 1: Add Rust models**
- Add structs for doc document, doc chunk, doc chunk embedding, doc outbox job.

**Step 2: Add PG queries**
- Insert doc + chunks transactionally.
- Fetch doc metadata and content by id (authoritative for hydrate).
- Fetch chunks by doc_id / chunk_id.
- Outbox: claim next doc job, mark done/failed with backoff.

**Step 3: Add unit tests (pure logic only)**
- Hash computation and bounds helpers (no DB required).

**Step 4: Run tests + commit**
- Run: `cargo make test`
- Commit.

---

### Task 3: Extend config for Qdrant docs collection

**Files:**
- Modify: `packages/elf-config/src/types.rs`
- Modify: `packages/elf-config/src/lib.rs`
- Modify: `elf.example.toml`

**Step 1: Add `docs_collection`**
- Add `docs_collection: String` to Qdrant config with default `doc_chunks_v1`.

**Step 2: Validate config**
- Ensure non-empty and printable.

**Step 3: Run tests + commit**
- Run: `cargo make test`
- Commit.

---

### Task 4: Add worker pipeline for Doc outbox → Qdrant docs collection

**Files:**
- Modify: `apps/elf-worker/src/worker.rs`
- Modify: `packages/elf-storage/src/qdrant.rs` (construct store for docs collection)

**Step 1: Add `docs_qdrant` store**
- Instantiate a second QdrantStore using `cfg.storage.qdrant.docs_collection`.

**Step 2: Process doc outbox jobs**
- `UPSERT`: load chunk text, embed, store embedding in PG, upsert Qdrant doc chunk point (dense+bm25).
- `DELETE`: delete doc chunk points by chunk_id/doc_id.

**Step 3: Run unit tests**
- Add small tests for payload shape helpers (no external PG/Qdrant).

**Step 4: Commit**

---

### Task 5: Implement Doc service methods (docs_put/docs_get/docs_search_l0/docs_excerpts_get)

**Files:**
- Create: `packages/elf-service/src/docs.rs`
- Modify: `packages/elf-service/src/lib.rs`

**Step 1: docs_put**
- Validate request size (<= 4 MiB) and English gate.
- Deterministically chunk content (2048/256).
- Persist doc+chunks, enqueue doc outbox jobs for chunks.
- If scope is shared, ensure project grant (project_shared) or org grant (org_shared in `__org__`).

**Step 2: docs_get**
- Return metadata + content_hash + bytes; omit full content by default.

**Step 3: docs_search_l0**
- Embed query once.
- Run Qdrant fusion query against docs collection with filters for tenant/project scope.
- Return L0 results: doc_id/chunk_id + tiny snippet + metadata handles.

**Step 4: docs_excerpts_get**
- Resolve selector (quote preferred, position fallback).
- Enforce L1/L2 byte bounds and return excerpt + verification signals.

**Step 5: Tests**
- Pure logic tests for selector resolution + bounds + hashing.
- Integration tests can be ignored when external PG/Qdrant not configured (mirror existing acceptance style).

**Step 6: Commit**

---

### Task 6: Wire HTTP endpoints in elf-api

**Files:**
- Modify: `apps/elf-api/src/routes.rs`

**Step 1: Add routes**
- `POST /v2/docs`
- `GET /v2/docs/{doc_id}`
- `POST /v2/docs/search/l0`
- `POST /v2/docs/excerpts`

**Step 2: Validate request bytes and headers**
- Reuse existing request size guards and context headers.

**Step 3: Commit**

---

### Task 7: Expose MCP tools via elf-mcp (single surface, per decision A)

**Files:**
- Modify: `apps/elf-mcp/src/server.rs`

**Step 1: Add tools**
- `docs_put`, `docs_get`, `docs_search_l0`, `docs_excerpts_get`

**Step 2: Ensure fail-closed behavior when disabled**
- If extension is disabled by config, return explicit error payload.

**Step 3: Commit**

---

### Task 8: Verification + regression checks

**Step 1: Run full test suite**
- Run: `cargo make test`

**Step 2: Manual smoke (optional)**
- Start services and run a minimal docs_put → docs_search_l0 → docs_excerpts_get flow.

**Step 3: Push**
- Push directly to `main` (per user preference).

