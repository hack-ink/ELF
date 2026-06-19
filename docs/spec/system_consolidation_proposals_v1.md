---
type: Spec
title: "Consolidation Proposals v1 Specification"
description: "Define the reviewable consolidation run and proposal contract for derived memory output."
resource: docs/spec/system_consolidation_proposals_v1.md
status: active
authority: normative
owner: spec
last_verified: 2026-06-18
tags:
  - docs
  - spec
source_refs: []
code_refs: []
related: []
drift_watch:
  - docs/spec/system_consolidation_proposals_v1.md
---
# Consolidation Proposals v1 Specification

Purpose: Define the reviewable consolidation run and proposal contract for derived memory output.
Status: normative
Read this when: You are implementing, validating, or reviewing dreaming-inspired consolidation storage, jobs, proposals, or review flows.
Not this document: Live LLM consolidation generation, viewer UI behavior, retrieval observability panels, or agentmemory import adapters.
Defines: `elf.consolidation/v1` runs, proposals, source snapshots, lineage, review lifecycle, and source immutability rules.
Also defines: `elf.dreaming_review_queue/v1` readback as a policy view over
consolidation proposals.

Related inputs:

- `docs/decisions/2026-06-08-agent-memory-selection.md`
- `docs/evidence/external_memory/comparison_external_projects.md`
- `docs/spec/system_elf_memory_service_v2.md`

## Core Rule

Consolidation output is derived and reviewable. It must never destructively rewrite
authoritative source notes, events, docs, traces, graph facts, or search traces.

The authoritative source-of-truth remains the ELF Core storage defined by
`docs/spec/system_elf_memory_service_v2.md`. Consolidation stores proposals over
immutable input snapshots. A proposal may later create or update a derived artifact,
but source evidence remains inspectable and unchanged.

## Contract Schema

Canonical schema identifier:

```text
elf.consolidation/v1
```

Every persisted run and proposal must carry `contract_schema = "elf.consolidation/v1"`.

## Source References

`source_refs` is a non-empty array of immutable input pointers.

Each item has:

- `kind`: one of `note`, `event`, `trace`, `trace_item`, `doc`, `doc_chunk`
- `id`: UUID of the referenced source artifact
- `snapshot`: source snapshot metadata captured before proposal storage

`snapshot` must contain at least one freshness or replay guard:

- `status`
- `updated_at`
- `content_hash`
- `embedding_version`
- `trace_version`
- non-empty `source_ref`
- non-empty `metadata`

`source_ref` and `metadata` must be JSON objects.

## Run Contract

Storage table: `consolidation_runs`.

Required fields:

- `run_id`
- `tenant_id`
- `project_id`
- `agent_id`
- `contract_schema`
- `job_kind`
- `status`
- `input_refs`
- `source_snapshot`
- `lineage`
- `error`
- `created_at`
- `updated_at`
- `completed_at`

`job_kind` identifies how the run was registered, for example `fixture`, `manual`, or
future `scheduled`. This issue only permits fixture-driven or manually supplied
proposal payloads. It does not permit live provider generation.

Run states:

- `pending`
- `running`
- `completed`
- `failed`
- `cancelled`

Allowed run transitions:

- `pending -> running`
- `pending -> cancelled`
- `running -> completed`
- `running -> failed`
- `running -> cancelled`

Terminal states are `completed`, `failed`, and `cancelled`.

## Worker Job Contract

Storage table: `consolidation_run_jobs`.

The first runtime implementation is queue-backed and deterministic. Creating a
fixture or manual consolidation run stores the immutable run input snapshot, enqueues
one worker job, and returns the run plus `job_id`. The worker materializes queued
proposal payloads into `consolidation_proposals`; API creation must not call LLM,
embedding, rerank, or external provider adapters.

Required fields:

- `job_id`
- `run_id`
- `tenant_id`
- `project_id`
- `agent_id`
- `job_kind`
- `status`
- `payload`
- `attempts`
- `last_error`
- `available_at`
- `created_at`
- `updated_at`

Job states:

- `PENDING`
- `CLAIMED`
- `DONE`
- `FAILED`

`payload` is a JSON object with:

- `contract_schema = "elf.consolidation/v1"`
- `proposals`: array of proposal contracts matching this spec

Worker rules:

- Claim one due `PENDING`, expired `CLAIMED`, or retryable `FAILED` job with a lease.
- Validate `payload.contract_schema` and every proposal before persistence.
- Transition the run through `pending -> running -> completed` when materialization
  succeeds.
- Insert proposals with `review_state = proposed`.
- Mark the job `DONE` in the same transaction as the proposal and run-state writes.
- On failure, mark the job `FAILED`, increment attempts, preserve a bounded error, and
  schedule retry.
- Never mutate authoritative source notes, events, docs, traces, graph facts, or
  search traces.

## Proposal Contract

Storage table: `consolidation_proposals`.

Required fields:

- `proposal_id`
- `run_id`
- `tenant_id`
- `project_id`
- `agent_id`
- `contract_schema`
- `proposal_kind`
- `apply_intent`
- `review_state`
- `source_refs`
- `source_snapshot`
- `lineage`
- `diff`
- `confidence`
- `unsupported_claim_flags`
- `contradiction_markers`
- `staleness_markers`
- `target_ref`
- `proposed_payload`
- `reviewer_agent_id`
- `review_comment`
- `reviewed_at`
- `created_at`
- `updated_at`

`confidence` must be finite and in the inclusive range `0.0..=1.0`.

`lineage` must include non-empty `source_refs`. It may also include `parent_run_id`
and `parent_proposal_ids`.

`unsupported_claim_flags` is a reviewer prompt array. Each flag has:

- `claim_id`: optional stable claim identifier
- `message`: non-empty reviewer-facing text
- `source`: optional source reference

`contradiction_markers` and `staleness_markers` are review prompts. Each marker has:

- `severity`: `low`, `medium`, or `high`
- `message`: non-empty reviewer-facing text
- `source`: optional source reference

## Diff And Apply Intent

`diff` is a JSON object with:

- `summary`: non-empty text
- `before`: JSON object
- `after`: JSON object

The diff must describe a derived output change. It must not include source mutation
keys such as `source_mutation`, `source_mutations`, `source_note_updates`,
`delete_source`, `delete_sources`, `source_delete`, or `overwrite_source`.

Allowed `apply_intent` values:

- `create_derived_note`
- `update_derived_note`
- `create_derived_knowledge_page`
- `update_derived_knowledge_page`
- `create_derived_graph_view`
- `no_op`

No `apply_intent` may update, delete, overwrite, or deprecate authoritative source
notes, docs, events, traces, or graph facts.

## Review Lifecycle

Review states:

- `proposed`
- `approved`
- `rejected`
- `applied`
- `archived`

Allowed review transitions:

- `proposed -> approved`
- `proposed -> rejected`
- `proposed -> archived`
- `approved -> applied`
- `approved -> rejected`
- `approved -> archived`

Terminal states are `rejected`, `applied`, and `archived`.

`applied` means the proposal has been approved and marked as applied to the derived
target. It does not mean authoritative source memory was changed.

Operator review actions map to the lifecycle states:

- `approve`: `proposed -> approved`
- `apply`: `approved -> applied`, or `proposed -> approved -> applied` with both
  transitions audited
- `discard`: `proposed|approved -> rejected`
- `defer`: `proposed|approved -> archived`

Every review transition must write an append-only audit event with proposal id, run id,
reviewer agent id, action, prior state, next state, optional comment, and timestamp.

## Service Boundary

The first implementation exposes fixture-driven service flows:

- create a consolidation run with optional proposal payloads and queued worker `job_id`
- list consolidation runs
- get a consolidation run
- list consolidation proposals
- get a consolidation proposal
- transition proposal review state through `approve`, `apply`, `discard`, and `defer`
  actions with review-event readback

These flows must not call LLM, embedding, rerank, or external provider adapters.

## Dreaming Review Queue

The Dreaming review queue is a readback and policy surface over
`consolidation_proposals`. It does not create a new source-of-truth table and does not
run provider generation. The canonical response schema is:

```text
elf.dreaming_review_queue/v1
```

Queue items must expose:

- proposal id, run id, proposal kind, queue variant, apply intent, and review state
- `source_refs` and `source_snapshot`
- `target_ref` and derived `affected_refs`
- `confidence`
- `unsupported_claim_flags`, contradiction markers, and staleness markers
- reviewable `diff`
- proposed derived payload
- per-item policy readback
- current review state, available review actions, reviewer metadata, and append-only
  review events

The queue variant is inferred from explicit proposal payload metadata first, then from
`proposal_kind`, then from `apply_intent`. The required supported variants are:

- `memory_summary`
- `proactive_brief`
- `scheduled_memory`
- `tag`
- `duplicate_merge`
- `page_rebuild`
- `memory_promotion`
- `graph_fact`
- `correction`

Policy rules:

- `source_mutation_allowed` must be `false`.
- Source mutation keys in `diff`, `target_ref`, or `proposed_payload` must prevent
  auto-apply.
- High-impact variants such as `memory_promotion`, `graph_fact`, and `correction`
  require explicit review.
- `tag` and `duplicate_merge` are the only low-risk derived organization variants.
- Low-risk derived organization may be marked auto-applyable only after approval, with
  confidence at or above the queue threshold, no unsupported-claim flags, no
  contradiction or staleness markers, and no source mutation request.
- Applying a queue item still means applying a derived target or marking review state;
  it must not update, delete, overwrite, or deprecate authoritative notes, docs,
  events, traces, graph facts, or source pointers.

## Future Connections

Future viewer work should render proposals as reviewable records with source refs,
snapshots, lineage, diff, confidence, contradiction markers, and staleness markers.

Future derived knowledge pages may use approved proposals as input, but those pages
remain rebuildable derived output. They must retain source pointers and must not become
a hidden replacement for evidence-bound ELF Core memory.
