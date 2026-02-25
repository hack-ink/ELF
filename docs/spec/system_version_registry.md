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

### source_ref resolver: Doc Extension v1 doc pointer

- Identifier: `elf_doc_ext/v1`.
- Type: `source_ref.resolver` identifier for Doc Extension v1 pointers.
- Defined in: `docs/spec/system_source_ref_doc_pointer_v1.md`.
- Consumers: Agents that hydrate doc excerpts and build evidence-linked facts; Doc Extension v1 excerpt endpoints.
- Bump rule: Introduce `elf_doc_ext/v2` only when the dereference contract (required fields, semantics, or verification surface) becomes incompatible.

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
