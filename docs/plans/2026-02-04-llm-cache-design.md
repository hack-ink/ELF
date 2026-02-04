# LLM Cache for Query Expansion and Reranking Design

Date: 2026-02-04

## Summary

This design adds a Postgres-backed cache for LLM query expansion and reranking. The cache reduces repeated LLM calls while keeping behavior consistent across service instances. Cache keys are derived from the minimal correctness inputs and hashed with BLAKE3 (256-bit). Entries are stored with TTL metadata and cleaned by the worker.

## Goals

- Reduce latency and cost for repeated queries.
- Keep outputs consistent for identical inputs.
- Avoid new infrastructure dependencies.

## Non-Goals

- Cross-query semantic reuse or approximate caching.
- Caching of final scores that depend on time-based decay.
- Introducing a distributed cache service.

## Data Model

Add `llm_cache` with:

- `cache_id` (uuid, primary key)
- `cache_kind` (text: `expansion` or `rerank`)
- `cache_key` (text, unique with `cache_kind`)
- `payload` (jsonb)
- `created_at`, `last_accessed_at` (timestamptz)
- `expires_at` (timestamptz)
- `hit_count` (bigint)

Indexes: unique `(cache_kind, cache_key)` and `expires_at` for cleanup.

## Cache Keys

Keys are derived from a structured payload and hashed with BLAKE3 (256-bit) to a hex string.

### Expansion

Include: normalized query, provider id, model, temperature, `expansion_version`, and config fields that affect output (`max_queries`, `include_original`).

### Rerank

Include: normalized query, provider id, model, `rerank_version`, and an ordered candidate signature. The candidate signature is the ordered list of `note_id` and `updated_at` pairs for the notes sent to the rerank provider.

## Data Flow

### Expansion

1. If expansion is triggered, compute the expansion cache key.
2. On hit, deserialize cached queries and proceed.
3. On miss, call the LLM, normalize queries, and insert the cache entry with TTL.

### Rerank

1. After candidate filtering, compute the rerank cache key.
2. On hit, validate candidate signature and score count, then reuse scores.
3. On miss, call the rerank provider, then cache scores aligned to candidate order.

Tie-breaker and final scores are computed per request to preserve recency effects.

## Invalidation and Retention

- Changing `expansion_version` or `rerank_version` invalidates prior entries.
- Changes in provider model or relevant config fields change the key.
- Candidate updates change the signature via `updated_at`.
- Entries expire after configurable TTLs.

## Error Handling

Cache failures are treated as misses. Errors are logged with structured fields and do not fail the search request.

## Configuration

Add `search.cache`:

- `enabled` (bool)
- `expansion_ttl_days` (i64)
- `rerank_ttl_days` (i64)
- `max_payload_bytes` (u64, optional)
- `expansion_version` (string)
- `rerank_version` (string)

Defaults: enabled, 7 days TTL, version strings set to `v1`.

## Observability

Log cache hits and misses with `cache_kind`, `cache_key_prefix`, `hit`, `payload_size`, and `ttl_days`.

## Testing

- Unit tests for cache key construction and version invalidation.
- Unit tests for payload validation (candidate signature, score counts).
- Service tests for cache hit/miss behavior for expansion and rerank.

## Migration

- Add `sql/tables/008_llm_cache.sql` and include it in schema initialization.
- Add worker cleanup for expired cache entries.
