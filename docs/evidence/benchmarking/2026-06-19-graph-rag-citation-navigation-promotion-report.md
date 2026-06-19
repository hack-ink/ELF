---
type: Evidence
title: "Graph/RAG Citation and Navigation Promotion Report - June 19, 2026"
description: "Checked-in benchmark evidence record: Graph/RAG Citation and Navigation Promotion Report - June 19, 2026."
resource: docs/evidence/benchmarking/2026-06-19-graph-rag-citation-navigation-promotion-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-19
tags:
  - docs
  - evidence
  - benchmarking
---
# Graph/RAG Citation and Navigation Promotion Report - June 19, 2026

Goal: Promote graph/RAG citation, navigation, stale-source lint, and knowledge
surface cases only when adapters emit benchmark-comparable evidence-linked outputs.
Read this when: You need to know whether XY-985 changed the graph/RAG comparison
status after XY-955, which adapter paths are evidence-linked, and which outcomes must
remain blocked, incomplete, wrong_result, not_encoded, or non_goal.
Inputs:
`apps/elf-eval/fixtures/report_snapshots/2026-06-19-graph-rag-citation-navigation-promotion-report.json`,
`tmp/real-world-memory/graph-rag/report.json`,
`tmp/real-world-memory/graph-rag/report.md`,
`apps/elf-eval/fixtures/real_world_external_adapters/graph_rag/`,
and `docs/evidence/benchmarking/2026-06-17-dreaming-competitor-strength-retest-report.md`.
Outputs: A fresh graph/RAG command run, a JSON companion, typed scenario outcomes,
artifact paths, and an unchanged/improved/regressed judgment.

## Executive Judgment

The graph/RAG comparison status is unchanged: typed non-pass, no parity claim.

`cargo make real-world-memory-graph-rag` is reproducible and publishes a fresh
representative graph/RAG report:

- 5 jobs.
- 0 pass.
- 1 wrong_result.
- 1 incomplete.
- 3 blocked.
- Evidence/source-ref/quote coverage: 3/12, or 0.250.
- Knowledge citation coverage: 0.667.
- Stale claim detection: 0.000.

The useful promotion is auditability: the June 19 report keeps the graphify
graph/report path as evidence-linked output while preserving the non-pass result.
The competitive result did not improve. RAGFlow, GraphRAG, and Graphiti/Zep remain
blocked; LightRAG remains incomplete; graphify remains wrong_result; llm-wiki remains
not encoded; gbrain remains blocked.

## Command Evidence

| Command | Result | Artifact |
| --- | --- | --- |
| `cargo make real-world-memory-graph-rag` | command pass; representative suite typed non-pass | `tmp/real-world-memory/graph-rag/report.json`, `tmp/real-world-memory/graph-rag/report.md` |

## Scenario Outcomes

| Project | Scenario | Status | Artifact | Boundary |
| --- | --- | --- | --- | --- |
| RAGFlow | Reference-chunk citation mapping | `blocked` | `apps/elf-eval/fixtures/real_world_external_adapters/graph_rag/ragflow_reference_chunks_blocked.json` | Returned chunks must map generated document ids, chunk ids, content, and metadata before scoring. |
| LightRAG | Context/source reference mapping | `incomplete` | `apps/elf-eval/fixtures/real_world_external_adapters/graph_rag/lightrag_context_sources_incomplete.json` | Source paths, snippets, or references are required before comparison. |
| GraphRAG | Output-table citation mapping | `blocked` | `apps/elf-eval/fixtures/real_world_external_adapters/graph_rag/graphrag_output_tables_blocked.json` | Documents, text units, communities, reports, entities, and relationships must map to generated evidence ids. |
| Graphiti/Zep | Temporal graph validity mapping | `blocked` | `apps/elf-eval/fixtures/real_world_external_adapters/graph_rag/graphiti_temporal_validity_blocked.json` | Current and historical graph facts must carry validity windows and evidence ids. |
| graphify | Graph report navigation and lint | `wrong_result` | `apps/elf-eval/fixtures/real_world_external_adapters/graph_rag/graphify_graph_report_wrong_result.json` | Evidence-linked graph/report output exists, but stale-source detection is missing and unsupported summary remains present. |
| llm-wiki | Wiki page citation lint | `not_encoded` | none | No contained page materializer exists. |
| gbrain | Compiled-truth/timeline export | `blocked` | none | Docker-local brain repository and database setup remain unproven. |

## Improvement/Regression Readback

- Improved: a fresh June 19 checked-in companion records the command run and scenario
  boundaries after XY-955.
- Unchanged: no graph/RAG scenario moved to pass.
- Unchanged: graphify produces evidence-linked output but still scores wrong_result.
- Unchanged: setup/provider/output contracts still block RAGFlow, GraphRAG,
  Graphiti/Zep, LightRAG, llm-wiki, and gbrain.
- No regression: no previously passing graph/RAG comparison became non-pass.

## Claim Boundaries

Allowed:

- The representative graph/RAG command is reproducible.
- graphify emits evidence-linked graph/report output but remains wrong_result.
- The graph/RAG comparison status is unchanged relative to XY-955.

Not allowed:

- Do not claim graph/RAG parity or broad graph-navigation quality.
- Do not convert research gates, tiny smokes, blocked setup, incomplete output, or
  graphify wrong_result into an ELF win.
- Do not use private providers, hosted services, or unrecorded credentials for this
  lane.

## Next Optimization Direction

The next benchmark step is not an ELF graph/RAG product rewrite. It is adapter output
materialization:

- RAGFlow reference chunk ids and document metadata,
- LightRAG context source paths or snippets,
- GraphRAG output table rows with generated evidence ids,
- Graphiti/Zep `valid_at`/`invalid_at` evidence mapping,
- graphify stale-source lint pass,
- llm-wiki contained page materializer,
- gbrain Docker-local brain repo export.
