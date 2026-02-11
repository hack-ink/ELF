# Chunked Embeddings Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement chunk-first indexing and retrieval with sentence-aware chunking, chunk-level embeddings, and chunk-native search responses.

**Architecture:** Chunk metadata and embeddings are stored in Postgres, Qdrant stores one point per chunk, and search is chunk-first with rerank on stitched snippets. A pooled note vector (mean of chunk vectors) is stored in `note_embeddings` for fast update and duplicate detection.

**Tech Stack:** Rust, sqlx/Postgres, Qdrant, `tokenizers` (HTTP feature), `unicode-segmentation`.

---

### Task 1: Add chunking config and validation

**Files:**

- Modify: `packages/elf-config/src/types.rs`
- Modify: `packages/elf-config/src/lib.rs`
- Modify: `elf.example.toml`
- Test: `packages/elf-config/tests/config_validation.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn chunking_config_requires_valid_bounds() {
    let mut cfg = base_config();
    cfg.chunking.max_tokens = 0;
    assert!(validate(&cfg).is_err());

    cfg = base_config();
    cfg.chunking.overlap_tokens = cfg.chunking.max_tokens;
    assert!(validate(&cfg).is_err());
}

#[test]
fn chunking_tokenizer_repo_can_inherit_from_embedding_model() {
    let mut cfg = base_config();
    cfg.chunking.tokenizer_repo = None;
    assert!(validate(&cfg).is_ok());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p elf-config config_validation -v`
Expected: FAIL because `chunking` is not defined or validated yet.

**Step 3: Write minimal implementation**

Add to `packages/elf-config/src/types.rs`:

```rust
#[derive(Debug, Deserialize)]
pub struct Config {
    pub service: Service,
    pub storage: Storage,
    pub providers: Providers,
    pub scopes: Scopes,
    pub memory: Memory,
    pub search: Search,
    pub ranking: Ranking,
    pub lifecycle: Lifecycle,
    pub security: Security,
    pub chunking: Chunking,
}

#[derive(Debug, Deserialize)]
pub struct Chunking {
    pub enabled: bool,
    pub max_tokens: u32,
    pub overlap_tokens: u32,
    pub tokenizer_repo: Option<String>,
}
```

Update `packages/elf-config/src/lib.rs` validation:

```rust
if !cfg.chunking.enabled {
    return Err(color_eyre::eyre::eyre!("chunking.enabled must be true."));
}
if cfg.chunking.max_tokens == 0 {
    return Err(color_eyre::eyre::eyre!("chunking.max_tokens must be greater than zero."));
}
if cfg.chunking.overlap_tokens >= cfg.chunking.max_tokens {
    return Err(color_eyre::eyre::eyre!(
        "chunking.overlap_tokens must be less than chunking.max_tokens."
    ));
}
// tokenizer_repo may be empty or omitted to inherit providers.embedding.model.
```

Update `elf.example.toml`:

```toml
[chunking]
enabled = true
max_tokens = 512
overlap_tokens = 128
# If empty, uses providers.embedding.model
tokenizer_repo = ""
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p elf-config config_validation -v`
Expected: PASS.

**Step 5: Commit**

```bash
git add packages/elf-config/src/types.rs packages/elf-config/src/lib.rs packages/elf-config/tests/config_validation.rs elf.example.toml
git commit -m '{"schema":"cmsg/1","type":"feat","scope":"config","summary":"Add chunking configuration","intent":"Introduce chunking config and validation","impact":"Enables chunk-first settings and defaults","breaking":false,"risk":"low","refs":[]}'
```

---

### Task 2: Add chunk tables and adjust schema

**Files:**

- Create: `sql/tables/009_memory_note_chunks.sql`
- Create: `sql/tables/010_note_chunk_embeddings.sql`
- Modify: `sql/tables/004_memory_hits.sql`
- Modify: `sql/tables/006_search_traces.sql`
- Modify: `sql/init.sql`
- Modify: `packages/elf-storage/src/schema.rs`
- Modify: `packages/elf-storage/src/models.rs`

**Step 1: Write the failing test**

Add a schema bootstrap test in `packages/elf-storage/tests/db_smoke.rs`:

```rust
#[test]
fn chunk_tables_exist_after_bootstrap() {
    let dsn = std::env::var("ELF_PG_DSN").expect("ELF_PG_DSN required");
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let db = elf_storage::db::Db::connect(&dsn).await.unwrap();
        let rows: (i64,) = sqlx::query_as("SELECT count(*) FROM information_schema.tables WHERE table_name = 'memory_note_chunks'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(rows.0, 1);
    });
}
```

**Step 2: Run test to verify it fails**

Run: `ELF_PG_DSN=... cargo test -p elf-storage db_smoke -v`
Expected: FAIL because table does not exist.

**Step 3: Write minimal implementation**

Create `sql/tables/009_memory_note_chunks.sql`:

```sql
CREATE TABLE IF NOT EXISTS memory_note_chunks (
    chunk_id uuid PRIMARY KEY,
    note_id uuid NOT NULL REFERENCES memory_notes(note_id) ON DELETE CASCADE,
    chunk_index int NOT NULL,
    start_offset int NOT NULL,
    end_offset int NOT NULL,
    text text NOT NULL,
    embedding_version text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_note_chunks_note
    ON memory_note_chunks (note_id);
CREATE INDEX IF NOT EXISTS idx_note_chunks_note_index
    ON memory_note_chunks (note_id, chunk_index);
```

Create `sql/tables/010_note_chunk_embeddings.sql`:

```sql
CREATE TABLE IF NOT EXISTS note_chunk_embeddings (
    chunk_id uuid NOT NULL REFERENCES memory_note_chunks(chunk_id) ON DELETE CASCADE,
    embedding_version text NOT NULL,
    embedding_dim int NOT NULL,
    vec vector(<VECTOR_DIM>) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (chunk_id, embedding_version)
);
```

Update `sql/tables/004_memory_hits.sql`:

```sql
ALTER TABLE memory_hits
    ADD COLUMN IF NOT EXISTS chunk_id uuid NULL;
```

Update `sql/tables/006_search_traces.sql`:

```sql
ALTER TABLE search_trace_items
    ADD COLUMN IF NOT EXISTS chunk_id uuid NULL;
```

Update `sql/init.sql` to include new tables after `001_memory_notes.sql`:

```sql
\ir tables/009_memory_note_chunks.sql
\ir tables/010_note_chunk_embeddings.sql
```

Update `packages/elf-storage/src/schema.rs` to include the new SQL files in order.

Add models in `packages/elf-storage/src/models.rs`:

```rust
#[derive(Debug, sqlx::FromRow)]
pub struct MemoryNoteChunk {
    pub chunk_id: uuid::Uuid,
    pub note_id: uuid::Uuid,
    pub chunk_index: i32,
    pub start_offset: i32,
    pub end_offset: i32,
    pub text: String,
    pub embedding_version: String,
    pub created_at: time::OffsetDateTime,
}

#[derive(Debug, sqlx::FromRow)]
pub struct NoteChunkEmbedding {
    pub chunk_id: uuid::Uuid,
    pub embedding_version: String,
    pub embedding_dim: i32,
    pub vec: Vec<f32>,
    pub created_at: time::OffsetDateTime,
}
```

**Step 4: Run test to verify it passes**

Run: `ELF_PG_DSN=... cargo test -p elf-storage db_smoke -v`
Expected: PASS.

**Step 5: Commit**

```bash
git add sql/init.sql sql/tables/009_memory_note_chunks.sql sql/tables/010_note_chunk_embeddings.sql sql/tables/004_memory_hits.sql sql/tables/006_search_traces.sql packages/elf-storage/src/schema.rs packages/elf-storage/src/models.rs packages/elf-storage/tests/db_smoke.rs
git commit -m '{"schema":"cmsg/1","type":"feat","scope":"storage","summary":"Add chunk tables and schema hooks","intent":"Persist chunk metadata and embeddings","impact":"Enables chunk-first indexing and trace fields","breaking":false,"risk":"medium","refs":[]}'
```

---

### Task 3: Add chunking utilities and dependencies

**Files:**

- Modify: `Cargo.toml`
- Modify: `apps/elf-worker/Cargo.toml`
- Create: `apps/elf-worker/src/chunking.rs`
- Modify: `apps/elf-worker/src/lib.rs`
- Test: `apps/elf-worker/src/chunking.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn splits_into_chunks_with_overlap() {
    let cfg = ChunkingConfig {
        max_tokens: 10,
        overlap_tokens: 2,
    };
    let tokenizer = Tokenizer::from_pretrained("Qwen/Qwen3-Embedding-8B", None).unwrap();
    let chunks = split_text("One. Two. Three. Four.", &cfg, &tokenizer);
    assert!(chunks.len() >= 1);
    assert!(chunks[0].text.contains("One"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p elf-worker chunking -v`
Expected: FAIL because module does not exist.

**Step 3: Write minimal implementation**

Add workspace deps in `Cargo.toml`:

```toml
[workspace.dependencies]
unicode-segmentation = "1.11"
tokenizers = { version = "0.20", features = ["http"] }
```

Add to `apps/elf-worker/Cargo.toml`:

```toml
[dependencies]
unicode-segmentation = { workspace = true }
tokenizers = { workspace = true }
```

Create `apps/elf-worker/src/chunking.rs`:

```rust
use unicode_segmentation::UnicodeSegmentation;
use tokenizers::Tokenizer;

#[derive(Clone, Debug)]
pub struct ChunkingConfig {
    pub max_tokens: u32,
    pub overlap_tokens: u32,
}

#[derive(Clone, Debug)]
pub struct Chunk {
    pub chunk_index: i32,
    pub start_offset: usize,
    pub end_offset: usize,
    pub text: String,
}

pub fn split_text(text: &str, cfg: &ChunkingConfig, tokenizer: &Tokenizer) -> Vec<Chunk> {
    let sentences: Vec<(usize, &str)> = text.split_sentence_bound_indices().collect();
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_start = 0usize;
    let mut last_end = 0usize;
    let mut chunk_index = 0i32;

    for (idx, sentence) in sentences {
        let candidate = format!("{}{}", current, sentence);
        let token_count = tokenizer.encode(candidate.as_str(), false).map(|e| e.len()).unwrap_or(0);
        if token_count as u32 > cfg.max_tokens && !current.is_empty() {
            chunks.push(Chunk { chunk_index, start_offset: current_start, end_offset: last_end, text: current.clone() });
            chunk_index += 1;
            let overlap = overlap_tail(&current, cfg.overlap_tokens, tokenizer);
            current_start = last_end.saturating_sub(overlap.len());
            current = overlap;
        }
        if current.is_empty() {
            current_start = idx;
        }
        current.push_str(sentence);
        last_end = idx + sentence.len();
    }
    if !current.is_empty() {
        chunks.push(Chunk { chunk_index, start_offset: current_start, end_offset: last_end, text: current });
    }
    chunks
}

fn overlap_tail(text: &str, overlap_tokens: u32, tokenizer: &Tokenizer) -> String {
    if overlap_tokens == 0 {
        return String::new();
    }
    let encoding = tokenizer.encode(text, false).unwrap();
    let tokens = encoding.get_ids();
    let start = tokens.len().saturating_sub(overlap_tokens as usize);
    let tail_ids = &tokens[start..];
    tokenizer.decode(tail_ids.to_vec(), true).unwrap_or_default()
}
```

Export in `apps/elf-worker/src/lib.rs`:

```rust
pub mod chunking;
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p elf-worker chunking -v`
Expected: PASS.

**Step 5: Commit**

```bash
git add Cargo.toml apps/elf-worker/Cargo.toml apps/elf-worker/src/chunking.rs apps/elf-worker/src/lib.rs
git commit -m '{"schema":"cmsg/1","type":"feat","scope":"worker","summary":"Add chunking utilities","intent":"Provide sentence-aware chunking helpers","impact":"Worker can split text into token-limited chunks","breaking":false,"risk":"medium","refs":[]}'
```

---

### Task 4: Implement chunk-first indexing in worker

**Files:**

- Modify: `apps/elf-worker/src/worker.rs`
- Modify: `packages/elf-storage/src/models.rs`
- Modify: `packages/elf-storage/src/queries.rs`
- Test: `apps/elf-worker/src/worker.rs`

**Step 1: Write the failing test**

Add a worker unit test for pooled vectors in `apps/elf-worker/src/worker.rs`:

```rust
#[test]
fn pooled_vector_is_mean_of_chunks() {
    let chunks = vec![vec![1.0_f32, 3.0_f32], vec![3.0_f32, 5.0_f32]];
    let pooled = mean_pool(&chunks).unwrap();
    assert_eq!(pooled, vec![2.0_f32, 4.0_f32]);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p elf-worker pooled_vector_is_mean_of_chunks -v`
Expected: FAIL (function missing).

**Step 3: Write minimal implementation**

Update `apps/elf-worker/src/worker.rs`:

- Add a tokenizer to `WorkerState` initialized from config (`tokenizer_repo` or `providers.embedding.model`).
- On UPSERT:
  - Split note text into chunks.
  - Insert chunk rows into `memory_note_chunks`.
  - Embed all chunk texts in one call.
  - Insert `note_chunk_embeddings` rows.
  - Compute pooled vector (mean) and update `note_embeddings`.
  - Upsert Qdrant points per chunk.

Add helper code:

```rust
fn mean_pool(chunks: &[Vec<f32>]) -> Option<Vec<f32>> {
    if chunks.is_empty() {
        return None;
    }
    let dim = chunks[0].len();
    let mut out = vec![0.0_f32; dim];
    for vec in chunks {
        for (i, value) in vec.iter().enumerate() {
            out[i] += value;
        }
    }
    for value in &mut out {
        *value /= chunks.len() as f32;
    }
    Some(out)
}
```

Insert chunk metadata:

```rust
sqlx::query(
    "INSERT INTO memory_note_chunks (chunk_id, note_id, chunk_index, start_offset, end_offset, text, embedding_version) \
     VALUES ($1,$2,$3,$4,$5,$6,$7) \
     ON CONFLICT (chunk_id) DO UPDATE SET text = EXCLUDED.text, start_offset = EXCLUDED.start_offset, end_offset = EXCLUDED.end_offset"
)
```

Insert chunk embeddings:

```rust
sqlx::query(
    "INSERT INTO note_chunk_embeddings (chunk_id, embedding_version, embedding_dim, vec) \
     VALUES ($1,$2,$3,$4) \
     ON CONFLICT (chunk_id, embedding_version) DO UPDATE SET embedding_dim = EXCLUDED.embedding_dim, vec = EXCLUDED.vec, created_at = now()"
)
```

Update delete handler to remove all chunk points via Qdrant filter on `note_id` instead of point ID.

**Step 4: Run test to verify it passes**

Run: `cargo test -p elf-worker pooled_vector_is_mean_of_chunks -v`
Expected: PASS.

**Step 5: Commit**

```bash
git add apps/elf-worker/src/worker.rs packages/elf-storage/src/queries.rs

git commit -m '{"schema":"cmsg/1","type":"feat","scope":"worker","summary":"Index chunks and pooled vectors","intent":"Embed chunk text and upsert chunk points","impact":"Worker indexes chunk-level embeddings into Postgres and Qdrant","breaking":false,"risk":"high","refs":[]}'
```

---

### Task 5: Update rebuild and search traces for chunks

**Files:**

- Modify: `packages/elf-service/src/admin.rs`
- Modify: `apps/elf-worker/src/worker.rs`
- Modify: `sql/tables/006_search_traces.sql`
- Modify: `sql/tables/004_memory_hits.sql`

**Step 1: Write the failing test**

Update `packages/elf-service/tests/acceptance/rebuild_qdrant.rs`:

```rust
// After inserting chunk embeddings, rebuild should use them and not note_embeddings.
assert_eq!(report.missing_vector_count, 0);
assert!(report.rebuilt_count >= 1);
```

**Step 2: Run test to verify it fails**

Run: `ELF_PG_DSN=... ELF_QDRANT_URL=... cargo test -p elf-service rebuild_qdrant -v`
Expected: FAIL because rebuild still reads note_embeddings.

**Step 3: Write minimal implementation**

Update `packages/elf-service/src/admin.rs` to query `memory_note_chunks` joined with `note_chunk_embeddings`, and build Qdrant points per chunk. Use chunk text for BM25.

Update `apps/elf-worker/src/worker.rs` `TraceItemInsert` to include `chunk_id`, and write it in the `search_trace_items` insert.

Update `memory_hits` insertion to include `chunk_id` in the hit log.

**Step 4: Run test to verify it passes**

Run: `ELF_PG_DSN=... ELF_QDRANT_URL=... cargo test -p elf-service rebuild_qdrant -v`
Expected: PASS.

**Step 5: Commit**

```bash
git add packages/elf-service/src/admin.rs apps/elf-worker/src/worker.rs packages/elf-service/tests/acceptance/rebuild_qdrant.rs sql/tables/006_search_traces.sql sql/tables/004_memory_hits.sql

git commit -m '{"schema":"cmsg/1","type":"feat","scope":"search","summary":"Rebuild and trace chunks","intent":"Use chunk embeddings for rebuild and explain","impact":"Qdrant and traces operate on chunk-level results","breaking":false,"risk":"high","refs":[]}'
```

---

### Task 6: Make search chunk-first and add note fetch endpoint

**Files:**

- Modify: `packages/elf-service/src/search.rs`
- Modify: `packages/elf-service/src/list.rs`
- Create: `packages/elf-service/src/notes.rs`
- Modify: `packages/elf-service/src/lib.rs`
- Modify: `apps/elf-api/src/routes.rs`
- Test: `packages/elf-service/tests/acceptance/` (new test)

**Step 1: Write the failing test**

Add `packages/elf-service/tests/acceptance/chunk_search.rs`:

```rust
#[tokio::test]
async fn search_returns_chunk_items() {
    let test = elf_testkit::TestHarness::new().await;
    test.add_note("First sentence. Second sentence.").await;
    test.wait_for_index().await;

    let response = test.search("First").await;
    let item = response.items.first().expect("expected item");
    assert!(item.chunk_id.is_some());
    assert!(!item.snippet.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `ELF_PG_DSN=... ELF_QDRANT_URL=... cargo test -p elf-service chunk_search -v`
Expected: FAIL because response is note-level.

**Step 3: Write minimal implementation**

Update `SearchItem` in `packages/elf-service/src/search.rs`:

```rust
pub struct SearchItem {
    pub result_handle: uuid::Uuid,
    pub note_id: uuid::Uuid,
    pub chunk_id: uuid::Uuid,
    pub chunk_index: i32,
    pub start_offset: i32,
    pub end_offset: i32,
    pub snippet: String,
    // note metadata fields
}
```

Adjust search pipeline:

- Parse Qdrant payload for `chunk_id`, `chunk_index`, `start_offset`, `end_offset`.
- Load chunk text from `memory_note_chunks` for snippet stitching.
- Rerank chunk snippets (chunk + neighbors).
- Aggregate by note using top-1 chunk score.

Add `packages/elf-service/src/notes.rs` with:

```rust
pub struct NoteFetchRequest { pub note_id: uuid::Uuid }
pub struct NoteFetchResponse { pub note_id: uuid::Uuid, pub text: String, /* metadata */ }
```

Wire new endpoint in `apps/elf-api/src/routes.rs`:

```rust
.route("/v1/memory/notes/:note_id", get(get_note))
```

**Step 4: Run test to verify it passes**

Run: `ELF_PG_DSN=... ELF_QDRANT_URL=... cargo test -p elf-service chunk_search -v`
Expected: PASS.

**Step 5: Commit**

```bash
git add packages/elf-service/src/search.rs packages/elf-service/src/notes.rs packages/elf-service/src/lib.rs apps/elf-api/src/routes.rs packages/elf-service/tests/acceptance/chunk_search.rs

git commit -m '{"schema":"cmsg/1","type":"feat","scope":"api","summary":"Return chunk-first search results","intent":"Make search results chunk-native and add note fetch","impact":"Search response aligns with chunk retrieval","breaking":true,"risk":"high","refs":[]}'
```

---

### Task 7: Update specs and docs

**Files:**

- Modify: `docs/spec/system_elf_memory_service_v1.md`
- Modify: `docs/guide/integration-testing.md`

**Step 1: Write the failing test**

Add a doc lint placeholder (if no doc lint exists, skip this test step).

**Step 2: Update docs**

- Mark `note_chunk_embeddings` as source-of-truth vectors.
- Mark `note_embeddings` as pooled derived vectors.
- Add `memory_note_chunks` and `note_chunk_embeddings` tables.
- Update search explain and memory hits to include `chunk_id`.
- Document new chunking config.

**Step 3: Commit**

```bash
git add docs/spec/system_elf_memory_service_v1.md docs/guide/integration-testing.md

git commit -m '{"schema":"cmsg/1","type":"docs","scope":"global","summary":"Document chunk-first retrieval","intent":"Align specs and guides with chunk embeddings","impact":"Specs reflect new schema and API","breaking":false,"risk":"low","refs":[]}'
```

---

## Final Verification

Run: `cargo test`
Expected: PASS (integration tests may be ignored if external services are not set).

---

## Execution Handoff

Plan complete and saved to `docs/plans/2026-02-04-chunked-embeddings-implementation.md`.

Two execution options:

1. Subagent-Driven (this session) — I dispatch fresh subagent per task, review between tasks, fast iteration.
2. Parallel Session (separate) — Open new session with executing-plans, batch execution with checkpoints.

Which approach?
