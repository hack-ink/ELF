---
type: Spec
title: "Work Journal v1 Specification"
description: "Define source-adjacent Work Journal capture and readback without authoritative memory promotion."
resource: docs/spec/system_work_journal_v1.md
status: active
authority: normative
owner: spec
last_verified: 2026-06-27
tags:
  - docs
  - spec
  - work-journal
source_refs:
  - https://linear.app/hackink/issue/XY-1117
code_refs:
  - packages/elf-service/src/work_journal.rs
  - packages/elf-storage/src/work_journal.rs
  - sql/tables/042_work_journal_entries.sql
  - apps/elf-api/src/routes.rs
  - apps/elf-mcp/src/server.rs
related:
  - docs/spec/system_elf_memory_service_v2.md
  - docs/spec/system_consolidation_proposals_v1.md
  - docs/spec/system_recall_debug_panel_v1.md
drift_watch:
  - packages/elf-service/src/work_journal.rs
  - sql/tables/042_work_journal_entries.sql
  - apps/elf-api/src/routes.rs
  - apps/elf-mcp/src/server.rs
---
# Work Journal v1 Specification

Purpose: Define source-adjacent Work Journal capture and readback without authoritative memory promotion.
Status: normative
Read this when: You are implementing, validating, or using Work Journal session logs, handoff briefs, janitor reports, next-step captures, rejected-option captures, or readback.
Not this document: Memory Note authority, Source Library document indexing, Dreaming proposal review, or benchmark scoreboard contracts.
Defines: Work Journal v1 storage, entry families, redaction, source refs, promotion boundary, and readback behavior.

## Boundary

Work Journal is source-adjacent session evidence. It is not Memory Authority, not
Knowledge Workspace output, and not an archival search source.

Work Journal rows must not:

- create or update `memory_notes`;
- write `indexing_outbox`;
- create Qdrant points;
- answer current-fact questions as authoritative memory unless the response also
  carries an accepted Memory Authority or Dreaming Review promotion reference.

## Storage

The source-of-truth table is `work_journal_entries`.

Required row fields:

- `entry_id`: stable UUID for the journal entry.
- `tenant_id`, `project_id`, `agent_id`, `scope`: normal ELF ownership and visibility context.
- `session_id`: stable session grouping key.
- `family`: one of the canonical entry families below.
- `status`: lifecycle state; v1 current readback uses `active`.
- `title`: optional operator-facing label.
- `body`: durable redacted journal text.
- `source_refs`: non-empty JSON array of non-empty JSON object source references supporting the entry.
- `explicit_next_steps`: JSON array of source-stated next steps.
- `inferred_next_steps`: JSON array of non-authoritative inferred next-step hints.
- `rejected_options`: JSON array of rejected options.
- `promotion_boundary`: normalized promotion metadata.
- `redaction_audit`: write-policy audit for the stored body.
- `created_at`, `updated_at`: timestamps.

The table enforces storage-level checks for canonical `scope`, `family`, and `status`
values plus JSON shape checks for `source_refs`, side lists, `promotion_boundary`,
and `redaction_audit`.

## Entry Families

Canonical `family` values:

- `session_log`
- `handoff_brief`
- `janitor_report`
- `explicit_next_step`
- `inferred_next_step`
- `rejected_option`

Equivalent UI labels may be displayed, but API, MCP, storage, tests, and benchmark
fixtures must use the canonical values above.

## Write Rules

`POST /v2/work-journal/entries` captures one entry.

Request-controlled fields:

- `entry_id` optional UUID. When omitted, the service creates one.
- `scope`, `session_id`, `family`, `title`, `body`, `source_refs`.
- `write_policy` with the same exclusion/redaction shape used by note and document writes.
- `explicit_next_steps`, `inferred_next_steps`, `rejected_options`.
- `promotion_boundary`.

Rules:

- `source_refs` must be a non-empty JSON array of non-empty JSON objects.
- The English gate applies to body, title, list text, and identifier-like source-ref strings.
- `write_policy` is applied before persistence.
- If durable `body` or list text still contains secret markers after write-policy application,
  the request is rejected.
- Shared-scope rows use normal ELF shared-grant behavior for readback.

## Promotion Boundary

The normalized `promotion_boundary` object uses schema
`elf.work_journal.promotion_boundary/v1`.

Required normalized fields:

- `schema = "elf.work_journal.promotion_boundary/v1"`
- `journal_entry_authority = "source_adjacent_only"`
- `authoritative_memory_allowed`
- `promotion_required_for_current_facts`
- `accepted_memory_authority_ref`
- `accepted_dreaming_review_ref`
- `requested_authoritative_memory_allowed`

`authoritative_memory_allowed` may be true only when either
`accepted_memory_authority_ref` or `accepted_dreaming_review_ref` is present and
matches a supported accepted-reference shape.
If a caller requests authority without accepted promotion evidence, readback must preserve
that request in `requested_authoritative_memory_allowed` while keeping
`authoritative_memory_allowed = false`.

In v1, supported accepted-reference shapes are intentionally narrow:

- `accepted_memory_authority_ref`: JSON object with
  `schema = "elf.memory_record_ref/v1"`, `kind = "note"`, UUID `id`, and
  `status = "active"`.
- `accepted_dreaming_review_ref`: JSON object with
  `schema = "elf.dreaming_review_queue/v1"`, UUID `proposal_id`, and
  `review_state` of `approved` or `applied`.

Primitive values, empty objects, wrong schemas, and incomplete objects are invalid
accepted references and must not make Work Journal authoritative.
Syntactically valid accepted references are not sufficient by themselves. The service
must resolve `accepted_memory_authority_ref` to an active, readable Memory Authority
note and `accepted_dreaming_review_ref` to an existing same-tenant/project Dreaming
Review or consolidation proposal in `approved` or `applied` state before setting
`authoritative_memory_allowed = true`.

## Readback

HTTP routes:

- `GET /v2/work-journal/entries/{entry_id}`
- `POST /v2/work-journal/readback`

MCP tools:

- `elf_work_journal_entry_create`
- `elf_work_journal_entry_get`
- `elf_work_journal_session_readback`

Session readback returns `elf.work_journal/v1` with newest-first `items` and an optional
`where_stopped` projection. `where_stopped` includes the latest entry id and family,
latest source refs, most recent returned explicit next steps, most recent returned
inferred next steps, most recent returned rejected options, and the latest promotion
boundary.

Current-fact answers must route through accepted memory, graph, knowledge, or reviewed
Dreaming authority. Work Journal readback may answer "where did we stop?" with journal
evidence, but journal-only content remains source-adjacent.
