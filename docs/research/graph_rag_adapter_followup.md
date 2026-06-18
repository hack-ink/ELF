---
type: Research Contract
title: "Graph and RAG Adapter Follow-Up"
description: "Research contract for unresolved graph/RAG adapter value after the June 2026 feasibility verdicts."
resource: docs/research/graph_rag_adapter_followup.md
status: active
authority: current_state
owner: research
last_verified: 2026-06-18
tags:
  - docs
  - research
  - graph-rag
  - adapter
source_refs: []
code_refs:
  - docs/evidence/external_memory/comparison_external_projects.md
  - docs/evidence/external_memory/research_projects_inventory.md
  - docs/evidence/benchmarking/2026-06-11-graph-rag-scored-smoke-adapter-report.md
  - apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json
related: []
drift_watch:
  - apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json
  - docs/evidence/external_memory/research_projects_inventory.md
---
# Graph and RAG Adapter Follow-Up

Purpose: Preserve only the unresolved, valuable research from the retired
`2026-06-10-xy-882-rag-graph-adapter-feasibility` run.
Read this when: You are deciding whether a RAG or graph-memory project has enough
contained evidence to become a scored ELF real-world adapter.
Not this document: A live adapter pass, a broad quality ranking, or a replacement
decision for ELF core memory.

## Question

Which graph/RAG systems still deserve further research or implementation proof before
ELF can score them as real-world memory adapters?

## Scope

In scope:

- RAGFlow, LightRAG, GraphRAG, Graphiti/Zep, and graphify adapter-candidate follow-up.
- Letta, LangGraph, nanograph, llm-wiki, and gbrain reference-only or blocked value
  that should not be promoted into live evidence.
- Docker containment, resource envelope, source-id output, citation output, and
  typed non-pass states.

Out of scope:

- Host-global installs as proof.
- Provider-backed private corpus claims.
- Any claim that `research_gate` is equivalent to fixture-backed or live evidence.

## Evidence

- `docs/evidence/external_memory/research_projects_inventory.md` owns the accepted June 10,
  2026 verdict table.
- `docs/evidence/external_memory/comparison_external_projects.md` owns the broader project
  comparison and benchmark-dimension map.
- `apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`
  owns the executable adapter ledger.
- `docs/evidence/benchmarking/2026-06-11-graph-rag-scored-smoke-adapter-report.md`
  owns current scored graph/RAG smoke evidence.

## Options

- Promote candidate projects only after Docker execution emits evidence-linked
  adapter output.
- Keep reference-only projects as research inputs for specs and UX, not adapter rows.
- Keep blocked projects blocked until contained setup is proven.

## Judgment

Continue research. The accepted verdicts remain:

- `adapter_candidate`: RAGFlow, LightRAG, GraphRAG, Graphiti/Zep, graphify.
- `research_only`: Letta, LangGraph, nanograph, llm-wiki.
- `blocked`: gbrain until a Docker-local brain repository and database path is proven.

These labels do not imply live adapter quality.

## Challenge

The main risk is label drift: `adapter_candidate` can be mistaken for benchmark
evidence. The mitigation is to preserve `research_gate` until a Docker-contained run
emits source IDs, document IDs, file paths, citations, graph facts, or equivalent
evidence handles that `real_world_job` scoring can inspect.

## Decision

Not decision-ready for live evidence. Keep the active research contract open until the
next adapter implementation or source-review pass either promotes a concrete report or
retires the candidate.

## Promotion

Promote only these outputs:

- Adapter implementation evidence goes to `docs/evidence/benchmarking/`.
- Schema or scoring-contract changes go to `docs/spec/real_world_agent_memory_benchmark_v1.md`.
- Accepted inventory status changes go to `docs/evidence/external_memory/research_projects_inventory.md`.

Do not re-create a raw research JSON owner for this lane.

## Drift Impact

Watch for upstream changes that alter Docker setup, local resource envelope, source
mapping, citation output, or graph/temporal fact output. Also watch for new ELF adapter
rows that should replace this research contract with benchmark evidence.

## Citations

- `docs/evidence/external_memory/comparison_external_projects.md`
- `docs/evidence/external_memory/research_projects_inventory.md`
- `docs/evidence/benchmarking/2026-06-11-graph-rag-scored-smoke-adapter-report.md`
- `apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`
