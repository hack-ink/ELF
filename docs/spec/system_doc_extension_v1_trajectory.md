# System: Doc Extension v1 Retrieval Trajectory (`doc_retrieval_trajectory/v1`)

Purpose: Define the optional, response-only stage traces for Doc Extension v1 retrieval
(`docs_search_l0` and `docs_excerpts_get`) when `explain=true`.

This schema is intentionally lightweight and not persisted. It is returned directly in API
responses to support explainability and debugging.

==================================================
1) Schema
==================================================

- Identifier: `doc_retrieval_trajectory/v1`
- Type: JSON payload for response-only trajectory traces.
- Shape:

```json
{
  "schema": "doc_retrieval_trajectory/v1",
  "stages": [
    {
      "stage_order": 0,
      "stage_name": "request_validation",
      "stats": {}
    }
  ]
}
```

==================================================
2) Stage Names
==================================================

Endpoints:
- `POST /v2/docs/search/l0` (`DocsSearchL0Response`)
- `POST /v2/docs/excerpts` (`DocsExcerptResponse`)

Allowed/expected stage names (in order):

1. `request_validation`  
   Input validation and request-shape checks.

2. `query_embedding`  
   Embedding request preparation/dispatch.

3. `vector_dimension_check`  
   Ensures returned vector size matches the configured model/vector size.

4. `vector_search`  
   Dense and optional sparse retrieval from Qdrant.
   Dense retrieval runs first on every request; sparse retrieval is controlled by
   `sparse_mode` (`auto`, `on`, `off`).
   - `auto`: sparse retrieval only for symbol-heavy / exact-match style queries.
   - `on`: always run both dense and sparse retrieval.
   - `off`: dense-only retrieval.

5. `dedupe`  
   Chunk-id deduplication between retrieval tiers.

6. `chunk_lookup`  
   Document/chunk metadata hydration from Postgres.

7. `result_projection`  
   Final scored item projection and output truncation.  
   Implementations apply a recency tie-break using `updated_at` and expose the
   policy knobs in stage stats when available (`recency_tau_days`, `tie_breaker_weight`).

8. `level_selection` (excerpts only)  
   `L0|L1|L2` selection and byte budget.

9. `match_resolution` (excerpts only)  
   Selector resolution for `chunk_id` / `quote` / `position`.

10. `window_projection` (excerpts only)  
   Byte-window expansion to the requested level.

11. `verification` (excerpts only)  
   Verification flag/error summary and excerpt hash metadata.

Any implementation may choose to emit a subset of stages, but stage order must be stable
and `stage_name` values should be non-empty and meaningful for downstream readers.

==================================================
3) Examples
==================================================

```json
{
  "schema": "doc_retrieval_trajectory/v1",
  "stages": [
    {
      "stage_order": 0,
      "stage_name": "request_validation",
      "stats": { "query_len": 23, "top_k": 5, "candidate_k": 30 }
    },
    {
      "stage_order": 1,
      "stage_name": "vector_search",
      "stats": {
        "sparse_mode": "auto",
        "channels": ["dense"],
        "dense_raw_points": 24,
        "sparse_raw_points": 0,
        "raw_points": 24
      }
    },
    {
      "stage_order": 2,
      "stage_name": "result_projection",
      "stats": {
        "returned_items": 5,
        "pre_authorization_candidates": 8,
        "recency_tau_days": 60,
        "tie_breaker_weight": 0.12
      }
    }
  ]
}
```

==================================================
5) Evaluation Scenarios
==================================================

- English dense-first over mixed-language docs (expected dense-first)
  - Request `sparse_mode` omitted or `off` for a normal English query.
  - Example: natural-language question with low symbol density from mixed `chat/dev` content.
  - `trajectory.stages.vector_search` should show `channels=["dense"]` and `sparse_raw_points=0` (or absent).
  - `trajectory.stages.result_projection` should show normal ranking output and no symbolic jump from sparse-only terms.

- Exact-match cases (`auto` vs `on`)
  - Query contains symbols / identifiers (`/`, `:`, `#`, hex, URLs, error codes like `ERR_...`, full stack traces, full identifiers).
  - With `sparse_mode=auto`, expect `channels=["dense"]` for generic prose and `channels` may include `"sparse"` when the query is symbol-heavy.
  - With `sparse_mode=on`, expect `channels` to include both `"dense"` and `"sparse"` even if `auto` would stay dense-only.
  - Compare `vector_search.raw_points` and `result_projection` stability across modes for the same corpus; `sparse_mode=on` should improve retrieval of exact token patterns in symbol-heavy queries.

- Recency bias checks
  - Configure `cfg.ranking.recency_tau_days` and `cfg.ranking.tie_breaker_weight` > 0.
  - In `trajectory.stages.result_projection`, verify fields:
    - `recency_tau_days` (current effective value),
    - `tie_breaker_weight` (current effective weight),
    - `pre_authorization_candidates` and `returned_items`.
  - Expected signal: newer `updated_at` chunks should move upward when fusion scores are close and tie-break is active.

```json
{
  "schema": "doc_retrieval_trajectory/v1",
  "stages": [
    {
      "stage_order": 0,
      "stage_name": "request_validation",
      "stats": { "doc_id": "..." }
    },
    {
      "stage_order": 1,
      "stage_name": "match_resolution",
      "stats": { "selector_kind": "quote", "match_start": 84, "match_end": 120 }
    },
    {
      "stage_order": 2,
      "stage_name": "verification",
      "stats": { "verified": true, "error_count": 0 }
    }
  ]
}
```
