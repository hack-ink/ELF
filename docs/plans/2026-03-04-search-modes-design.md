# Search Modes: `quick_find` vs `planned_search` (Design)

Date: 2026-03-04

## Goal

Expose an explicit **latency-vs-quality** choice at search-creation time, while keeping the response contract deterministic and inspectable:

- `quick_find`: low-latency path for straightforward lookups.
- `planned_search`: higher-quality path that returns a machine-readable `query_plan`.

## Public API (v2)

### Create a search session

`POST /v2/searches`

Body:

```json
{
  "query": "English-only",
  "mode": "quick_find|planned_search",
  "top_k": 12,
  "candidate_k": 60,
  "filter": { "schema": "search_filter_expr/v1", "expr": { "op": "and", "args": [] } },
  "payload_level": "l0|l1|l2|null"
}
```

Response (single shape; `query_plan` present only for `planned_search`):

```json
{
  "trace_id": "uuid",
  "search_id": "uuid",
  "expires_at": "...",
  "mode": "quick_find|planned_search",
  "items": [ { "note_id": "uuid", "summary": "...", "final_score": 0.0 } ],
  "query_plan": { "schema": "elf.search.query_plan", "version": "v1" }
}
```

### Read a search session

`GET /v2/searches/{search_id}?top_k=12&touch=true`

- Returns the same response shape as create.
- `query_plan` is returned when present in the stored session (planned searches).

## Semantics

### `quick_find`

- Query expansion: **off**.
- Rerank provider call: **skipped** (deterministic placeholder scores), to keep latency predictable.
- Returns a compact index view; no `query_plan` field.

### `planned_search`

- Query expansion: follows configured expansion policy (`off|always|dynamic`).
- Rerank provider call: **on**.
- Returns `query_plan` (machine-readable retrieval plan + policy snapshot).

## Storage

Search sessions persist enough context to make `GET /v2/searches/{search_id}` reflect the creation response:

- `mode` (text, required)
- `query_plan` (jsonb, nullable; present for `planned_search`)

## MCP surface

The MCP server maps 1:1 to v2 endpoints and exposes a single creation tool:

- `elf_searches_create` → `POST /v2/searches` (requires `mode`)

## Evaluation / Acceptance

Latency can be benchmarked by running `elf-eval` in mode A vs mode B on the same dataset/config and comparing `latency_ms_p95`:

- Expectation: `quick_find` p95 < `planned_search` p95 on the same queries/environment.

