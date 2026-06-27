---
type: Drift Audit
title: "Work Journal Drift Audit"
description: "Drift audit for source-adjacent Work Journal capture, readback, and promotion-boundary behavior."
resource: docs/evidence/2026-06-27-work-journal-drift-audit.md
status: active
authority: current_state
owner: docs
last_verified: 2026-06-27
tags:
  - docs
  - drift-audit
  - work-journal
source_refs:
  - https://linear.app/hackink/issue/XY-1117
code_refs:
  - packages/elf-service/src/work_journal.rs
  - packages/elf-storage/src/work_journal.rs
  - packages/elf-storage/src/models.rs
  - sql/tables/042_work_journal_entries.sql
  - apps/elf-api/src/routes.rs
  - apps/elf-mcp/src/server.rs
  - packages/elf-service/tests/acceptance/work_journal.rs
related:
  - docs/spec/system_work_journal_v1.md
  - docs/spec/system_elf_memory_service_v2.md
  - docs/spec/system_version_registry.md
drift_watch:
  - packages/elf-service/src/work_journal.rs
  - packages/elf-storage/src/work_journal.rs
  - packages/elf-storage/src/models.rs
  - sql/tables/042_work_journal_entries.sql
  - apps/elf-api/src/routes.rs
  - apps/elf-mcp/src/server.rs
  - docs/spec/system_work_journal_v1.md
---
# Work Journal Drift Audit

Purpose: Anchor the Work Journal v1 source-adjacent capture and readback contract to
the current service, storage, HTTP, MCP, and test surfaces.
Read this when: You need evidence behind Work Journal session logs, handoff briefs,
janitor reports, next-step captures, rejected options, source refs, redaction, or
promotion-boundary behavior.
Not this document: Memory Authority note promotion, Dreaming Review proposal scoring,
or benchmark competitor interpretation.

## Watched Claims

- `work_journal_entries` is the source-of-truth table for Work Journal rows.
- Work Journal entries use stable UUID `entry_id` values and session grouping through
  `session_id`.
- Canonical families are `session_log`, `handoff_brief`, `janitor_report`,
  `explicit_next_step`, `inferred_next_step`, and `rejected_option`.
- Work Journal capture requires non-empty object source refs and stores redacted
  durable journal output plus redaction audit.
- Work Journal readback exposes one-entry lookup and session-level `where_stopped`
  evidence without writing authoritative memory notes, indexing outbox rows, search
  sessions, traces, or Qdrant points.
- `authoritative_memory_allowed` is true only for supported accepted Memory Authority
  or Dreaming Review references that resolve to current storage evidence; caller-
  requested authority without accepted promotion evidence remains source-adjacent only.
- HTTP and MCP expose the Work Journal create, get, and session readback surfaces.

## Evidence Anchors

- `sql/tables/042_work_journal_entries.sql` defines the Work Journal table and
  storage-level scope, family, status, JSON-shape checks, and session/scope indexes.
- `packages/elf-storage/src/models.rs` and `packages/elf-storage/src/work_journal.rs`
  own row shape, insert, lookup, and newest-first session queries.
- `packages/elf-service/src/work_journal.rs` owns validation, write-policy redaction,
  accepted-reference normalization and storage resolution, read authorization, and
  `where_stopped` shaping.
- `apps/elf-api/src/routes.rs` exposes:
  - `POST /v2/work-journal/entries`
  - `GET /v2/work-journal/entries/{entry_id}`
  - `POST /v2/work-journal/readback`
- `apps/elf-mcp/src/server.rs` exposes:
  - `elf_work_journal_entry_create`
  - `elf_work_journal_entry_get`
  - `elf_work_journal_session_readback`
- `packages/elf-service/tests/acceptance/work_journal.rs` checks persistence,
  redacted readback, source refs, `where_stopped`, and no Memory Ledger or indexing
  side effects.

## Reverse Checks

- Run `cargo make check-docs` after Work Journal docs changes.
- Run the focused Work Journal service tests after service validation or readback
  changes.
- Run the registered repository gate before review handoff.
- Re-check `elf.memory_record_ref/v1`, Memory Authority readability, and Dreaming
  Review accepted-reference resolution if Memory Authority or Dreaming Review target
  refs change.
- Re-check HTTP OpenAPI and MCP tool registration if Work Journal route or tool names
  change.

## Verdict

pass

## Required Updates

- Update `docs/spec/system_work_journal_v1.md`, this drift audit, and
  `docs/spec/system_version_registry.md` when Work Journal fields, canonical family
  values, accepted-reference shapes or storage resolution rules, readback ordering,
  or `where_stopped` semantics change.
- Do not treat Work Journal readback as current-fact authority unless the response is
  backed by accepted Memory Authority, graph, Knowledge Workspace, or reviewed
  Dreaming evidence.

## Citations

- `sql/tables/042_work_journal_entries.sql`
- `packages/elf-storage/src/work_journal.rs`
- `packages/elf-service/src/work_journal.rs`
- `apps/elf-api/src/routes.rs`
- `apps/elf-mcp/src/server.rs`
- `packages/elf-service/tests/acceptance/work_journal.rs`
