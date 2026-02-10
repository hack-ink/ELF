# LLM Cache Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a Postgres-backed cache for LLM query expansion and reranking to reduce repeated calls while keeping results consistent.

**Architecture:** The cache lives in Postgres with TTL metadata and is keyed by a BLAKE3 hash of the minimal correctness inputs. Expansion and rerank logic consults the cache before calling providers, and the worker periodically deletes expired rows.

**Tech Stack:** Rust, sqlx (Postgres), time, blake3, cargo-make.

---

### Task 1: Add Search Cache Config and Validation

**Files:**
- Modify: `packages/elf-config/src/types.rs`
- Modify: `packages/elf-config/src/lib.rs`
- Modify: `packages/elf-config/tests/config_validation.rs`
- Modify: `elf.example.toml`

**Step 1: Write the failing test**

Add a validation test that fails when cache TTL is zero.

```rust
#[test]
fn cache_ttl_must_be_positive() {
	let payload = sample_toml_with_cache(0, 7, true, "v1", "v1");
	let path = write_temp_config(payload);
	let result = elf_config::load(&path);
	std::fs::remove_file(&path).expect("Failed to remove test config.");
	let err = result.expect_err("Expected cache TTL validation error.");
	assert!(
		err.to_string().contains("search.cache.expansion_ttl_days must be greater than zero."),
		"Unexpected error: {err}"
	);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p elf-config -- tests/config_validation.rs::cache_ttl_must_be_positive`

Expected: FAIL with missing validation or missing config fields.

**Step 3: Write minimal implementation**

Add `SearchCache` to config types, wire it into `Search`, and validate values.

```rust
#[derive(Debug, Deserialize)]
pub struct SearchCache {
	pub enabled: bool,
	pub expansion_ttl_days: i64,
	pub rerank_ttl_days: i64,
	pub max_payload_bytes: Option<u64>,
	pub expansion_version: String,
	pub rerank_version: String,
}
```

Add validation in `validate`:

```rust
if cfg.search.cache.expansion_ttl_days <= 0 {
	return Err(color_eyre::eyre::eyre!(
		"search.cache.expansion_ttl_days must be greater than zero."
	));
}
if cfg.search.cache.rerank_ttl_days <= 0 {
	return Err(color_eyre::eyre::eyre!(
		"search.cache.rerank_ttl_days must be greater than zero."
	));
}
if let Some(max) = cfg.search.cache.max_payload_bytes {
	if max == 0 {
		return Err(color_eyre::eyre::eyre!(
			"search.cache.max_payload_bytes must be greater than zero."
		));
	}
}
if cfg.search.cache.expansion_version.trim().is_empty() {
	return Err(color_eyre::eyre::eyre!(
		"search.cache.expansion_version must be non-empty."
	));
}
if cfg.search.cache.rerank_version.trim().is_empty() {
	return Err(color_eyre::eyre::eyre!(
		"search.cache.rerank_version must be non-empty."
	));
}
```

Update `elf.example.toml` and the config test sample to include:

```toml
[search.cache]
enabled = true
expansion_ttl_days = 7
rerank_ttl_days = 7
max_payload_bytes = 262144
expansion_version = "v1"
rerank_version = "v1"
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p elf-config -- tests/config_validation.rs::cache_ttl_must_be_positive`

Expected: PASS.

**Step 5: Commit**

```bash
git add packages/elf-config/src/types.rs packages/elf-config/src/lib.rs \
  packages/elf-config/tests/config_validation.rs elf.example.toml
git commit -m '{"schema":"cmsg/1","type":"feat","scope":"global","summary":"Add search cache configuration","intent":"Expose cache toggles and TTL validation","impact":"Search config includes cache controls with validation","breaking":false,"risk":"low","refs":[]}'
```

---

### Task 2: Add LLM Cache Table and Schema Wiring

**Files:**
- Create: `sql/tables/008_llm_cache.sql`
- Modify: `sql/init.sql`
- Modify: `packages/elf-storage/src/schema.rs`

**Step 1: Write the schema**

Create the table and indexes:

```sql
CREATE TABLE IF NOT EXISTS llm_cache (
    cache_id uuid PRIMARY KEY,
    cache_kind text NOT NULL,
    cache_key text NOT NULL,
    payload jsonb NOT NULL,
    created_at timestamptz NOT NULL,
    last_accessed_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL,
    hit_count bigint NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_llm_cache_key
    ON llm_cache (cache_kind, cache_key);
CREATE INDEX IF NOT EXISTS idx_llm_cache_expires
    ON llm_cache (expires_at);
```

**Step 2: Wire schema includes**

Add `\ir tables/008_llm_cache.sql` to `sql/init.sql` and `schema.rs`.

**Step 3: Run tests to ensure compilation**

Run: `cargo test -p elf-storage --lib`

Expected: PASS.

**Step 4: Commit**

```bash
git add sql/tables/008_llm_cache.sql sql/init.sql packages/elf-storage/src/schema.rs
git commit -m '{"schema":"cmsg/1","type":"feat","scope":"global","summary":"Add LLM cache table","intent":"Store expansion and rerank cache entries","impact":"Database schema supports LLM cache retention","breaking":false,"risk":"low","refs":[]}'
```

---

### Task 3: Add Cache Key Helpers and Unit Tests

**Files:**
- Modify: `Cargo.toml`
- Modify: `packages/elf-service/Cargo.toml`
- Modify: `packages/elf-service/src/search.rs`

**Step 1: Write failing tests**

Add unit tests that prove cache keys change when versions or timestamps change.

```rust
#[test]
fn expansion_cache_key_changes_with_version() {
	let key_a = build_expansion_cache_key("alpha", "v1", 4, true, "llm", "model", 0.1);
	let key_b = build_expansion_cache_key("alpha", "v2", 4, true, "llm", "model", 0.1);
	assert_ne!(key_a, key_b);
}

#[test]
fn rerank_cache_key_changes_with_updated_at() {
	let ts_a = time::OffsetDateTime::from_unix_timestamp(1).expect("Valid timestamp.");
	let ts_b = time::OffsetDateTime::from_unix_timestamp(2).expect("Valid timestamp.");
	let note_id = uuid::Uuid::new_v4();
	let key_a = build_rerank_cache_key("q", "v1", "rerank", "model", vec![(note_id, ts_a)]);
	let key_b = build_rerank_cache_key("q", "v1", "rerank", "model", vec![(note_id, ts_b)]);
	assert_ne!(key_a, key_b);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p elf-service search::tests::expansion_cache_key_changes_with_version`

Expected: FAIL due to missing helper functions.

**Step 3: Implement helpers and add blake3 dependency**

Add `blake3` to `[workspace.dependencies]` and `elf-service` dependencies. Implement helpers in `search.rs`:

```rust
fn hash_cache_key(payload: &serde_json::Value) -> ServiceResult<String> {
	let raw = serde_json::to_vec(payload).map_err(|err| ServiceError::Storage {
		message: format!("Failed to encode cache key payload: {err}"),
	})?;
	Ok(blake3::hash(&raw).to_hex().to_string())
}
```

Add `build_expansion_cache_key` and `build_rerank_cache_key` to call `hash_cache_key`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p elf-service search::tests::expansion_cache_key_changes_with_version`

Expected: PASS.

**Step 5: Commit**

```bash
git add Cargo.toml packages/elf-service/Cargo.toml packages/elf-service/src/search.rs
git commit -m '{"schema":"cmsg/1","type":"feat","scope":"global","summary":"Add cache key helpers","intent":"Generate stable LLM cache keys","impact":"Search can compute expansion and rerank cache keys","breaking":false,"risk":"low","refs":[]}'
```

---

### Task 4: Add Cache Payload Validation Helpers

**Files:**
- Modify: `packages/elf-service/src/search.rs`

**Step 1: Write failing test**

Add a unit test that rejects mismatched rerank payloads.

```rust
#[test]
fn rerank_cache_payload_rejects_mismatched_counts() {
	let payload = RerankCachePayload {
		items: vec![RerankCacheItem {
			note_id: uuid::Uuid::new_v4(),
			updated_at: time::OffsetDateTime::from_unix_timestamp(1).expect("Valid timestamp."),
			score: 0.5,
		}],
	};
	let candidates = vec![RerankCacheCandidate {
		note_id: uuid::Uuid::new_v4(),
		updated_at: time::OffsetDateTime::from_unix_timestamp(1).expect("Valid timestamp."),
	}];
	assert!(build_cached_scores(&payload, &candidates).is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p elf-service search::tests::rerank_cache_payload_rejects_mismatched_counts`

Expected: FAIL due to missing types and helper.

**Step 3: Implement minimal validation helpers**

Add payload structs and a validator that returns `Option<Vec<f32>>`.

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RerankCacheItem {
	note_id: uuid::Uuid,
	updated_at: time::OffsetDateTime,
	score: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct RerankCachePayload {
	items: Vec<RerankCacheItem>,
}

fn build_cached_scores(
	payload: &RerankCachePayload,
	candidates: &[RerankCacheCandidate],
) -> Option<Vec<f32>> {
	if payload.items.len() != candidates.len() {
		return None;
	}
	let mut map = std::collections::HashMap::new();
	for item in &payload.items {
		map.insert((item.note_id, item.updated_at), item.score);
	}
	let mut out = Vec::with_capacity(candidates.len());
	for candidate in candidates {
		let score = map.get(&(candidate.note_id, candidate.updated_at))?;
		out.push(*score);
	}
	Some(out)
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p elf-service search::tests::rerank_cache_payload_rejects_mismatched_counts`

Expected: PASS.

**Step 5: Commit**

```bash
git add packages/elf-service/src/search.rs
git commit -m '{"schema":"cmsg/1","type":"feat","scope":"global","summary":"Add cache payload validation","intent":"Validate rerank cache payloads before reuse","impact":"Cache hits skip invalid payloads","breaking":false,"risk":"low","refs":[]}'
```

---

### Task 5: Wire Cache into Expansion and Rerank Paths

**Files:**
- Modify: `packages/elf-service/src/search.rs`

**Step 1: Write failing test**

Add a unit test for the cache key prefix helper to ensure log safety.

```rust
#[test]
fn cache_key_prefix_is_stable() {
	let prefix = cache_key_prefix("abcd1234efgh5678");
	assert_eq!(prefix, "abcd1234efgh");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p elf-service search::tests::cache_key_prefix_is_stable`

Expected: FAIL due to missing helper.

**Step 3: Implement cache read/write and integrate**

Add helpers:

```rust
async fn fetch_cache_payload(
	pool: &sqlx::PgPool,
	kind: CacheKind,
	key: &str,
	now: time::OffsetDateTime,
) -> ServiceResult<Option<serde_json::Value>> { /* ... */ }

async fn store_cache_payload(
	pool: &sqlx::PgPool,
	kind: CacheKind,
	key: &str,
	payload: serde_json::Value,
	expires_at: time::OffsetDateTime,
	now: time::OffsetDateTime,
) -> ServiceResult<()> { /* ... */ }
```

Then integrate:

- In `expand_queries`, when `cfg.search.cache.enabled` is true, attempt cache read before calling the extractor provider.
- In `finish_search`, after filtering notes, build the rerank cache key. If cached scores are valid, skip provider call and reuse scores.
- On cache miss, call the provider and store the payload.
- Enforce `max_payload_bytes` on cache writes.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p elf-service search::tests::cache_key_prefix_is_stable`

Expected: PASS.

**Step 5: Commit**

```bash
git add packages/elf-service/src/search.rs
git commit -m '{"schema":"cmsg/1","type":"feat","scope":"global","summary":"Wire LLM cache into search","intent":"Reuse expansion and rerank outputs on repeated queries","impact":"Reduces repeated LLM calls while preserving scoring","breaking":false,"risk":"medium","refs":[]}'
```

---

### Task 6: Add Worker Cleanup for Expired Cache Rows

**Files:**
- Modify: `apps/elf-worker/src/worker.rs`

**Step 1: Write failing test**

Add a unit test for the cleanup SQL builder if needed, or skip test and rely on integration tests.

**Step 2: Implement cleanup**

Add a cleanup function similar to `purge_expired_traces`:

```rust
async fn purge_expired_cache(db: &Db, now: OffsetDateTime) -> Result<()> {
	sqlx::query("DELETE FROM llm_cache WHERE expires_at <= $1")
		.bind(now)
		.execute(&db.pool)
		.await?;
	Ok(())
}
```

Call it on an interval in the worker loop.

**Step 3: Run tests to ensure compilation**

Run: `cargo test -p elf-worker --lib`

Expected: PASS.

**Step 4: Commit**

```bash
git add apps/elf-worker/src/worker.rs
git commit -m '{"schema":"cmsg/1","type":"feat","scope":"global","summary":"Purge expired LLM cache entries","intent":"Keep cache table bounded","impact":"Worker deletes expired cache rows","breaking":false,"risk":"low","refs":[]}'
```

---

### Task 7: Final Verification

**Files:**
- None

**Step 1: Run full test suite**

Run: `cargo make test`

Expected: PASS (external integration tests may be ignored without Postgres/Qdrant).

**Step 2: Summarize behavior changes**

Document cache defaults, TTLs, and invalidation rules in the PR summary.
