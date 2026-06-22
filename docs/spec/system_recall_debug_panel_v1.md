---
type: Spec
title: "Recall Debug Panel v1 Specification"
description: "Define the cross-layer recall/debug panel readback contract for memory, source documents, knowledge pages, graph facts, and Dreaming proposals."
resource: docs/spec/system_recall_debug_panel_v1.md
status: active
authority: normative
owner: memory-service
last_verified: 2026-06-23
tags:
  - spec
  - recall
  - debug
  - mcp
source_refs: []
code_refs:
  - packages/elf-service/src/recall_debug.rs
  - apps/elf-api/src/routes.rs
  - apps/elf-mcp/src/server.rs
related:
  - docs/spec/system_elf_memory_service_v2.md
  - docs/spec/system_doc_extension_v1_trajectory.md
  - docs/spec/system_knowledge_pages_v1.md
  - docs/spec/system_graph_memory_postgres_v1.md
  - docs/spec/system_consolidation_proposals_v1.md
---
# Recall Debug Panel v1 Specification

Purpose: Define `elf.recall_debug_panel/v1`, the cross-layer readback surface for
replayable recall and operator debugging.
Status: normative
Read this when: You need to inspect why memory, source documents, knowledge pages,
graph facts, or Dreaming proposals were selected, dropped, or made available.
Not this document: Ranking math internals, document ingestion, graph mutation, page
rebuild, or proposal review state transitions.
Defines: Request anchors, layer rows, evidence classes, replay boundaries, and
authority/freshness fields for the recall/debug panel.

## Contract

The response schema is `elf.recall_debug_panel/v1`.

Agent-facing endpoint:

- `POST /v2/recall-debug/panel`

Operator mirror:

- `POST /v2/admin/recall-debug/panel`

Both routes use the same service read model. The admin route is a mirror for local
operator tooling; the public route is the agent-facing recall/debug API and remains
read-only.

The panel is a read model over existing authoritative surfaces:

- Memory Notes: search traces, trace items, trajectory stages, replay candidates, and
  note `source_ref` values.
- Source Library: `docs_search_l0` document chunk results and retrieval trajectory.
- Knowledge Workspace: admin project-level `knowledge_pages_search` derived page
  sections, source refs, lint summary, trust state, and rebuild metadata.
- Graph facts: `elf.graph_report/v1` facts and topic-map status markers.
- Dreaming proposals: `elf.dreaming_review_queue/v1` reviewable proposal rows.

The panel MUST NOT mutate notes, documents, pages, graph facts, or proposals.

The response includes:

- `summary`: aggregate layer counters.
- `recall_trace`: deterministic `elf.recall_trace/v1` projection for agent use,
  fixture assertions, and compact debug readback.
- `layers`: full layer rows and layer-level debug artifacts for detailed operator
  inspection.

## Request Anchors

`RecallDebugPanelRequest` requires tenant, project, agent, and read profile from the
authenticated request context. Client-provided anchors are optional:

- `trace_id`: loads memory selected and dropped rows from a persisted search trace.
- `query`: shared query fallback for document and knowledge-page layers.
- `docs_query`: Source Library query override.
- `knowledge_query`: Knowledge Workspace query override.
- `graph_subject`: graph entity selector by `entity_id` or `surface`.
- `graph_predicate`: optional graph predicate selector by `predicate_id` or `surface`.
- `include_dreaming`: includes Dreaming review queue proposals when true.
- `limit`: per-layer row cap, clamped to the implementation maximum.

If an anchor is absent, the corresponding layer MUST return `evidence_class =
"not_requested"` instead of pretending the layer was tested.
If an anchor is supplied but the backing typed readback fails, the corresponding
layer MUST return `evidence_class = "blocked"` instead of failing the whole panel.
The Source Library layer inherits the docs-search effective `top_k` cap of 32
rows even when the panel-level `limit` is higher; returned document rows expose
the requested and effective limits in `debug_artifacts`.

## Layers And Rows

Each returned layer MUST include:

- `layer`: one of `memory_notes`, `source_documents`, `knowledge_pages`,
  `graph_facts`, or `dreaming_proposals`.
- `evidence_class`, `summary`, `anchor`, row counters, `raw_sql_needed`, and
  `replayable`.
- `debug_artifacts`: compact layer-level replay or diagnosis payloads. Layers without
  layer-level artifacts return an empty object.

For the Memory Notes layer, `debug_artifacts.compact_replay` is an
`elf.recall_debug.compact_replay/v1` artifact derived from the persisted trace
metadata, trajectory stages, replay candidates, final selected rows, and source refs.
It MUST expose:

- `controls`: `top_k`, `candidate_count`, expansion mode, expanded queries, allowed
  scopes, search snapshot, ranking policy id, blend/diversity/retrieval-source
  decisions, and any stored ranking override.
- `stage_movement`: ordered stage names with item counts, stats, decisions, and
  filter impact when present.
- `candidate_replay`: selected and dropped candidate rows with retrieval rank,
  rerank rank, rerank delta, rerank score, retrieval score, selection state, stage
  reason, policy reason, source-ref availability, source ref, scope, and diversity
  decision fields.
- `selected_context`: final selected result handles, note/chunk ids, source-ref
  availability, source ref, freshness, final rank, final score, ranking policy id,
  compact ranking terms, policy reason, and relation-context count so
  answer-composition can diagnose selected-but-not-narrated context.
- `authority`: source/policy/raw-SQL flags showing that source refs and policy
  reasons remain visible and raw SQL is not required.

Each returned row MUST include:

- `layer`: one of `memory_notes`, `source_documents`, `knowledge_pages`,
  `graph_facts`, or `dreaming_proposals`.
- `item_ref`: stable identifiers for replay or hydration.
- `selection_state`: `selected`, `dropped`, `available`, or `reviewable`.
- `authority_layer`: the system surface that owns the row.
- `freshness_state`: lifecycle, temporal, lint, or review state.
- `source_refs`: source refs, evidence note ids, source snapshots, or coverage
  metadata that supports the row.
- `score` and `rank` when available.
- `rationale`: short reason for inclusion.
- `stage_reason`: stage name, diversity/drop reason, temporal marker, or review
  policy reason.
- `replay_command`: MCP tool command or deterministic artifact path when available.
- `evidence_class`: row-level evidence class.
- `debug_artifacts`: layer-specific explain payloads.

## Evidence Classes

Allowed layer evidence classes are:

- `pass`: the layer readback was executed through a typed ELF surface.
- `not_requested`: the client did not supply the anchor needed for that layer.
- `incomplete`: a requested layer lacks enough data to prove a recall/debug claim.
- `blocked`: an external dependency or authority boundary prevents execution.
- `wrong_result`: the layer executed but returned data that contradicts expected
  evidence.

The panel summary MUST preserve evidence class counts. Aggregate success MUST NOT
hide `not_requested`, `incomplete`, `blocked`, or `wrong_result` layers.

## Recall Trace

`recall_trace` is a compact, deterministic projection over the returned layers. It
MUST be stable in layer order and row order for the same persisted trace and backing
readback inputs. It MUST NOT include a generation timestamp.

Each trace entry MUST include:

- `layer`
- `context_state`: `selected`, `dropped`, `available`, `reviewable`, `stale`,
  `blocked`, `not_requested`, `incomplete`, or `wrong_result`.
- `selection_state`: the original row selection state or layer evidence class.
- `authority_layer`
- `freshness_state`
- `item_ref`
- `source_refs`
- `score` and `rank` when available.
- `policy_reason`: compact stage, drop, lint, temporal, review, blocked, or
  not-requested reason.
- `replay_command` when available.
- `evidence_class`
- `raw_sql_needed`

Rows with stale or non-current freshness such as `stale`, `deprecated`, `deleted`,
`superseded`, `tombstoned`, `historical`, `archived`, `lint_warning`, or `lint_error`
MUST appear in the trace with `context_state = "stale"` while preserving their
original `selection_state`.

Layers without rows but with `blocked`, `not_requested`, `incomplete`, or
`wrong_result` evidence MUST still contribute a trace entry carrying the layer summary
as `policy_reason`. This lets agents and reports distinguish absent anchors,
blocked readback, and actual empty pass results without raw database inspection.

## Replay Boundary

The panel may return replay commands such as `elf_admin_trace_bundle_get`,
`elf_docs_search_l0`, `elf_graph_report`, and `elf_dreaming_review_queue`. These
commands are readback aids only. They MUST NOT bypass write policy, proposal review,
graph mutation rules, or source-library authority.

## Raw SQL Boundary

The panel MUST prefer typed service, HTTP, and MCP surfaces. `raw_sql_needed` is false
for normal rows. If a future layer requires raw database inspection to explain a
claim, it must mark that layer or row explicitly instead of hiding the gap.
