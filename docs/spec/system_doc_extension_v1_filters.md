# System: Document Extension v1 Filter and Payload Contract

Purpose: Define the `docs_search_filters/v1` filter contract for
`POST /v2/docs/search/l0` and MCP `elf_docs_search_l0`.

Registry identifiers:
- `docs_search_filters/v1`: API filter compatibility contract for `docs_search_l0`.
- `doc_extension_payload/v1`: Qdrant payload + index compatibility contract for doc chunks.

Status: shipped with Doc Extension v1.

==================================================
Scope
==================================================

- Defines filter parameters and Qdrant payload/index requirements for `docs_search_l0`.
- Does not define ranking, vector geometry, query text handling, or ingestion internals.

==================================================
1) Filter Parameters
==================================================

- `scope` (optional string): one of `agent_private`, `project_shared`, `org_shared`.
- `status` (optional string): defaults to `active` when omitted. Current implementation matches
  this value exactly against stored doc status (`active`/`deleted` in current schema).
- `doc_type` (optional string): exact-match filter.
- `sparse_mode` (optional string): retrieval fusion control mode:
  `auto` (default), `on`, `off`.
- `agent_id` (optional string): exact-match filter.
- `thread_id` (optional string): exact-match filter for `thread_id` payload field.
- `domain` (optional string): exact-match filter for `domain` payload field.
- `repo` (optional string): exact-match filter for `repo` payload field.
- `updated_after` (optional string): RFC3339 timestamp lower bound for `updated_at`.
- `updated_before` (optional string): RFC3339 timestamp upper bound for `updated_at`.
- `ts_gte` (optional string): RFC3339 timestamp lower bound for `doc_ts`.
- `ts_lte` (optional string): RFC3339 timestamp upper bound for `doc_ts`.
- Timestamp bounds are exclusive (`updated_after < updated_at < updated_before`), and values are parsed
  as timezone-aware RFC3339 datetimes.
- `ts_gte`/`ts_lte` bounds are inclusive (`ts_gte <= doc_ts <= ts_lte`), and values are parsed
  as timezone-aware RFC3339 datetimes.
- `level` on `POST /v2/docs/excerpts` is `L0|L1|L2` where `L0` is a compact 256-byte retrieval window.
- `explain` is an optional boolean on `docs_search_l0` and `docs_excerpts_get` responses that requests
  staged diagnostics.

Filter evaluation:
- Every supplied filter is combined with logical AND.
- `status` defaults to `active` when omitted.
- `sparse_mode` is validated as one of `auto|on|off` (default `auto`).
- `domain` requires `doc_type=search` and is rejected with `400` when used with other
  `doc_type` values or when `doc_type` is omitted.
- `repo` requires `doc_type=dev` and is rejected with `400` when used with other
  `doc_type` values or when `doc_type` is omitted.
- Invalid date values or `updated_after >= updated_before` are rejected with `400`.
- Invalid date values or `ts_gte >= ts_lte` are rejected with `400`.
- In `auto` sparse mode, sparse retrieval is enabled only when the query is judged as
  symbol-heavy / exact-match oriented; otherwise only dense retrieval is used.
- `sparse_mode=on` runs both dense and sparse retrieval; `sparse_mode=off` runs dense-only.

Response behavior:
- `docs_search_l0` always returns `trace_id`.
- `docs_excerpts_get` always returns `trace_id` and `locator`.
- When `explain=true`, both endpoints additionally return optional `trajectory` under
  `doc_retrieval_trajectory/v1`.

==================================================
2) Qdrant Payload Contract
==================================================

Each point used by `docs_search_l0` MUST include payload fields:
- `scope`
- `status`
- `doc_type`
- `agent_id`
- `thread_id`
- `domain`
- `repo`
- `updated_at`
- `doc_ts`

Payload field names are part of `docs_search_filters/v1` and `doc_extension_payload/v1` compatibility.

==================================================
3) Qdrant Index Requirements
==================================================

Implementations MUST provision payload indexes for:
- `scope` (keyword)
- `status` (keyword)
- `doc_type` (keyword)
- `agent_id` (keyword)
- `thread_id` (keyword)
- `domain` (keyword)
- `repo` (keyword)
- `updated_at` (datetime)
- `doc_ts` (datetime)

Indexing is a deploy-time requirement before filtered production traffic is enabled.
