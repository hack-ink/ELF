---
type: Evidence
title: "Graph/RAG Adapter Matrix Report - June 23, 2026"
description: "Checked-in benchmark evidence record for the RAGFlow, GraphRAG, and LightRAG citation/navigation adapter matrix."
resource: docs/evidence/benchmarking/2026-06-23-graph-rag-adapter-matrix-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-23
tags:
  - docs
  - evidence
  - benchmarking
---
# Graph/RAG Adapter Matrix Report - June 23, 2026

Goal: Add the XY-1071 adapter matrix for RAGFlow, GraphRAG, and LightRAG while
preserving graph/RAG typed blockers and avoiding any generic RAG-platform claim for
ELF.

Read this when: You need the current RAGFlow, GraphRAG, and LightRAG coverage rows
for retrieval quality, citation quality, graph or document navigation, stale-source
behavior, answer faithfulness, and knowledge compilation quality.

Inputs:
`apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`,
`apps/elf-eval/fixtures/real_world_external_adapters/graph_rag/`, and
`apps/elf-eval/fixtures/report_snapshots/2026-06-23-graph-rag-adapter-matrix-report.json`.

Outputs: Manifest-backed scenario rows, a checked-in JSON companion, and bounded
claims for ELF Knowledge Workspace and Recall Debug learnings.

## Executive Judgment

The graph/RAG comparison remains typed non-pass. The matrix adds coverage clarity,
not quality wins.

- Matrix rows: 18.
- Pass rows: 0.
- Blocked rows: 8.
- Incomplete rows: 4.
- Not-encoded rows: 6.
- Adapters covered: RAGFlow, LightRAG, and GraphRAG.
- Dimensions covered for each adapter: retrieval quality, citation quality,
  navigation quality, stale-source behavior, answer faithfulness, and knowledge
  compilation quality.

No graph/RAG parity claim is made. No RAGFlow, GraphRAG, or LightRAG retrieval,
citation, navigation, stale-source, faithfulness, or knowledge-compilation pass is
claimed until scored same-corpus outputs exist.

## Adapter Matrix

| Adapter | Retrieval Quality | Citation Quality | Navigation Quality | Stale-Source Behavior | Answer Faithfulness | Knowledge Compilation |
| --- | --- | --- | --- | --- | --- | --- |
| RAGFlow | `blocked`: answer text plus selected reference chunks must map to evidence ids. | `blocked`: returned chunks need document ids, chunk ids, content, and metadata. | `blocked`: document/chunk handles must be followable to source evidence. | `not_encoded`: no stale-source replacement or lint artifact. | `blocked`: answers must be checked against cited chunks and decoys. | `not_encoded`: no page, section, citation, or lint artifact. |
| LightRAG | `incomplete`: Docker API context export is not available by default. | `incomplete`: context references or file paths must map to evidence ids. | `incomplete`: graph/context source paths or snippets must be exported. | `not_encoded`: no stale-source replacement or lint artifact. | `incomplete`: only_need_context output must support answer checking. | `not_encoded`: no page, section, citation, or lint artifact. |
| GraphRAG | `not_encoded`: local-search retrieval quality is not scored. | `blocked`: output tables must map documents, text units, communities, reports, entities, and relationships to evidence ids. | `blocked`: community/entity/relationship navigation requires mapped output tables. | `not_encoded`: no stale-source replacement or lint artifact. | `blocked`: summaries or local-search answers must be checked against mapped tables. | `not_encoded`: graph-summary synthesis quality remains not tested. |

## Checked-In Evidence

| Artifact | Role |
| --- | --- |
| `apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json` | Manifest-backed adapter scenario matrix. |
| `apps/elf-eval/fixtures/real_world_external_adapters/graph_rag/ragflow_reference_chunks_blocked.json` | RAGFlow same-corpus reference-chunk typed blocker. |
| `apps/elf-eval/fixtures/real_world_external_adapters/graph_rag/lightrag_context_sources_incomplete.json` | LightRAG same-corpus context/source typed incomplete state. |
| `apps/elf-eval/fixtures/real_world_external_adapters/graph_rag/graphrag_output_tables_blocked.json` | GraphRAG same-corpus output-table typed blocker. |
| `apps/elf-eval/fixtures/report_snapshots/2026-06-23-graph-rag-adapter-matrix-report.json` | XY-1071 durable matrix snapshot. |

## ELF Feedback

- Knowledge Workspace should keep citation coverage separate from knowledge
  compilation quality. Source refs alone do not prove answer faithfulness.
- Recall Debug should surface missing adapter source handles as typed blocker evidence
  rather than hiding them behind aggregate retrieval scores.
- Stale-source behavior needs explicit changed-source or validity-window artifacts
  before any graph/RAG comparison can move beyond `not_encoded` or `blocked`.

## Claim Boundaries

Allowed:

- The RAGFlow, GraphRAG, and LightRAG matrix rows are checked in.
- The current rows preserve typed blockers, incomplete setup states, and not-encoded
  quality dimensions.
- The representative graph/RAG command remains the focused rerun path.

Not allowed:

- Do not claim graph/RAG parity or broad graph-navigation quality.
- Do not claim RAGFlow, GraphRAG, or LightRAG pass retrieval, citation, navigation,
  stale-source, faithfulness, or knowledge-compilation quality until scored artifacts
  exist.
- Do not reposition ELF as a generic RAG platform from this adapter matrix.
