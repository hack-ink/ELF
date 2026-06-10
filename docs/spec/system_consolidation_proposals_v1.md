# Consolidation Proposals v1 Specification

Purpose: Define the reviewable consolidation run and proposal contract for derived memory output.
Status: normative
Read this when: You are implementing, validating, or reviewing dreaming-inspired consolidation storage, jobs, proposals, or review flows.
Not this document: Live LLM consolidation generation, viewer UI behavior, retrieval observability panels, or agentmemory import adapters.
Defines: `elf.consolidation/v1` runs, proposals, source snapshots, lineage, review lifecycle, and source immutability rules.

Related inputs:

- `docs/research/2026-06-08-agent-memory-selection.json`
- `docs/guide/research/comparison_external_projects.md`
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

- create a consolidation run with optional proposal payloads
- list consolidation runs
- get a consolidation run
- list consolidation proposals
- get a consolidation proposal
- transition proposal review state through `approve`, `apply`, `discard`, and `defer`
  actions with review-event readback

These flows must not call LLM, embedding, rerank, or external provider adapters.

## Future Connections

Future viewer work should render proposals as reviewable records with source refs,
snapshots, lineage, diff, confidence, contradiction markers, and staleness markers.

Future derived knowledge pages may use approved proposals as input, but those pages
remain rebuildable derived output. They must retain source pointers and must not become
a hidden replacement for evidence-bound ELF Core memory.
