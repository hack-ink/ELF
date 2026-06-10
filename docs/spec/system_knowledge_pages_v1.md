# Derived Knowledge Pages v1 Specification

Purpose: Define derived knowledge page storage, rebuild, citation, and lint contracts.
Status: normative
Read this when: You are implementing, validating, or reviewing project/entity/concept/issue/decision page rebuild behavior.
Not this document: Viewer integration, search ranking, live LLM page generation, or source-note mutation.
Defines: `elf.knowledge_page/v1` pages, sections, source refs, lint findings, and deterministic rebuild metadata.

## Core Rule

Knowledge pages are derived artifacts. They must never replace or mutate authoritative
notes, docs, event audits, graph facts, consolidation proposals, traces, or source
pointers.

Postgres remains the storage authority for both source memory and derived page records.
Knowledge pages are rebuildable from explicit source references and may be deleted or
rebuilt without changing source memory.

## Storage

The v1 storage tables are:

- `knowledge_pages`
- `knowledge_page_sections`
- `knowledge_page_source_refs`
- `knowledge_page_lint_findings`

`knowledge_pages.contract_schema` must be `elf.knowledge_page/v1`.

Allowed `knowledge_pages.page_kind` values:

- `project`
- `entity`
- `concept`
- `issue`
- `decision`

Allowed `knowledge_page_source_refs.source_kind` values:

- `note`
- `event`
- `relation`
- `proposal`

`event` currently means a durable `add_event` audit row in `memory_ingest_decisions`.

## Citation Contract

Every persisted page section must have at least one citation or an explicit
`unsupported_reason`.

Each citation must be persisted twice:

- in `knowledge_page_sections.citations` for section-local readback
- in `knowledge_page_source_refs` for normalized lint and stale-source detection

The normalized source ref must preserve:

- `source_kind`
- `source_id`
- source status when available
- source `updated_at` or equivalent freshness timestamp when available
- source content hash when available
- source snapshot metadata

## Rebuild Contract

The v1 rebuild path is deterministic for the same explicit source snapshot.

Rebuild input sources may include:

- active or historical `memory_notes`
- durable `add_event` audit rows from `memory_ingest_decisions`
- `graph_facts` plus `graph_fact_evidence`
- applied `consolidation_proposals`

Unreviewed consolidation proposals must not be used as source input for persisted pages.

`knowledge_pages.source_coverage` must include:

- `schema = "elf.knowledge_page.source_coverage/v1"`
- page kind and page key
- per-kind source counts
- total source count
- cited source count
- section count
- unsupported section count
- `coverage_complete`

`knowledge_pages.rebuild_metadata` must include:

- `schema = "elf.knowledge_page.rebuild/v1"`
- `source_snapshot_hash`
- `deterministic`
- `provider_metadata`
- `allowed_variance`

When future provider-backed or LLM-derived page text is persisted,
`rebuild_metadata.deterministic` must be false unless the provider output is fully
replayable from recorded metadata.

## Lint Contract

The v1 lint path compares stored normalized source refs with current source rows.

At minimum, lint must detect:

- missing source rows
- changed source status
- changed source freshness timestamp
- changed source content hash

Stale or missing source references must be stored in `knowledge_page_lint_findings`
with `finding_type = "stale_source_ref"` and enough `details` to show stored versus
current source state.

Lint findings are derived diagnostics. They must not mutate authoritative source
memory.

## Admin API

Minimal admin readback endpoints:

- `POST /v2/admin/knowledge/pages/rebuild`
- `GET /v2/admin/knowledge/pages`
- `GET /v2/admin/knowledge/pages/{page_id}`
- `POST /v2/admin/knowledge/pages/{page_id}/lint`

These endpoints are local admin/operator surfaces. They must not call LLM, embedding,
rerank, or external provider adapters in v1.
