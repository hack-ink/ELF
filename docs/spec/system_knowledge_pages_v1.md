---
type: Spec
title: "Derived Knowledge Pages v1 Specification"
description: "Define derived knowledge page storage, rebuild, citation, and lint contracts."
resource: docs/spec/system_knowledge_pages_v1.md
status: active
authority: normative
owner: spec
last_verified: 2026-06-22
tags:
  - docs
  - spec
source_refs: []
code_refs:
  - apps/elf-api/src/routes.rs
  - packages/elf-domain/src/knowledge.rs
  - packages/elf-service/src/knowledge.rs
  - packages/elf-storage/src/knowledge.rs
  - sql/tables/035_knowledge_pages.sql
  - sql/tables/037_knowledge_page_source_refs.sql
related: []
drift_watch:
  - docs/spec/system_knowledge_pages_v1.md
  - apps/elf-api/src/routes.rs
  - packages/elf-domain/src/knowledge.rs
  - packages/elf-service/src/knowledge.rs
  - packages/elf-storage/src/knowledge.rs
  - sql/tables/035_knowledge_pages.sql
  - sql/tables/037_knowledge_page_source_refs.sql
---
# Derived Knowledge Pages v1 Specification

Purpose: Define derived knowledge page storage, rebuild, citation, source-span, and lint contracts.
Status: normative
Read this when: You are implementing, validating, or reviewing project/entity/concept/issue/decision/author/timeline page rebuild behavior.
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
- `author`
- `timeline`

Allowed `knowledge_page_source_refs.source_kind` values:

- `doc`
- `doc_chunk`
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
- Source Library document id and chunk/span locator when `source_kind = "doc_chunk"`
- source status when available
- source `updated_at` or equivalent freshness timestamp when available
- source content hash when available
- source snapshot metadata

## Rebuild Contract

The v1 rebuild path is deterministic for the same explicit source snapshot.

Rebuild input sources may include:

- active Source Library `doc_documents`
- active Source Library `doc_chunks` as cited source spans
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
- `generated_by` metadata with actor agent id, runtime path, mode, and per-kind source input counts
- `version_identity` with schema `elf.knowledge_page.version_identity/v1`, page kind, page key, source snapshot hash, content hash, section hashes, and `source_mutation_allowed = false`
- `memory_candidate_policy` with schema `elf.knowledge_page.memory_candidate_policy/v1`, `review_required = true`, `review_surface = "consolidation_proposals"`, allowed memory-promotion apply intents, `direct_memory_ledger_mutation_allowed = false`, and `source_mutation_allowed = false`
- `provider_metadata`
- `allowed_variance`
- `previous_version_diff`

`previous_version_diff` must use schema `elf.knowledge_page.version_diff/v1`.
Initial rebuilds must set `available = false` and explain that no previous version
exists. Later rebuilds must set `available = true` and include previous and new
content/source hashes, title/source/content changed booleans, added/removed/changed/
unchanged section key lists and counts, a human-readable summary, and
`source_mutation_allowed = false`.

Previous-version diff metadata is rebuild readback metadata, not source content. Page
content hashes must not include `previous_version_diff`, `generated_by`,
`version_identity`, or `memory_candidate_policy`; otherwise repeating the same source
rebuild would appear nondeterministic solely because readback metadata changed.

When future provider-backed or LLM-derived page text is persisted,
`rebuild_metadata.deterministic` must be false unless the provider output is fully
replayable from recorded metadata.

## Changed-Source Watch/Rebuild Contract

The changed-source watch/rebuild path exposes the deterministic operational loop for
source changes. Its response schema is `elf.knowledge_page.watch_rebuild/v1`.

Input:

- `changed_sources`: non-empty list of source refs with `source_kind` and `source_id`.
- `page_kind`: optional page-kind filter.
- `limit`: optional affected-page limit.
- `generate_memory_candidates`: optional boolean, default `true`.

Behavior:

- The service must look up only knowledge pages that already cite one of the supplied
  changed source refs. It must not rebuild unrelated pages.
- For each affected page, the service must lint the currently stored page first, then
  rebuild from that page's stored normalized source refs and current authoritative
  source rows.
- A page that cannot resolve all stored sources or cannot rebuild must be returned as
  `blocked` with an operator-readable reason. Other page states are `changed`,
  `unchanged`, or `stale`.
- Per-section output must classify `changed`, `unchanged`, `stale`, or `blocked`
  sections. Classified outputs must include:
  - `stale_section` for stale or missing stored source snapshots.
  - `changed_claim` for sections whose derived content changed after rebuild.
  - `missing_citation` for citation or normalized backlink gaps.
  - `conflict` when a stale stored section also changes after current-source rebuild.
- Responses must include operator-readable summary lines with affected, changed,
  unchanged, stale, blocked, and memory-candidate counts.

The watch/rebuild path is a derived-artifact operation. It may update
`knowledge_pages`, `knowledge_page_sections`, `knowledge_page_source_refs`, and
`knowledge_page_lint_findings` for affected pages only. It must not mutate
authoritative source notes, docs, events, graph facts, traces, or source pointers.

## Lint Contract

The v1 lint path compares stored normalized source refs with current source rows.

At minimum, lint must detect:

- missing source rows
- changed source status
- changed source freshness timestamp
- changed source content hash
- persisted sections with no citations and no explicit unsupported reason
- persisted sections with an explicit unsupported reason
- sections whose citations have no normalized source backlinks
- page-level low source coverage where `coverage_complete` is false or the cited
  source count differs from the total source count

Stale or missing source references must be stored in `knowledge_page_lint_findings`
with `finding_type = "stale_source_ref"` and enough `details` to show stored versus
current source state.

Unsupported sections must be stored with `finding_type = "unsupported_claim"`.
Missing citations must use `finding_type = "missing_citation"`.
Missing normalized source backlinks must use `finding_type = "missing_source_ref"`.
Incomplete page coverage must use `finding_type = "low_source_coverage"`.
Every lint finding response must include repair or rebuild guidance. Guidance is
advisory and must not mutate source memory.

Lint findings are derived diagnostics. They must not mutate authoritative source
memory.

## Memory Candidate Boundary

Generated knowledge page content may feed memory candidates only through reviewable
consolidation proposals. Knowledge page rebuild, list, detail, search, and lint
readback must not insert, update, delete, deprecate, restore, or enqueue indexing for
`memory_notes`.

When a page section becomes candidate memory, the candidate must be represented as a
`consolidation_proposals` row with `contract_schema = "elf.consolidation/v1"` and
`apply_intent` of `create_derived_note` or `update_derived_note`. Applying that
proposal follows the Memory Promotion Apply Contract in
`system_consolidation_proposals_v1.md`.

Changed-source watch/rebuild may generate `MemoryCandidate` proposal payloads from
`changed_claim` or `conflict` knowledge deltas. These candidates must carry source
refs, source snapshots, a reason, a reviewable diff, and a proposed memory payload.
The service must route them through a queued consolidation run on the
`consolidation_proposals` review surface; it must not directly write the memory
ledger.

## Search and Viewer Readback

Knowledge page search is a derived-artifact readback surface, not the authoritative
note search surface. Page snippets may be shown beside search sessions only when they
are labeled as derived knowledge page snippets and include visible citation and source
coverage metadata.

Page search results must include:

- result type discriminator `knowledge_page_section`
- page id, page kind, page key, title, status, section id, section key, heading, role
- bounded section snippet
- section citations and normalized source backlinks
- page source coverage metadata
- rebuild metadata, including previous-version diff metadata when present
- lint summary and trust state that distinguishes clean, warning, error, and low
  coverage results
- a derived-result notice that source documents, spans, approved memory notes, event
  audits, relation facts, and applied proposals remain authoritative
- repair or rebuild guidance when lint or source coverage indicates stale,
  unsupported, missing, or weakly covered content

Knowledge page snippets must not be inserted into note search results as if they were
authoritative memory notes.

## Admin API

Minimal admin readback endpoints:

- `POST /v2/admin/knowledge/pages/rebuild`
- `POST /v2/admin/knowledge/pages/rebuild-changed-sources`
- `GET /v2/admin/knowledge/pages`
- `POST /v2/admin/knowledge/pages/search`
- `GET /v2/admin/knowledge/pages/{page_id}`
- `POST /v2/admin/knowledge/pages/{page_id}/lint`

These endpoints are local admin/operator surfaces. They must not call LLM, embedding,
rerank, or external provider adapters in v1.
