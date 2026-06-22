---
type: Evidence
title: "Recall Debug Panel Report - June 20, 2026"
description: "Checked-in benchmark evidence record for the cross-layer recall/debug panel."
resource: docs/evidence/benchmarking/2026-06-20-recall-debug-panel-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-23
tags:
  - docs
  - evidence
  - benchmarking
  - recall
---
# Recall Debug Panel Report - June 20, 2026

Goal: Close XY-1022 by exposing one typed recall/debug read model across Memory
Notes, Source Library documents, Knowledge Workspace pages, graph facts, and Dreaming
review proposals.

Inputs: `packages/elf-service/src/recall_debug.rs`, `apps/elf-api/src/routes.rs`,
`apps/elf-mcp/src/server.rs`, `docs/spec/system_recall_debug_panel_v1.md`, and
`apps/elf-eval/fixtures/report_snapshots/2026-06-20-recall-debug-panel-report.json`.

## Executive Judgment

ELF now has `elf.recall_debug_panel/v1`, a read-only panel response that lets an
agent or operator inspect why recall candidates were selected, dropped, available, or
reviewable across the main Agent Knowledge OS layers. This is a product/debug surface
over existing authority layers, not a new mutating worker and not a replacement for
the underlying trace, docs, graph, knowledge, or proposal APIs.

The agent-facing endpoint is `POST /v2/recall-debug/panel`. The local admin endpoint
`POST /v2/admin/recall-debug/panel` remains an operator mirror over the same service
read model. Responses include `elf.recall_trace/v1`, a compact deterministic
projection for selected, dropped, stale, blocked, and not-requested context.

June 23 XY-1067 addendum: the Memory Notes layer also emits
`debug_artifacts.compact_replay` with schema
`elf.recall_debug.compact_replay/v1`. The artifact packages stored trace controls,
stage movement, replay candidates, rerank effects, selected final context, and
source/policy authority flags so an operator can reproduce a recall decision from
the panel without treating ELF as a separate local search sidecar.

## Layer Coverage

| Layer | Anchor | Selection states | Replay/readback |
| --- | --- | --- | --- |
| Memory Notes | `trace_id` | `selected`, `dropped` | `elf_admin_trace_bundle_get`; `debug_artifacts.compact_replay` |
| Source Library documents | `docs_query` or `query` | `selected` | `elf_docs_search_l0` |
| Knowledge Workspace pages | `knowledge_query` or `query` | `selected` | `elf_recall_debug_panel` with a page query |
| Graph facts | `graph_subject` | `available` | `elf_graph_report` |
| Dreaming proposals | `include_dreaming` | `reviewable` | `elf_dreaming_review_queue` |

Each row exposes item refs, authority layer, freshness state, source refs or source
snapshots, score/rank when available, stage reason, evidence class, replay command,
and layer-specific debug artifacts.

The embedded `elf.recall_trace/v1` projection flattens these rows into stable
layer/row order for fixture and report assertions. It carries `context_state`,
`selection_state`, freshness, source refs, score/rank, policy reason, replay command,
and evidence class without requiring raw database inspection.

The panel-level `limit` is a per-layer request cap, but the Source Library layer
inherits the docs-search effective cap of 32 rows and reports requested/effective
limits in document row debug artifacts.

The Memory Notes compact replay artifact exposes:

- Search controls from the stored trace, including expansion mode, allowed scopes,
  top-k, candidate count, and ranking config snapshot.
- Stage movement from persisted trajectory stages, including kept, dropped,
  selected, and blocked counts.
- Candidate replay rows for selected and dropped evidence, including retrieval rank,
  rerank rank, score delta, stage reason, policy reason, and source-ref pointers.
- Selected context rows that show the final narrated context order.
- Authority flags that keep source refs, policy reasons, and raw SQL out of the
  artifact boundary.

## Command Evidence

| Command | Status | Purpose |
| --- | --- | --- |
| `cargo test -p elf-service recall_trace --lib` | pass | Unit-check deterministic `recall_trace` stale, dropped, blocked, and not-requested projection. |
| `cargo test -p elf-service recall_debug -- --nocapture` | pass | Unit-check panel summary counters and `not_requested` layer behavior. |
| `cargo test -p elf-service compact_replay_artifact_exposes_controls_stage_movement_and_rerank_effects --lib -- --nocapture` | pass | Unit-check compact replay controls, stage movement, selected/dropped candidates, rerank delta, and raw-SQL boundary. |
| `cargo test -p elf-eval --test real_world_job_benchmark operator_debug_fixture_reports_trace_links_and_failure_details -- --nocapture` | pass | Fixture-check qmd-style compact replay benchmark coverage and qmd short-replay claim boundary. |
| `cargo test -p elf-mcp registers_all_tools -- --nocapture` | pass | Guard MCP tool registration for `elf_recall_debug_panel`. |
| `cargo test -p elf-eval --test real_world_job_benchmark recall_debug_panel_report_wires_cross_layer_debug_contract -- --nocapture` | pass | Guard service, API, MCP, docs, README, and snapshot coverage for XY-1022. |

## Claim Boundaries

Allowed:

- ELF exposes a typed cross-layer recall/debug read model.
- Memory trace selected rows and retained dropped replay candidates are visible
  through trace bundles when candidate capture/retention preserved them.
- Source documents, knowledge pages, graph facts, and Dreaming proposals can be
  inspected from one panel response when their anchors are supplied.
- The agent-facing panel response includes a deterministic `elf.recall_trace/v1`
  projection for selected, dropped, stale, blocked, and not-requested context.
- The Memory Notes layer exposes `elf.recall_debug.compact_replay/v1` for search
  controls, stage movement, candidate replay, rerank effects, dropped candidates,
  selected context, and source/policy authority flags.
- Missing anchors stay visible as `not_requested` layers instead of hidden pass
  claims.
- Requested layer readback failures stay visible as `blocked` layers instead of
  failing or hiding the rest of the panel.

Not allowed:

- Do not claim the panel mutates notes, docs, pages, graph facts, or proposals.
- Do not claim external competitor UI parity from this read model alone.
- Do not claim ELF broadly beats qmd's local replay ergonomics; qmd remains the
  short CLI replay reference where ELF has not matched that workflow.
- Do not treat missing anchors as pass evidence.
- Do not collapse blocked, incomplete, or wrong-result evidence into a broad win.

## Next Optimization Direction

The next useful layer is a visual operator panel that groups rows by layer,
authority, freshness, and stage reason, with one-click replay into trace bundles,
docs search, graph reports, and Dreaming queue filters. XY-1023 should then run the
full benchmark closeout and keep competitor debug advantages separate from ELF's
typed cross-layer readback.
