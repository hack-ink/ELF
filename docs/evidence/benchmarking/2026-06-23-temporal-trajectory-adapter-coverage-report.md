---
type: Evidence
title: "Temporal and Trajectory Adapter Coverage Report - June 23, 2026"
description: "Checked-in benchmark evidence record for XY-1070 Graphiti/Zep temporal and OpenViking context-trajectory adapter coverage."
resource: docs/evidence/benchmarking/2026-06-23-temporal-trajectory-adapter-coverage-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-23
tags:
  - docs
  - evidence
  - benchmarking
source_refs: []
code_refs:
  - apps/elf-eval/fixtures/real_world_external_adapters/graph_rag/graphiti_temporal_validity_blocked.json
  - apps/elf-eval/fixtures/real_world_memory/context_trajectory/openviking_staged_retrieval_blocked.json
  - apps/elf-eval/fixtures/real_world_memory/context_trajectory/openviking_hierarchy_selection_blocked.json
  - apps/elf-eval/fixtures/real_world_memory/context_trajectory/openviking_recursive_expansion_blocked.json
  - scripts/graphiti-zep-docker-temporal-smoke.py
drift_watch:
  - docs/spec/real_world_agent_memory_benchmark_v1.md
  - docs/runbook/benchmarking/real_world_agent_memory_benchmark.md
---
# Temporal and Trajectory Adapter Coverage Report - June 23, 2026

Purpose: Record the XY-1070 refresh for Graphiti/Zep temporal-validity and
OpenViking context-trajectory adapter coverage.
Read this when: You need to know whether temporal graph validity or staged context
trajectory evidence changed after the P2 Knowledge Workspace closeout.
Not this document: Broad graph-memory, hosted Zep, or OpenViking parity evidence.
Inputs: Graph/RAG representative fixtures, OpenViking context-trajectory fixtures,
and the Graphiti/Zep Docker temporal smoke materializer.
Outputs: Typed blocker artifacts with current/historical temporal source ids,
stage-level trajectory blockers, and updated benchmark requirements.

## Judgment

XY-1070 improves adapter auditability, not competitive status.

- Graphiti/Zep temporal-validity coverage now includes a checked-in adapter response
  for the representative blocked fixture. It names current, historical, and provider
  boundary evidence ids, and exposes `graphiti.provider_boundary` as the blocked trace
  stage.
- The generated Graphiti/Zep smoke manifest now emits a
  `temporal_validity_window_mapping` scenario row. A live pass still requires
  provider-backed Graphiti search output that maps current and historical facts to
  validity windows and source evidence ids.
- OpenViking staged retrieval, hierarchy selection, and recursive/context expansion
  remain typed blockers. Each fixture now exposes trace stages for the same-corpus
  gate and the missing stage, hierarchy, rejected sibling/decoy, or recursive
  expansion artifact.
- No ELF graph-memory, Graphiti/Zep, hosted Zep, or OpenViking parity, win, tie, or
  loss claim is created by this refresh.

ELF graph-lite remains a derived projection over authoritative source evidence. These
adapter artifacts refine the benchmark and recall-planning trace requirements; they do
not replace memory notes, source refs, or Postgres source-of-truth authority.

## Command Evidence

| Command | Result | Evidence |
| --- | --- | --- |
| `jq empty ...graphiti_temporal_validity_blocked.json ...openviking_*.json` | `pass` | All patched JSON fixtures parse. |
| `python3 -m py_compile scripts/graphiti-zep-docker-temporal-smoke.py` | `pass` | The generated manifest scenario change is syntactically valid. |
| `cargo run -p elf-eval --bin real_world_job_benchmark -- run --fixtures apps/elf-eval/fixtures/real_world_memory/context_trajectory --out tmp/real-world-memory/context-trajectory/report.json --run-id real-world-memory-context-trajectory --adapter-id fixture_context_trajectory --adapter-name 'ELF context trajectory fixture'` | `pass` | Report has 3 jobs, 0 pass, 0 wrong_result, 3 blocked, trace explainability count 3, and expected evidence recall 9/9. |
| `cargo run -p elf-eval --bin real_world_job_benchmark -- publish --report tmp/real-world-memory/context-trajectory/report.json --out tmp/real-world-memory/context-trajectory/report.md` | `pass` | Markdown report renders the OpenViking trace-stage blockers. |
| `cargo run -p elf-eval --bin real_world_job_benchmark -- run --fixtures apps/elf-eval/fixtures/real_world_external_adapters/graph_rag --out tmp/real-world-memory/graph-rag/report.json --run-id real-world-memory-graph-rag --adapter-id fixture_graph_rag_external_adapters --adapter-name 'Graph/RAG representative external-adapter fixtures'` | `pass` | Graphiti/Zep job remains `blocked`, has `temporal_validity_not_encoded = true`, produces all three temporal/provider evidence ids, and reports `graphiti.provider_boundary`. |
| `cargo run -p elf-eval --bin real_world_job_benchmark -- publish --report tmp/real-world-memory/graph-rag/report.json --out tmp/real-world-memory/graph-rag/report.md` | `pass` | Markdown report renders the Graphiti/Zep temporal blocker under Trace Explainability. |

## Scenario Readback

| Scenario | Current outcome | Materialized readback | Claim boundary |
| --- | --- | --- | --- |
| Graphiti/Zep temporal validity | `blocked` | Current fact contract, historical fact contract, provider boundary, and `graphiti.provider_boundary` trace stage. | No pass until live Graphiti search maps validity windows and source ids. |
| OpenViking staged retrieval trajectory | `blocked` | Same-corpus gate plus missing stage-artifact gate; decoy ELF win evidence is dropped. | No ELF win, tie, or loss until both systems publish comparable stage artifacts. |
| OpenViking hierarchy selection | `blocked` | Same-corpus gate plus hierarchy-artifact gate; selected node and rejected sibling/decoy evidence is required. | OpenViking hierarchy remains a design reference, not a scored comparison. |
| OpenViking recursive/context expansion | `blocked` | Same-corpus gate, recursive expansion gate, comparison gate, and dropped trace-doc decoy. | No ELF tie, win, or loss until comparable expansion-path artifacts exist. |

## Requirement Refinement

- Temporal graph validity requires materialized current fact ids, historical fact ids,
  validity windows, source ids, and rationale/update evidence, or a typed setup,
  runtime, or provider blocker.
- Context trajectory requires stage-level readback for same-corpus coverage,
  selected hierarchy nodes, rejected siblings or decoys, expansion paths, pruned
  branches, and the comparison gate.
- Recall planning traces should keep blocked, dropped, demoted, distractor, and
  not-tested context visible instead of collapsing missing adapter artifacts into a
  broad retrieval failure or a false parity claim.

## Not Claimed

- No hosted Zep or broad Graphiti/Zep graph-memory quality claim.
- No OpenViking context-trajectory pass, win, tie, or loss.
- No replacement of ELF source authority by graph-lite or external graph output.
- No private-corpus, provider-backed, or large-corpus performance result.
