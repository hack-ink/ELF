---
type: Evidence
title: "P3 Competitor-Strength Absorption Report - June 23, 2026"
description: "P3 closeout report for competitor-strength absorption, remaining external strengths, typed blockers, and the P4 optimization queue."
resource: docs/evidence/benchmarking/2026-06-23-p3-competitor-strength-absorption-report.md
status: active
authority: evidence
owner: benchmarking
last_verified: 2026-06-23
tags:
  - docs
  - evidence
  - benchmarking
  - p3-closeout
source_refs:
  - apps/elf-eval/fixtures/report_snapshots/2026-06-23-p3-competitor-strength-absorption-report.json
code_refs:
  - Makefile.toml
  - apps/elf-eval/fixtures/real_world_external_adapters/pageindex_openkb/
  - apps/elf-eval/fixtures/real_world_external_adapters/mem0_openmemory_letta/
  - apps/elf-eval/fixtures/real_world_external_adapters/graph_rag/
  - apps/elf-eval/fixtures/real_world_memory/context_trajectory/
related:
  - docs/spec/agent_memory_knowledge_system_v1.md
  - docs/evidence/benchmarking/2026-06-19-qmd-debug-ergonomics-dreaming-retest-report.md
  - docs/evidence/benchmarking/2026-06-22-pageindex-openkb-same-corpus-adapter-report.md
  - docs/evidence/benchmarking/2026-06-22-mem0-openmemory-letta-memory-history-core-archive-report.md
  - docs/evidence/benchmarking/2026-06-23-temporal-trajectory-adapter-coverage-report.md
  - docs/evidence/benchmarking/2026-06-23-graph-rag-adapter-matrix-report.md
drift_watch:
  - docs/evidence/benchmarking/2026-06-23-p3-competitor-strength-absorption-report.md
  - apps/elf-eval/fixtures/report_snapshots/2026-06-23-p3-competitor-strength-absorption-report.json
  - docs/evidence/benchmarking/index.md
  - README.md
---
# P3 Competitor-Strength Absorption Report - June 23, 2026

Purpose: Close XY-1072 by publishing which competitor strengths ELF absorbed, which
remain stronger elsewhere, and which adapters are still blocked before P4 quality
hardening.
Status: evidence
Read this when: You need the P3 closeout answer for qmd, PageIndex/OpenKB,
mem0/OpenMemory, Letta, Graphiti/Zep, OpenViking, RAGFlow, GraphRAG, and LightRAG.
Not this document: A P4 queue action, hosted/private-corpus proof, or broad
ELF-over-every-competitor claim.
Inputs: The June 19 qmd report, June 22 PageIndex/OpenKB and
mem0/OpenMemory/Letta reports, June 23 temporal/trajectory report, June 23 graph/RAG
adapter matrix, and their focused rerun commands.

## Executive Judgment

P3 is decision-ready for main-thread inspection. It is not a P4 queue action.

ELF is strongest at governed, source-linked, reviewable memory and knowledge
authority: Source Library records, Memory Authority correction/history, Knowledge
Workspace pages, graph-lite reports, Dreaming review queue, and recall/debug readback
are all typed, source-linked, and bounded. P3 absorbed competitor strengths by turning
them into ELF-owned evidence surfaces, same-corpus adapter blockers, and concrete P4
optimization inputs.

The competitor picture is still mixed. qmd keeps the default top-k JSON and short
local replay edge. PageIndex/OpenKB, OpenMemory UI/export, Letta core/archive,
Graphiti/Zep temporal validity, OpenViking trajectory, and graph/RAG citation or
navigation strengths remain blocked, incomplete, or not encoded until comparable
same-corpus artifacts exist. Typed non-pass states are not wins.

No P4 issue receives `decodex:queued:elf` from this closeout. The queue below is
ready for main-thread inspection only after this report and validation evidence are
accepted.

## Rerun Evidence

| Command | Result | Evidence |
| --- | --- | --- |
| `cargo make real-world-memory-pageindex-openkb` | `pass` | PageIndex/OpenKB slice remains 2 jobs, 0 pass, 0 wrong_result, 0 incomplete, and 2 blocked. |
| `cargo make real-world-memory-mem0-openmemory-letta` | `pass` | mem0/OpenMemory/Letta slice remains 4 jobs, 1 pass, 0 wrong_result, 0 incomplete, and 3 blocked. |
| `cargo make real-world-memory-context-trajectory` | `pass` | OpenViking context-trajectory slice remains 3 jobs, 0 pass, 0 wrong_result, 0 incomplete, and 3 blocked. |
| `cargo make real-world-memory-graph-rag` | `pass` | Representative graph/RAG slice remains 5 jobs, 0 pass, 1 wrong_result, 1 incomplete, and 3 blocked; the adapter matrix records 0 pass rows. |

Checked-in closeout snapshot:

- `apps/elf-eval/fixtures/report_snapshots/2026-06-23-p3-competitor-strength-absorption-report.json`

## Product Strengths And ELF Response

| Product/reference | What ELF absorbed | What remains stronger elsewhere or blocked | P4 optimization input |
| --- | --- | --- | --- |
| qmd | ELF recall/debug exposes trace hydration, replay commands, candidate-drop visibility, and selected-but-not-narrated evidence in the operator-debug slice. | qmd still has the default top-k JSON artifact and short local CLI replay edge; expansion, dense/sparse, fusion, and rerank attribution parity is not proven. | `qmd_candidate_replay_parity` |
| VectifyAI PageIndex | ELF Source Library has long-document source records, hydrated excerpts, source refs, and explicit same-corpus PageIndex blocker requirements. | PageIndex remains the vectorless long-document tree retrieval and PageIndex MCP reference until tree artifacts, cited node paths, traversal output, and MCP readback map to ELF source ids. | `source_library_tree_and_wiki_adapters` |
| VectifyAI OpenKB | ELF Knowledge Workspace has source-linked project/entity/concept/issue pages, stale lint, watch/rebuild, and version-diff readback. | OpenKB remains the compiled wiki, saved exploration, concept/entity index, lint, watch, and recompile workflow reference until contained exports map to ELF source ids. | `source_library_tree_and_wiki_adapters` |
| mem0/OpenMemory | The P3 slice maps mem0 SDK `Memory.history`, scoped search, and local `get_all` export-style output to source ids while keeping OpenMemory product evidence separate. | mem0 remains stronger on explicit local SDK ADD, UPDATE, DELETE history readback; OpenMemory UI/export remains blocked until product-container and app-database exports map same-corpus rows. | `memory_history_export_and_core_archive` |
| Letta | The P3 slice names ELF core-block and archival source ids that a contained Letta export/readback must map before scoring. | Letta remains the core/archive memory model and export/readback reference until exported core block JSON, archival passage/readback/search JSON, visibility/provenance metadata, and source ids exist. | `memory_history_export_and_core_archive` |
| Graphiti/Zep | The fixture now names current facts, historical facts, provider-boundary evidence, and the blocked trace stage. | Graphiti/Zep remains the temporal graph validity reference; hosted Zep and provider-backed graph quality are not proven locally. | `temporal_trajectory_graph_rag_adapters` |
| OpenViking | Context-trajectory fixtures expose same-corpus, hierarchy, recursive-expansion, rejected-sibling, decoy, and comparison gates as typed blockers. | OpenViking remains the filesystem-like URI, hierarchy selection, staged retrieval trajectory, and recursive expansion reference until comparable staged artifacts exist. | `temporal_trajectory_graph_rag_adapters` |
| RAGFlow | The adapter matrix turns retrieval, citation, navigation, stale-source, faithfulness, and knowledge-compilation expectations into explicit rows. | RAGFlow remains blocked or not encoded until answers and selected reference chunks map document ids, chunk ids, content, metadata, and stale-source outputs to evidence ids. | `temporal_trajectory_graph_rag_adapters` |
| GraphRAG | The adapter matrix names output-table, citation, graph/community navigation, faithfulness, and stale-source requirements without claiming parity. | GraphRAG remains blocked or not encoded until documents, text units, communities, reports, entities, relationships, local-search answers, and unsupported/stale claim lint map to evidence ids. | `temporal_trajectory_graph_rag_adapters` |
| LightRAG | The adapter matrix records context/source reference, retrieval, navigation, faithfulness, stale-source, and knowledge-compilation coverage gaps. | LightRAG remains incomplete or not encoded until Docker API output exposes context, file paths, snippets, source references, and answer checking mapped to evidence ids. | `temporal_trajectory_graph_rag_adapters` |

## What ELF Is Strongest At

ELF's durable strength is not a single retrieval trick. It is governed memory change
control backed by source evidence:

- Source material remains source material until an explicit reviewable memory path
  promotes it.
- Memory changes have policy decisions, history, correction, rollback, and recall
  debug readback.
- Knowledge pages are derived, cited, linted, rebuildable, and version-diffed.
- Graph-lite and Dreaming outputs stay source-backed and reviewable.
- Recall/debug surfaces show selected, dropped, stale, blocked, not-requested, and
  reviewable context instead of hiding missing evidence behind a broad score.

That makes ELF strongest as an integrated agent memory and knowledge authority
system. It does not make ELF stronger than each competitor on that competitor's own
specialty.

## P4 Optimization Queue

The P4 queue is ready for main-thread inspection after this closeout passes
self-assessment. No queue label is applied here.

| Priority | Queue item | Scope |
| --- | --- | --- |
| P0 | `qmd_candidate_replay_parity` | Emit comparable immediate candidate replay artifacts with expansion, dense/sparse, fusion, rerank, dropped evidence, and one-command replay lines. |
| P0 | `adapter_outcome_grammar_and_metrics` | Harden public comparison grammar, typed outcomes, expected evidence recall, irrelevant context ratio, unsupported-claim counts, and resource metrics. |
| P1 | `source_library_tree_and_wiki_adapters` | Materialize PageIndex tree artifacts and OpenKB wiki/index/lint/watch outputs over the same corpus. |
| P1 | `memory_history_export_and_core_archive` | Harden mem0/OpenMemory history/export comparison and Letta core/archive export/readback mapping. |
| P1 | `temporal_trajectory_graph_rag_adapters` | Materialize Graphiti/Zep temporal validity, OpenViking trajectory, and RAGFlow/GraphRAG/LightRAG citation/navigation artifacts. |

## Claim Boundaries

Allowed:

- ELF is strongest at governed source-linked memory and knowledge authority in the
  checked-in evidence.
- P3 absorbed competitor strengths into ELF-owned evidence surfaces, same-corpus
  blockers, and P4 optimization inputs.
- The P4 optimization queue is ready for main-thread inspection after this closeout
  passes self-assessment.

Not allowed:

- Typed non-pass states are not wins.
- Do not claim ELF broadly beats qmd, PageIndex, OpenKB, mem0/OpenMemory, Letta,
  Graphiti/Zep, OpenViking, RAGFlow, GraphRAG, or LightRAG.
- Do not claim private-corpus, hosted, provider-backed, UI/export, graph/RAG, or
  core/archive parity from fixture-only, blocked, incomplete, wrong-result, or
  not-encoded evidence.
- Do not apply `decodex:queued:elf` to a P4 issue until the main thread accepts the
  P3 closeout.
