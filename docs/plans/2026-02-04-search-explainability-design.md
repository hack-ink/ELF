# Search Explainability Outputs Design

Date: 2026-02-04

## Summary
This design adds persistent, query-scoped explainability for search results while keeping the default search response self-explanatory. Each search returns a `trace_id` plus a per-item `result_handle`. A trace payload is enqueued to an outbox and persisted asynchronously by the worker, keeping the hot path fast. The response embeds compact, stable explain data so developers can see ranking causes in a single response. A follow-up explain endpoint provides full trace context for inspection and future extension.

## Goals
- Provide component scores in the search response (retrieval score, rerank score, tie-breaker boost, final score).
- Surface matched terms and fields when available.
- Return a stable `result_handle` that can be used for follow-up inspection.
- Keep architecture clear and extensible for new ranking stages and explanations.

## Non-Goals
- Full relevance auditing or model-attribution introspection.
- Complex tokenization or language-specific matchers.

## Data Model
- `search_traces` stores the query context, expansion mode, expanded queries, allowed scopes, candidate count, configuration snapshot, and retention metadata.
- `search_trace_items` stores per-result explainability components and matched terms/fields.
- `search_trace_outbox` stores the serialized trace payload for asynchronous persistence.
- Traces are retained for `search.explain.retention_days` and cleaned by the worker.

## API
- `POST /v1/memory/search` response includes `trace_id`, `result_handle`, and `explain` with component scores and matches.
- `GET /v1/memory/search/explain?result_handle=...` returns the trace metadata plus the item explanation.

## Data Flow
1. Resolve scopes and expansion mode.
2. Run hybrid retrieval and capture fusion scores and ranks.
3. Rerank, compute tie-breaker boost, and final scores.
4. Compute matched terms/fields against text and key.
5. Enqueue the trace payload in `search_trace_outbox`.
6. The worker writes `search_traces` and `search_trace_items` asynchronously.
7. Return the enriched response.

## Error Handling
- Invalid or unknown `result_handle` returns `INVALID_REQUEST` with a clear message.
- Provider errors still surface as provider errors and skip trace writes.
- Explain data may be temporarily unavailable until the worker persists the trace payload.

## Testing
- Config validation covers `search.explain.retention_days`.
- Search path is exercised by existing acceptance tests; response fields are additive.

## Migration
- New tables: `search_traces`, `search_trace_items`, `search_trace_outbox` added to schema.
- No existing tables are modified.
