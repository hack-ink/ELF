---
type: Drift Audit
title: "Privacy, Delete, Export, and Retention Boundaries Drift Audit"
description: "Drift audit for current-recall suppression across Source Library, Knowledge Workspace, graph-lite facts, and relation context."
resource: docs/evidence/2026-06-23-privacy-delete-export-boundaries-drift-audit.md
status: active
authority: evidence
owner: evidence
last_verified: 2026-06-23
tags:
  - docs
  - evidence
  - privacy
  - retention
source_refs:
  - docs/runbook/privacy_delete_export.md
code_refs:
  - packages/elf-service/src/graph_query.rs
  - packages/elf-service/src/graph_report.rs
  - packages/elf-service/src/knowledge.rs
  - packages/elf-service/src/search.rs
  - packages/elf-storage/src/knowledge.rs
  - packages/elf-service/tests/acceptance/graph_ingestion.rs
  - packages/elf-service/tests/acceptance/knowledge_pages.rs
related:
  - docs/spec/system_doc_source_ref_v1.md
  - docs/spec/system_elf_memory_service_v2.md
  - docs/spec/system_graph_memory_postgres_v1.md
  - docs/spec/system_knowledge_pages_v1.md
drift_watch:
  - docs/runbook/privacy_delete_export.md
  - docs/spec/system_doc_source_ref_v1.md
  - docs/spec/system_elf_memory_service_v2.md
  - docs/spec/system_graph_memory_postgres_v1.md
  - docs/spec/system_knowledge_pages_v1.md
  - packages/elf-service/src/graph_query.rs
  - packages/elf-service/src/graph_report.rs
  - packages/elf-service/src/knowledge.rs
  - packages/elf-service/src/search.rs
  - packages/elf-storage/src/knowledge.rs
---
# Privacy, Delete, Export, and Retention Boundaries Drift Audit

Purpose: Record the code and test evidence behind the privacy/delete/export boundary
docs added for XY-1078.
Read this when: You need to verify whether docs for source deletion, private spans,
graph evidence suppression, and export boundaries match current code.
Not this document: A legal compliance assessment, provider terms review, or raw
benchmark report.
Evidence for: `docs/runbook/privacy_delete_export.md` and the related Source
Library, Knowledge Workspace, graph memory, and core service specs.

## Claims Checked

- Source Library direct and derived readback uses current active source rows for
  recallable snippets.
- Knowledge Workspace page search suppresses snippets whose normalized source refs
  are deleted, expired, unreadable, ignored, rejected, unapplied, or contain
  non-captured spans.
- Graph query, graph report, and search relation context return facts only when
  current readable evidence notes exist and omit deleted or unreadable evidence ids.
- Delete and forget docs distinguish current-recall suppression from retained
  provenance, history, trace, and benchmark evidence.
- Export docs route through authorized read APIs and do not describe a bypass around
  scope, payload level, or write-policy spans.

## Implementation Evidence

- `packages/elf-storage/src/knowledge.rs` now resolves Knowledge Workspace note,
  event, relation, document, and chunk sources through active/readable source rows.
- `packages/elf-service/src/knowledge.rs` resolves current source keys before page
  search and suppresses sections with non-recallable source refs or non-captured
  spans.
- `packages/elf-service/src/graph_query.rs` and
  `packages/elf-service/src/graph_report.rs` require active, unexpired, readable
  graph evidence notes for fact readback.
- `packages/elf-service/src/search.rs` filters relation-context evidence notes to
  active, unexpired, readable notes and drops malformed relation rows with no
  evidence ids.

## Test Evidence

- `packages/elf-service/src/knowledge.rs` has pure coverage for deleted, ignored,
  missing, and non-captured source refs.
- `packages/elf-service/src/graph_query.rs` has pure coverage for suppressing graph
  rows without readable evidence.
- `packages/elf-service/src/graph_report.rs` has pure coverage for suppressing graph
  report facts without readable evidence.
- `packages/elf-service/src/search.rs` has pure coverage for suppressing relation
  context rows without evidence.
- `packages/elf-service/tests/acceptance/knowledge_pages.rs` adds an ignored
  integration case for Source Library document deletion suppressing page search.
- `packages/elf-service/tests/acceptance/graph_ingestion.rs` adds an ignored
  integration case for memory-note delete suppressing graph query readback.

## Residual Boundaries

- Provenance, note history, recall traces, and checked benchmark artifacts are audit
  evidence, not current recall. They may retain historical ids or snippets until
  their own retention or purge path runs.
- Provider retention remains outside ELF control once content is sent to external
  embedding, rerank, or LLM extractor providers.
- The runbook does not claim a universal public erase endpoint. Operators must act on
  the explicit authority surface and verify derived projections.
