---
type: Spec
title: "Recall Debug Panel v1 Specification"
description: "Define the cross-layer recall/debug panel readback contract for memory, source documents, knowledge pages, graph facts, and Dreaming proposals."
resource: docs/spec/system_recall_debug_panel_v1.md
status: active
authority: normative
owner: memory-service
last_verified: 2026-06-20
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

The panel is a read model over existing authoritative surfaces:

- Memory Notes: search traces, trace items, trajectory stages, replay candidates, and
  note `source_ref` values.
- Source Library: `docs_search_l0` document chunk results and retrieval trajectory.
- Knowledge Workspace: admin project-level `knowledge_pages_search` derived page
  sections, source refs, lint summary, trust state, and rebuild metadata.
- Graph facts: `elf.graph_report/v1` facts and topic-map status markers.
- Dreaming proposals: `elf.dreaming_review_queue/v1` reviewable proposal rows.

The panel MUST NOT mutate notes, documents, pages, graph facts, or proposals.

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

## Layer Rows

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

## Replay Boundary

The panel may return replay commands such as `elf_admin_trace_bundle_get`,
`elf_docs_search_l0`, `elf_graph_report`, and `elf_dreaming_review_queue`. These
commands are readback aids only. They MUST NOT bypass write policy, proposal review,
graph mutation rules, or source-library authority.

## Raw SQL Boundary

The panel MUST prefer typed service, HTTP, and MCP surfaces. `raw_sql_needed` is false
for normal rows. If a future layer requires raw database inspection to explain a
claim, it must mark that layer or row explicitly instead of hiding the gap.
