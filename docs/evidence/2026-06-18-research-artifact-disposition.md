---
type: Evidence
title: "Research Artifact Disposition"
description: "Evidence record for promoting, carrying forward, or deleting legacy research JSON artifacts during the OKF and LLM Wiki migration."
resource: docs/evidence/2026-06-18-research-artifact-disposition.md
status: active
authority: current_state
owner: docs
last_verified: 2026-06-18
tags:
  - docs
  - evidence
  - research-promotion
  - okf
source_refs: []
code_refs:
  - docs/policy.md
  - apps/elf-eval/fixtures/external_memory_pattern_radar/cursor.json
related: []
drift_watch:
  - docs/research/
  - docs/evidence/external_memory/
  - docs/evidence/benchmarking/
---
# Research Artifact Disposition

Purpose: Record how legacy research JSON artifacts were handled while forming the
Markdown-only OKF and LLM Wiki bundle.
Read this when: You need to know whether an old research JSON was promoted, carried
forward, moved as tool state, or deleted.
Not this document: Raw research payload storage or a benchmark result.

## Disposition Rules

- Settled decisions move to `docs/decisions/`, `docs/spec/`, `docs/runbook/`, or
  `docs/evidence/`.
- Unresolved but valuable points move to new `docs/research/` contracts.
- Machine reports already represented by Markdown benchmark reports leave the
  research lane; test-required structured snapshots move to app-owned fixtures.
- Tool cursor state moves outside `docs/` and outside the research lane.

## Promoted Research Runs

| Retired artifact | Disposition | New owner |
| --- | --- | --- |
| `2026-06-08-agent-memory-selection` | Accepted decision promoted. | `docs/decisions/2026-06-08-agent-memory-selection.md` |
| `2026-06-09-xy-841-external-memory-benchmark-dimensions` | Benchmark-dimension conclusions promoted. | `docs/spec/real_world_agent_memory_benchmark_v1.md`; `docs/evidence/external_memory/comparison_external_projects.md`; `docs/evidence/external_memory/research_projects_inventory.md` |
| `2026-06-10-xy-882-rag-graph-adapter-feasibility` | Accepted verdicts promoted; unresolved follow-up preserved. | `docs/evidence/external_memory/research_projects_inventory.md`; `docs/research/graph_rag_adapter_followup.md`; `docs/research/derived_knowledge_page_followup.md` |

## Rehomed Machine Reports

The June 11 and June 16 JSON reports were removed from `docs/research/` because their
settled content is already owned by Markdown benchmark reports under
`docs/evidence/benchmarking/` and by the relevant specs or fixtures. Structured snapshots
that Rust boundary tests still parse now live under
`apps/elf-eval/fixtures/report_snapshots/`; they are app fixtures, not documentation
owners or research contracts.

Representative owners:

- `docs/evidence/benchmarking/2026-06-11-competitor-strength-adoption-report.md`
- `docs/evidence/benchmarking/2026-06-11-competitor-strength-evidence-matrix.md`
- `docs/evidence/benchmarking/2026-06-11-graph-rag-scored-smoke-adapter-report.md`
- `docs/evidence/benchmarking/2026-06-16-dreaming-readiness-stage-ledger.md`
- `docs/evidence/benchmarking/2026-06-16-proactive-brief-scoring-report.md`
- `docs/evidence/benchmarking/2026-06-16-scheduled-memory-task-scoring-report.md`

## Carried Forward Research

Unresolved value points now live as explicit research contracts:

- `docs/research/graph_rag_adapter_followup.md`
- `docs/research/derived_knowledge_page_followup.md`
- `docs/research/dreaming_product_surface_followup.md`

## Tool State

The external memory pattern radar cursor is active tool state, not a research
conclusion. It now lives at
`apps/elf-eval/fixtures/external_memory_pattern_radar/cursor.json`.

## Verdict

pass

## Citations

- `docs/policy.md`
- `docs/decisions/2026-06-08-agent-memory-selection.md`
- `docs/research/graph_rag_adapter_followup.md`
- `docs/research/derived_knowledge_page_followup.md`
- `docs/research/dreaming_product_surface_followup.md`
