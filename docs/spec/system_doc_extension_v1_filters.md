# System: Document Extension v1 Filter Contract

Purpose: Define the `docs_search_filters/v1` filter contract for
`POST /v2/docs/search/l0` and MCP `elf_docs_search_l0`.

## Scope

- Defines only filter parameters and Qdrant payload/index requirements for
  `docs_search_l0`.
- Does not define ranking, vector geometry, query text handling, or ingestion
  internals.

## 1) Filter Parameters

- `scope` (optional string): one of `agent_private`, `project_shared`, `org_shared`.
- `status` (optional string): defaults to `active`, allowed `active`, `deleted`.
- `doc_type` (optional string): exact-match filter.
- `agent_id` (optional string): exact-match filter.
- `updated_after` (optional string): RFC3339 lower bound on `updated_at`.
- `updated_before` (optional string): RFC3339 upper bound on `updated_at`.

Filter evaluation:
- Every supplied filter is combined with logical AND.
- `status` defaults to `active` when omitted.
- Invalid date values or `updated_after > updated_before` must be rejected with `400`.

## 2) Qdrant Payload Contract

Each point used by `docs_search_l0` MUST include payload fields:
- `scope`
- `status`
- `doc_type`
- `agent_id`
- `updated_at`

Payload field names are part of `docs_search_filters/v1` compatibility.

## 3) Qdrant Index Requirements

Implementations MUST provision payload indexes for:
- `scope` (keyword)
- `status` (keyword)
- `doc_type` (keyword)
- `agent_id` (keyword)
- `updated_at` (datetime)

Indexing is a deploy-time requirement before filtered production traffic is
enabled.
