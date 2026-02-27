# System Version Registry

Purpose: Provide a single registry for versioned identifiers used across ELF.

This document is normative. When a new versioned identifier is introduced, it must be added here.

## Registry

### HTTP API version

- Identifier: `/v2` (URL path prefix).
- Type: HTTP API version.
- Defined in: `apps/elf-api/src/routes.rs`, `docs/spec/system_elf_memory_service_v2.md`.
- Consumers: Clients calling the ELF Memory Service API, `apps/elf-mcp`.
- Bump rule: Introduce a new prefix (for example, `/v3`) only for breaking API contract changes. Add a new spec file and keep old specs stable.

### source_ref envelope schema

- Identifier: `source_ref/v1`.
- Type: `source_ref` JSON envelope schema identifier.
- Defined in: `docs/spec/system_elf_memory_service_v2.md`.
- Consumers: Note/event ingestion payloads, persisted `source_ref` fields, extensions and agents that hydrate evidence.
- Bump rule: Introduce `source_ref/v2` only when the envelope becomes incompatible with v1. Keep older identifiers immutable.

### source_ref envelope for docs_put

- Identifier: `doc_source_ref/v1`.
- Type: `docs_put.source_ref` JSON envelope schema identifier.
- Defined in: `docs/spec/system_doc_source_ref_v1.md`.
- Consumers: Docs ingestion (`POST /v2/docs`, MCP `elf_docs_put`) and any doc evidence consumers that need durable source provenance.
- Bump rule: Introduce `doc_source_ref/v2` only when the required/optional key contract becomes incompatible with v1. Keep older identifiers immutable.

### source_ref resolver: Doc Extension v1 doc pointer

- Identifier: `elf_doc_ext/v1`.
- Type: `source_ref.resolver` identifier for Doc Extension v1 pointers.
- Defined in: `docs/spec/system_source_ref_doc_pointer_v1.md`.
- Consumers: Agents that hydrate doc excerpts and build evidence-linked facts; Doc Extension v1 excerpt endpoints.
- Bump rule: Introduce `elf_doc_ext/v2` only when the dereference contract (required fields, semantics, or verification surface) becomes incompatible.

### Doc Extension v1 docs filters contract

- Identifier: `docs_search_filters/v1`.
- Type: Filter parameters and required Qdrant payload/index requirements for
  `docs_search_l0` (HTTP/MCP).
- Defined in: `docs/spec/system_doc_extension_v1_filters.md`.
- Consumers: `apps/elf-api/src/routes.rs`, `apps/elf-mcp/src/server.rs`, `apps/elf-service/src/docs.rs`.
- Bump rule: Introduce `docs_search_filters/v2` only if accepted filter keys,
  value constraints, evaluation semantics, or required Qdrant filter/index fields
  become incompatible.

### Doc Extension v1 payload/index contract

- Identifier: `doc_extension_payload/v1`.
- Type: Qdrant payload shape and required indexes for doc chunk points.
- Defined in: `docs/spec/system_doc_extension_v1_filters.md`.
- Consumers: `apps/elf-worker/src/worker.rs`, `apps/elf-service/src/docs.rs`.
- Bump rule: Introduce `doc_extension_payload/v2` only when payload shape changes break compatible filter deployment.

### Search ranking explain schema

- Identifier: `search_ranking_explain/v2`.
- Type: JSON schema identifier for `SearchExplain.ranking`.
- Defined in: `packages/elf-service/src/ranking_explain_v2.rs`.
- Consumers: Search responses, trace items (`explain` JSON), evaluation harness.
- Bump rule: Change the identifier only when the payload becomes incompatible with the previous version. Do not reuse older identifiers.
- Notes: The v2 model is additive. `final_score` must equal the sum of `terms[].value`.

### Search retrieval trajectory schema

- Identifier: `search_retrieval_trajectory/v1`.
- Type: JSON schema identifier for staged retrieval trajectory payloads.
- Defined in: `packages/elf-service/src/search.rs` (`SEARCH_RETRIEVAL_TRAJECTORY_SCHEMA_V1`).
- Consumers: Admin trajectory endpoint, trace summaries, item explain trajectory output, evaluation attribution.
- Bump rule: Change the identifier only for incompatible trajectory payload changes. Keep previous identifiers immutable.

### Recent traces admin list schema

- Identifier: `elf.recent_traces/v1`.
- Type: Admin trace list response payload identifier.
- Defined in: `packages/elf-service/src/search.rs` (`RECENT_TRACES_SCHEMA_V1`) and
  `docs/spec/system_elf_memory_service_v2.md`.
- Consumers: `GET /v2/admin/traces/recent` API response, `apps/elf-api`, `apps/elf-mcp`.
- Bump rule: Introduce a new identifier only if this response payload becomes incompatible.

### Trace bundle schema

- Identifier: `elf.trace_bundle/v1`.
- Type: Trace bundle response payload identifier for diagnostics.
- Defined in: `packages/elf-service/src/search.rs` (`TRACE_BUNDLE_SCHEMA_V1`) and
  `docs/spec/system_elf_memory_service_v2.md`.
- Consumers: `GET /v2/admin/traces/{trace_id}/bundle` API response, `apps/elf-api`, `apps/elf-mcp`.
- Bump rule: Introduce a new identifier only if this response payload becomes incompatible.

### Search filter expression schema

- Identifier: `search_filter_expr/v1`.
- Type: JSON envelope schema for structured search filters (`filter` request payload on search endpoints).
- Defined in: `docs/spec/system_search_filter_expr_v1.md`, `apps/elf-api/src/routes.rs`, `apps/elf-mcp/src/server.rs`, `packages/elf-service/src/search.rs` (`SearchFilter`).
- Consumers: Search creation endpoints (`/v2/search/quick`, `/v2/search/planned`, `/v2/admin/searches/raw`, `/v2/searches`) and admin/observability surfaces.
- Bump rule: Introduce `search_filter_expr/v2` only if filter field allowlist, operators, parsing limits, value typing, or parse error model become incompatible.

### Search filter impact schema

- Identifier: `search_filter_impact/v1`.
- Type: Search trajectory payload for filter outcome diagnostics.
- Defined in: `docs/spec/system_search_filter_expr_v1.md`, `packages/elf-service/src/search/filter.rs` (`SearchFilterImpact`), `packages/elf-service/src/search.rs` (`SearchFilterImpact::to_stage_payload`).
- Consumers: Search trajectory stage `recall.candidates` stage payload (`search_retrieval_trajectory/v1`).
- Bump rule: Introduce `search_filter_impact/v2` only when impact fields become incompatible.

### Doc retrieval trajectory schema

- Identifier: `doc_retrieval_trajectory/v1`.
- Type: JSON schema identifier for staged retrieval/excerpt diagnostics in doc endpoints.
- Defined in: `packages/elf-service/src/docs.rs` (`DOC_RETRIEVAL_TRAJECTORY_SCHEMA_V1`).
- Consumers: `DocsSearchL0Response` and `DocsExcerptResponse` when `explain=true`, MCP adapters forwarding doc routes.
- Bump rule: Change the identifier only when stage format or stage ordering semantics become incompatible.

### Ranking policy identifier

- Identifier: `ranking_v2:<hash>`.
- Type: Ranking policy identifier recorded in traces.
- Defined in: `packages/elf-service/src/search.rs`, `docs/spec/system_elf_memory_service_v2.md`.
- Consumers: Trace inspection, evaluation replay, debugging.
- Bump rule: If the policy encoding or semantics change in a way that makes old and new policies non-comparable, introduce a new prefix (for example, `ranking_v3:`).

### Search trace version

- Identifier: `trace_version` (integer), current value `3`.
- Type: Trace schema version for search traces.
- Defined in: `packages/elf-service/src/search.rs` (`TRACE_VERSION`), `sql/tables/006_search_traces.sql`.
- Consumers: Worker trace persistence, trace readers, evaluation harness.
- Bump rule: Increment only when a trace schema change requires explicit version gating in readers or replay logic.

### Embedding version

- Identifier: `embedding_version` (string), format `{provider_id}:{model}:{vector_dim}`.
- Type: Embedding compatibility identifier.
- Defined in: `packages/elf-service/src/lib.rs` (`embedding_version(cfg)`).
- Consumers: Postgres keys (`note_embeddings`, `note_chunk_embeddings`, outbox), Qdrant payload filtering, rebuild flows.
- Bump rule: This is not a numeric version. Treat the full string as an immutable identifier. A change to any component (`provider_id`, `model`, or `vector_dim`) produces a new `embedding_version`.

### LLM cache payload schema versions

- Identifier: `schema_version` (integer), `expansion` current value `1`, `rerank` current value `1`.
- Type: Cache payload schema version.
- Defined in: `packages/elf-service/src/search.rs` (`EXPANSION_CACHE_SCHEMA_VERSION`, `RERANK_CACHE_SCHEMA_VERSION`).
- Consumers: Search cache read and write paths.
- Bump rule: Increment when the cached payload shape changes such that older entries must be rejected or migrated.

## Repository process identifiers

### Commit message schema

- Identifier: `cmsg/1`.
- Type: Commit message schema identifier.
- Defined in: `AGENTS.md`.
- Consumers: Automated agents and repository tooling.
- Bump rule: Introduce `cmsg/2` only when the schema becomes incompatible with existing automation.
