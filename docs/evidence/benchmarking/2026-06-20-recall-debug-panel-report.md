---
type: Evidence
title: "Recall Debug Panel Report - June 20, 2026"
description: "Checked-in benchmark evidence record for the cross-layer recall/debug panel."
resource: docs/evidence/benchmarking/2026-06-20-recall-debug-panel-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-20
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

## Layer Coverage

| Layer | Anchor | Selection states | Replay/readback |
| --- | --- | --- | --- |
| Memory Notes | `trace_id` | `selected`, `dropped` | `elf_admin_trace_bundle_get` |
| Source Library documents | `docs_query` or `query` | `selected` | `elf_docs_search_l0` |
| Knowledge Workspace pages | `knowledge_query` or `query` | `selected` | `elf_recall_debug_panel` with a page query |
| Graph facts | `graph_subject` | `available` | `elf_graph_report` |
| Dreaming proposals | `include_dreaming` | `reviewable` | `elf_dreaming_review_queue` |

Each row exposes item refs, authority layer, freshness state, source refs or source
snapshots, score/rank when available, stage reason, evidence class, replay command,
and layer-specific debug artifacts.

The panel-level `limit` is a per-layer request cap, but the Source Library layer
inherits the docs-search effective cap of 32 rows and reports requested/effective
limits in document row debug artifacts.

## Command Evidence

| Command | Status | Purpose |
| --- | --- | --- |
| `cargo test -p elf-service recall_debug -- --nocapture` | pass | Unit-check panel summary counters and `not_requested` layer behavior. |
| `cargo test -p elf-mcp registers_all_tools -- --nocapture` | pass | Guard MCP tool registration for `elf_recall_debug_panel`. |
| `cargo test -p elf-eval --test real_world_job_benchmark recall_debug_panel_report_wires_cross_layer_debug_contract -- --nocapture` | pass | Guard service, API, MCP, docs, README, and snapshot coverage for XY-1022. |

## Claim Boundaries

Allowed:

- ELF exposes a typed cross-layer recall/debug read model.
- Memory trace selected rows and retained dropped replay candidates are visible
  through trace bundles when candidate capture/retention preserved them.
- Source documents, knowledge pages, graph facts, and Dreaming proposals can be
  inspected from one panel response when their anchors are supplied.
- Missing anchors stay visible as `not_requested` layers instead of hidden pass
  claims.
- Requested layer readback failures stay visible as `blocked` layers instead of
  failing or hiding the rest of the panel.

Not allowed:

- Do not claim the panel mutates notes, docs, pages, graph facts, or proposals.
- Do not claim external competitor UI parity from this read model alone.
- Do not treat missing anchors as pass evidence.
- Do not collapse blocked, incomplete, or wrong-result evidence into a broad win.

## Next Optimization Direction

The next useful layer is a visual operator panel that groups rows by layer,
authority, freshness, and stage reason, with one-click replay into trace bundles,
docs search, graph reports, and Dreaming queue filters. XY-1023 should then run the
full benchmark closeout and keep competitor debug advantages separate from ELF's
typed cross-layer readback.
