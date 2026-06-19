---
type: Evidence
title: "Agent Knowledge OS Closeout Benchmark Report - June 20, 2026"
description: "Checked-in closeout evidence matrix for the Agent Knowledge OS program."
resource: docs/evidence/benchmarking/2026-06-20-agent-knowledge-os-closeout-benchmark-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-20
tags:
  - docs
  - evidence
  - benchmarking
  - agent-knowledge-os
---
# Agent Knowledge OS Closeout Benchmark Report - June 20, 2026

Goal: Close XY-1023 by publishing a table-driven self-assessment after the staged
Agent Knowledge OS lanes landed.

Inputs: `apps/elf-eval/fixtures/report_snapshots/2026-06-20-agent-knowledge-os-closeout-benchmark-report.json`,
the June 20 component reports, the June 19 competitor retests/materialization
reports, and the current VectifyAI PageIndex/OpenKB GitHub readback.

## Command Evidence

| Command | Status | Result |
| --- | --- | --- |
| `cargo make real-world-memory` | pass | Reran the checked-in all-project fixture suite: 62 jobs, 17 encoded suites, 55 pass, 0 wrong_result, 0 incomplete, 7 blocked, 1.000 evidence/source-ref/quote coverage, mean score 0.887. |
| `cargo test -p elf-eval --test real_world_job_benchmark agent_knowledge_os_closeout_benchmark -- --nocapture` | pass | Guards the XY-1023 summary counts, key matrix boundaries, VectifyAI reference-only rows, claim boundaries, README/index links, and optimization queue. |

## Executive Judgment

ELF is the strongest measured integrated product in the current checked-in Agent
Knowledge OS matrix. It is the only product with same-repo evidence across all six
layers: Source Library, Memory Authority, Knowledge Workspace, graph-lite facts,
Dreaming review queue, and recall/debug panel.

That is not a broad "ELF beats everyone everywhere" claim. Not every product has
complete live coverage. qmd remains the retrieval/debug ergonomics reference;
OpenViking remains the context-trajectory reference; mem0/OpenMemory remains the
entity-history and ecosystem reference; Letta remains the core/archive memory
reference; Graphiti/Zep and graph/RAG projects remain graph-memory references; and
VectifyAI PageIndex/OpenKB are now explicit reference-only competitors for long
document tree retrieval and knowledge-base compilation.

## Coverage Summary

| Metric | Value |
| --- | --- |
| Products/projects in matrix | 19 |
| Agent Knowledge OS scenarios | 6 |
| Complete same-repo product coverage | ELF only |
| Matrix pass cells | 9 |
| Matrix wrong_result cells | 7 |
| Matrix incomplete cells | 6 |
| Matrix blocked cells | 14 |
| Matrix not_tested cells | 78 |

Evidence classes keep their normal meaning: `pass` means checked-in evidence
supports the claim; `wrong_result` means the adapter ran but missed required
evidence; `incomplete` means partial behavior or artifact coverage; `blocked` means
the required setup/artifact is missing; `not_tested` means reference-only or no
same-corpus benchmark coverage.

## Product Scenario Matrix

| Product/project | Coverage | Source Library | Memory Authority | Knowledge Workspace | Graph-lite/Temporal | Dreaming Review | Recall Debug | Current strongest advantage |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ELF | complete_same_repo | pass | pass | pass | pass | pass | pass | Policy-gated, source-linked, replayable authority across all six layers. |
| qmd | partial_same_corpus | not_tested | wrong_result | wrong_result | not_tested | wrong_result | wrong_result | Query expansion, weighted fusion, rerank, compact replay, and local debug knobs. |
| agentmemory | partial_same_corpus | incomplete | incomplete | not_tested | not_tested | blocked | not_tested | Cross-agent hooks, MCP/REST packaging, local viewer, continuity workflow. |
| OpenViking | partial_same_corpus | wrong_result | blocked | not_tested | not_tested | not_tested | not_tested | Filesystem-like context URIs, hierarchy, staged retrieval trajectory. |
| mem0/OpenMemory | partial_same_corpus | not_tested | pass | not_tested | blocked | not_tested | blocked | Entity-scoped memory history, hosted ecosystem, OpenMemory UI/export direction. |
| claude-mem | partial_reference | blocked | incomplete | not_tested | not_tested | blocked | not_tested | Progressive disclosure UX, local viewer, automatic capture-loop reference. |
| memsearch | partial_same_corpus | pass | pass | not_tested | not_tested | not_tested | not_tested | Markdown-first canonical store, incremental reindex, hybrid retrieval. |
| Letta | blocked_materialization | not_tested | blocked | not_tested | not_tested | blocked | not_tested | Core/archive memory model and export/readback product concept. |
| Graphiti/Zep | blocked_reference | not_tested | not_tested | not_tested | blocked | not_tested | not_tested | Temporal graph validity vocabulary and graph-memory product reference. |
| GraphRAG | partial_reference | not_tested | not_tested | incomplete | blocked | not_tested | not_tested | Graph-oriented retrieval and citation/navigation reference direction. |
| RAGFlow | blocked_reference | not_tested | not_tested | blocked | blocked | not_tested | not_tested | RAG workflow and document processing product reference. |
| LightRAG | incomplete_reference | not_tested | not_tested | incomplete | incomplete | not_tested | not_tested | Lightweight graph/RAG architecture reference. |
| graphify | scored_smoke | not_tested | not_tested | wrong_result | wrong_result | not_tested | not_tested | Tiny scored graph/RAG smoke target for artifact-shape mismatches. |
| llm-wiki | reference_only | not_tested | not_tested | not_tested | not_tested | not_tested | not_tested | Compiled wiki and knowledge-page workflow reference. |
| gbrain | reference_only | not_tested | not_tested | not_tested | not_tested | not_tested | not_tested | Personal knowledge-base and query-save/lint loop reference. |
| LangGraph | blocked_reference | not_tested | not_tested | not_tested | blocked | not_tested | not_tested | Agent graph orchestration reference for stateful workflows. |
| nanograph | blocked_reference | not_tested | not_tested | not_tested | blocked | not_tested | not_tested | Typed relation and small-graph memory reference. |
| VectifyAI PageIndex | reference_only | not_tested | not_tested | not_tested | not_tested | not_tested | not_tested | Vectorless long-document tree retrieval and PageIndex MCP ecosystem direction. |
| VectifyAI OpenKB | reference_only | not_tested | not_tested | not_tested | not_tested | not_tested | not_tested | Document-to-wiki compilation, concept/entity pages, lint, watch, and recompile workflow. |

## Competitor Strengths To Preserve

| Competitor | Strength | Evidence class | ELF response |
| --- | --- | --- | --- |
| qmd | Transparent local retrieval pipeline and compact replay ergonomics. | wrong_result | Add retrieval expansion, fusion, rerank, top-k, and compact replay controls on top of recall/debug. |
| VectifyAI PageIndex | Long-document tree search without a vector database and PageIndex MCP ecosystem. | not_tested | Add a benchmark-owned long-document tree adapter and compare it with ELF source refs and page rebuilds. |
| VectifyAI OpenKB | Compiled Markdown wiki, concept/entity pages, saved explorations, lint, watch, and recompile workflows. | not_tested | Fold OpenKB-style library management into Knowledge Workspace without weakening source-of-truth boundaries. |
| OpenViking | Staged context trajectory, hierarchy selection, and recursive expansion. | blocked | Emit comparable recall-planning stage artifacts, rejected siblings, and recursive expansion evidence. |
| mem0/OpenMemory | Entity-scoped memory history, hosted ecosystem, UI/export, and optional graph memory direction. | blocked | Strengthen history/event APIs, export UX, and optional graph-context channel while preserving policy-gated writes. |
| Letta | Core/archive memory split and memory export/readback product model. | blocked | Keep ELF core/archive source refs, but add contained adapter output before win/tie/loss claims. |
| Graphiti/Zep and graph/RAG projects | Temporal graph validity, citation/navigation, and graph retrieval references. | blocked | Expand graph-lite reports into adapter-backed temporal fact comparison without replacing Postgres authority. |
| agentmemory and claude-mem | Capture hooks, local viewers, and practical continuity UX. | incomplete | Build the operator viewer around Source Library, Memory Authority, Dreaming queue, and recall debug surfaces. |

## ELF Advantages

ELF's current durable advantage is composition: the layers are independently typed,
source-linked, and replayable, but they now fit together as an Agent Knowledge OS.
Source Library records preserve captured material, Memory Authority controls what
becomes memory, Knowledge Workspace pages stay derived and linted, graph-lite facts
stay source-backed, Dreaming proposals stay reviewable, and the recall/debug panel
shows selected, dropped, available, reviewable, not_requested, and blocked context.

This combination is stronger than any single measured competitor in the current
matrix. It is also more conservative: ELF refuses to call reference-only strengths a
pass, and it keeps private/provider production proof separate from simulated or
public-proxy evidence.

## Optimization Queue

| Priority | Queue item | Generated from benchmark delta | Next action |
| --- | --- | --- | --- |
| P0 | `pageindex_openkb_source_library_adapter` | PageIndex/OpenKB are reference-only but directly target long-document library management and knowledge compilation. | Create a contained adapter over benchmark-owned sources and compare tree/wiki artifacts against ELF source refs, knowledge pages, and recall debug rows. |
| P0 | `qmd_retrieval_knobs_and_short_replay` | qmd keeps the measured retrieval-debug ergonomics edge. | Expose retrieval expansion, fusion, rerank, top-k, and compact replay artifacts in ELF recall/debug surfaces. |
| P0 | `operator_knowledge_library_ui` | ELF has APIs but no unified library management surface. | Build a UI for saved articles/threads, source docs, derived pages, graph facts, proposal queue, and replayable recall traces. |
| P1 | `openviking_context_trajectory_artifacts` | OpenViking trajectory/hierarchy/recursive expansion remains blocked. | Emit same-corpus stage trajectory, hierarchy selection, rejected siblings, and recursive expansion artifacts. |
| P1 | `letta_core_archive_export_readback` | Letta core/archive comparison remains blocked. | Run contained Letta export/readback with core block JSON, archival search/readback JSON, and source ids. |
| P1 | `openmemory_ui_export_and_history_parity` | OpenMemory UI/export remains blocked and mem0 history remains a reference advantage. | Add product-container UI/export readback and strengthen ELF history/export APIs. |
| P1 | `graph_rag_temporal_adapter_matrix` | Graph/RAG projects remain typed non-pass or reference-only. | Produce contained same-corpus graph fact/page/citation artifacts while keeping ELF graph-lite source-backed. |
| P2 | `agentmemory_claude_mem_capture_viewer` | Capture/viewer UX remains incomplete or blocked. | Add a local operator viewer and capture audit flow across Source Library, Memory Authority, and recall debug traces. |
| P2 | `private_provider_production_refresh` | XY-930 proxy/public-corpus evidence cannot prove real private-corpus or provider-backed quality. | Run only when routed private corpus and provider setup exist. |

## Claim Boundaries

Allowed:

- ELF is the strongest measured integrated Agent Knowledge OS product in this
  checked-in matrix.
- ELF has complete same-repo evidence across the six Agent Knowledge OS layers.
- qmd, PageIndex/OpenKB, OpenViking, mem0/OpenMemory, Letta, graph/RAG systems, and
  capture/viewer projects still provide important optimization direction.

Not allowed:

- Do not claim ELF broadly beats every competitor on every competitor-owned strength.
- Do not treat `not_tested`, `blocked`, `incomplete`, or `wrong_result` as pass.
- Do not count VectifyAI PageIndex or OpenKB as benchmark wins until a same-corpus
  adapter emits checked-in artifacts.
- Do not claim private-corpus or hosted-provider production quality from public-proxy
  or local fixture evidence.
