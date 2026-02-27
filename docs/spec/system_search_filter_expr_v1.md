# System: Search Filter Expression Contract v1

Purpose: Define the structured filter payload used by search endpoints via `search_filter_expr/v1`.

Registry identifier:
- `search_filter_expr/v1`: Structured filter request envelope.

Status: active.

==================================================
Scope
==================================================

- Defines valid `filter` JSON wrappers for search request payloads.
- Defines allowed comparison operators and fields.
- Defines validation and parsing limits.
- Does not define ranking or retrieval algorithm details.

==================================================
1) Envelope
==================================================

`filter` MUST be an object with this exact shape:

```json
{
  "schema": "search_filter_expr/v1",
  "expr": {
    "op": "and|or|not|eq|neq|in|contains|gt|gte|lt|lte",
    "args|expr|field|value": "..."
  }
}
```

`schema` is required and must be exactly `search_filter_expr/v1`.
`expr` is required.

==================================================
2) Expression model
==================================================

Allowed operators:

- logical
  - `and`: logical AND of `args`.
  - `or`: logical OR of `args`.
  - `not`: logical NOT of `expr`.
- leaf comparisons
  - `eq`: equality.
  - `neq`: inequality.
  - `contains`: substring contains.
  - `in`: membership in an array.
  - `gt`, `gte`, `lt`, `lte`: numeric/date comparisons.

Node shapes:

- Logical:
  - `{ "op": "and", "args": [<node>, ...] }`
  - `{ "op": "or", "args": [<node>, ...] }`
  - `{ "op": "not", "expr": <node> }`
- Leaf:
  - `{ "op": "eq|neq|contains|gt|gte|lt|lte", "field": <field>, "value": <value> }`
  - `{ "op": "in", "field": <field>, "value": [<value>, ...] }`

`field` is required for all leaf ops.
`args`/`expr` are required for logical ops.

==================================================
3) Field allowlist
==================================================

Only these fields are allowed:

- `type`
- `key`
- `scope`
- `agent_id`
- `importance`
- `confidence`
- `updated_at`
- `expires_at`
- `hit_count`
- `last_hit_at`

Requests using any other field name are rejected as validation errors.

==================================================
4) Value constraints
==================================================

- `importance`, `confidence`, `hit_count`: JSON number.
- `updated_at`, `expires_at`, `last_hit_at`: RFC3339 datetime strings.
- `type`, `key`, `scope`, `agent_id`: strings (trimmed).
- `contains` values must be strings.
- `in` value must be array.

==================================================
2b) Filter impact payload
==================================================

When filter is provided, search trajectory payload `recall.candidates` includes:

```json
{
  "filter_impact": {
    "schema": "search_filter_impact/v1",
    "requested_candidate_k": 10,
    "effective_candidate_k": 30,
    "candidate_count_pre": 100,
    "candidate_count_post": 60,
    "dropped_total": 40,
    "top_drop_reasons": [
      { "reason": "eq:scope", "count": 20 },
      { "reason": "in:type", "count": 15 }
    ],
    "filter": {
      "schema": "search_filter_expr/v1",
      "expr": {
        "op": "eq",
        "field": "scope",
        "value": "project_shared"
      }
    }
  }
}
```

- `requested_candidate_k`: candidate_k passed by the caller.
- `effective_candidate_k`: internal candidate overfetch value when filter is present.
  `effective_candidate_k = min(MAX_CANDIDATE_K, requested_candidate_k * 3)` then clamped to be >= `top_k`.
- `candidate_count_pre`: candidates before filter evaluation (after consistency checks).
- `candidate_count_post`: candidates after filter evaluation.
- `dropped_total`: `candidate_count_pre - candidate_count_post`.
- `top_drop_reasons`: up to five reasons with highest drop counts, sorted by count desc then reason asc.
- `filter`: the validated filter payload that was evaluated.

==================================================
5) Parse/validation limits
==================================================

- Max depth: `<= 8`
- Max node count: `<= 128`
- `in` list limit: `<= 128`
- String size limit: UTF-8 bytes `<= 512`

Validation errors are reported as `Error::InvalidRequest` equivalents and include JSONPath in the
message (for example, `$.filter.expr[0].field` for bad field declarations).

==================================================
6) Error reporting
==================================================

Errors are actionable and include the exact JSONPath where validation failed.
Examples:
- `$.filter.expr`
- `$.filter.expr.value`
- `$.filter.expr.args[1]`

==================================================
7) Service-side application
==================================================

`search_filter_expr/v1` is evaluated after retrieval candidate generation and
Postgres consistency checks.

- It is **not** pushed down to Qdrant payload filters.
- It is **not** translated into SQL filters.
- It is evaluated against authoritative Postgres note metadata.
