---
type: Spec
title: "Agent Memory and Knowledge System v1"
description: "Define the ELF Agent Memory + Knowledge System product contract, roadmap, phase gate, and claim boundaries."
resource: docs/spec/agent_memory_knowledge_system_v1.md
status: active
authority: normative
owner: spec
last_verified: 2026-06-23
tags:
  - docs
  - spec
  - agent-memory
  - knowledge
source_refs: []
code_refs:
  - Makefile.toml
related:
  - docs/evidence/benchmarking/2026-06-20-agent-knowledge-os-closeout-benchmark-report.md
  - docs/evidence/benchmarking/2026-06-22-p1-memory-authority-closeout-report.md
  - docs/evidence/benchmarking/2026-06-23-p4-quality-hardening-productization-readiness-report.md
  - docs/runbook/benchmarking/real_world_agent_memory_benchmark.md
  - docs/spec/real_world_agent_memory_benchmark_v1.md
  - docs/spec/system_elf_memory_service_v2.md
  - docs/spec/system_knowledge_pages_v1.md
  - docs/spec/system_recall_debug_panel_v1.md
  - docs/spec/system_graph_memory_postgres_v1.md
  - docs/spec/system_memory_summary_v1.md
drift_watch:
  - docs/spec/agent_memory_knowledge_system_v1.md
  - docs/evidence/benchmarking/2026-06-20-agent-knowledge-os-closeout-benchmark-report.md
  - docs/evidence/benchmarking/2026-06-23-p4-quality-hardening-productization-readiness-report.md
  - docs/runbook/benchmarking/real_world_agent_memory_benchmark.md
  - Makefile.toml
---
# Agent Memory and Knowledge System v1

Purpose: Define the ELF Agent Memory + Knowledge System product contract, roadmap,
phase gate, and claim boundaries.
Status: normative
Read this when: You are shaping product work, opening implementation issues, reviewing
Agent Memory + Knowledge System claims, or deciding which phase may be queued.
Not this document: Low-level service API semantics, benchmark fixture schemas,
operator run commands, or implementation details for one subsystem.
Defines: `elf.agent_memory_knowledge_system/v1` product boundary, P0-P5 roadmap,
phase-gate rules, agent-facing surfaces, UI role, benchmark metrics, competitor
absorption rules, and phase closeout checklist.

## Product Contract

ELF is an open-source Agent Memory + Knowledge System.

ELF turns sources into traceable knowledge, promotes reliable knowledge into agent
memory, and makes recall explainable, correctable, rollbackable, and benchmarked.

The lead wedge is source-linked memory authority plus recall/debug quality. ELF must
not be positioned as a generic RAG framework, wiki compiler, hosted memory SDK,
graph database, or document-search replacement.

## System Boundary

The product is composed of six typed layers:

| Layer | Authority | Required boundary |
| --- | --- | --- |
| Source Library | Captured documents, excerpts, imports, and source refs. | Sources remain evidence. Derived memory and pages must cite sources instead of replacing them. |
| Memory Authority | Notes, core blocks, ingest decisions, history, corrections, and rollback evidence. | Memory writes are policy-gated, evidence-linked, auditable, and reversible. |
| Knowledge Workspace | Derived project, entity, concept, issue, decision, author, and timeline pages. | Pages are rebuildable derived artifacts with citations, lint, and stale-source detection. |
| Graph-lite Facts | Postgres-backed relation facts and temporal markers. | Graph facts are source-backed context, not a separate authority store. |
| Dreaming Review | Reviewable consolidation, summary, brief, tag, correction, and promotion proposals. | Derived proposals must be reviewable and must not mutate sources without an explicit accepted transition. |
| Recall Debug | Search traces, dropped candidates, source/doc/page/graph/proposal rows, and replay aids. | Recall must expose why context was selected, dropped, unavailable, blocked, or not requested. |

Existing subsystem specs own their detailed contracts. This document owns how those
subsystems fit into the Agent Memory + Knowledge System product boundary.

## Non-Goals

- Do not turn ELF into a broad RAGFlow, OpenKB, PageIndex, mem0, Zep, Letta, qmd,
  OpenViking, agentmemory, claude-mem, or memsearch replacement.
- Do not weaken Postgres source-of-truth, source-ref, evidence-binding, English-gate,
  scope, lifecycle, or review boundaries to match another product's ergonomics.
- Do not claim hosted managed-memory, private-corpus, provider-backed, UI/export,
  graph/RAG, core/archive, context-trajectory, or long-document parity without
  same-corpus checked-in or operator-owned evidence for that exact claim.
- Do not collapse `blocked`, `incomplete`, `not_encoded`, `wrong_result`, or
  `unsupported_claim` states into pass claims.
- Do not queue later phases while the current accepted phase is still under review.

## Data Model Direction

All implementation phases must preserve the source-to-memory authority chain:

1. Sources are captured as documents, excerpts, event audits, issue/PR records, or
   other source refs with stable provenance.
2. Candidate knowledge is derived from sources as proposals, page sections, graph
   facts, summaries, or memory candidates.
3. Promotion into memory records an explicit policy decision, source refs, actor,
   confidence, importance, lifecycle state, and audit trail.
4. Correction and rollback create durable history instead of silently rewriting the
   evidence chain.
5. Recall reads from typed surfaces and returns enough trace data to debug selection,
   demotion, filtering, staleness, and missing anchors.

Postgres remains the authority for notes, docs metadata, graph-lite facts, derived
pages, proposal review state, traces, and audit history. Qdrant and any future
retrieval index remain derived and rebuildable.

## Agent-Facing Surfaces

Agent-facing tools must be thin MCP or HTTP facades over typed service behavior.
Business logic and policy remain in `elf-api` and `elf-service`.

Current and future Agent Memory + Knowledge System work should use these surface
families:

| Surface family | Examples | Boundary |
| --- | --- | --- |
| Source capture and hydration | `elf_docs_put`, `elf_docs_search_l0`, `elf_docs_excerpts_get` | Capture and retrieve source evidence without promoting it to memory by default. |
| Memory write and readback | `elf_notes_ingest`, `elf_events_ingest`, `elf_searches_create`, `elf_searches_notes`, `elf_core_blocks_get`, `elf_entity_memory_get` | Writes must preserve policy and evidence decisions; reads must honor scopes and lifecycle. |
| Provenance and history | `elf_admin_note_provenance_get`, `elf_admin_memory_history_get`, trace bundle tools | Debug memory authority without raw database access in normal workflows. |
| Knowledge and graph context | Knowledge page search/readback, `elf_graph_query`, graph report surfaces | Expose derived knowledge and graph facts as labeled context, not authoritative note hits. |
| Dreaming review | Dreaming review queue and proposal review surfaces | Keep proposals reviewable; auto-apply is limited to explicitly accepted low-risk derived organization cases. |
| Recall debug | `elf_recall_debug_panel`, trace and trajectory readback | Show selected, dropped, available, reviewable, blocked, and not-requested context. |

New MCP tools must name the underlying authority layer, link to the owning spec, and
preserve read/write boundaries. A readback tool must not become a hidden mutation path.

## UI Role

The UI is an operator console for source review, memory authority, knowledge pages,
proposal review, graph/topic inspection, and recall debugging.

The UI must:

- label authoritative notes, derived pages, graph facts, proposals, and trace rows
  differently;
- show citations, lint state, review state, lifecycle state, and rollback/correction
  affordances where applicable;
- prefer typed service readback over raw store inspection;
- avoid presenting derived pages or proposals as current memory unless they have been
  promoted through the relevant authority path.

The UI is not the source of truth and must not bypass API, MCP, scope, review, or
write-policy contracts.

## Roadmap

The roadmap phases below are product phases. They are not broad permission to queue or
implement every item in a phase at once.

| Phase | Name | Scope | Gate to leave phase |
| --- | --- | --- | --- |
| P0 | Product contract and phase gate | Codify this product boundary, roadmap, competitor absorption rules, validation expectations, and closeout checklist. | Docs are reviewed, repo docs validation passes, claim boundaries match the June 20 closeout evidence, and the main thread accepts the next phase. |
| P1 | Memory Authority MVP loop | Deliver one source-backed memory-authority vertical slice: capture source evidence, create/review one proposal through a proposal inbox, record the authority ledger, apply/correct/rollback, recall through agent-facing tools, and debug stale/correction behavior. | The slice has service tests, provenance/history evidence, recall/debug readback, and at least one real-world stale/correction benchmark job. |
| P2 | Knowledge Workspace | Promote source-linked project/entity/concept/issue/decision/author/timeline pages with rebuild, lint, watch, search, and version-diff readback. | Pages stay derived, every section is cited or explicitly unsupported, stale-source lint runs, and benchmark reports publish citation/staleness metrics. |
| P3 | Competitor-strength adapters | Add contained comparison adapters for qmd replay, PageIndex/OpenKB, mem0/OpenMemory, Letta, Graphiti/Zep, OpenViking, graph/RAG references, and other accepted deltas. | Each adapter preserves typed non-pass states and emits same-corpus evidence or a concrete typed setup blocker before any parity, win, tie, or loss claim. |
| P4 | Benchmark and quality hardening | Expand adversarial jobs, public comparison grammar, quality metrics, latency/cost/resource reporting, and unsupported-claim detection. | Reports preserve job/suite/project typed states, expected evidence recall, irrelevant context ratio, unsupported claims, and resource metrics. |
| P5 | Productization | Improve local setup, agent recipes, operator UI, privacy/delete/export boundaries, and production-quality workflows. | Operator workflows have documented setup, privacy/delete/export semantics, and validation evidence without weakening source authority. |

### First Implementation Phase Constraint

The first implementation issue after P0 must be the smallest coherent P1 vertical
slice. It may touch only the surfaces needed to prove one source-linked
memory-authority loop end to end.

The first P1 issue must not build the full Knowledge Workspace, broad operator UI,
external adapter pack, hosted memory behavior, graph/RAG parity, or product-wide
rewrites. Those are later phases unless a main-thread decision explicitly narrows and
accepts a different next slice.

## Decodex Phase Gate

Decodex execution for this project is single-phase gated:

- Only the next accepted phase may carry the service-scoped queue label
  `decodex:queued:elf`.
- Later-phase issues must remain unqueued while the current phase is running, under
  review, or waiting for main-thread acceptance.
- After each phase lands, the main thread must review evidence, tests, benchmark
  results, claim boundaries, and next-phase readiness before any later issue receives
  `decodex:queued:elf`.
- `decodex:active:elf` means runtime ownership of an active lane. It is not a request
  to start additional phases.
- `In Review` is a PR-backed handoff state. It is not phase acceptance by itself.

As of the June 22, 2026 XY-1063 closeout, P1 has fixture-backed self-assessment
evidence for the memory-authority MVP loop. P2 queueing remains conditional on
main-thread acceptance of that closeout and selection of one narrow next P2 issue.

As of the June 23, 2026 XY-1075 closeout, P4 has fixture-backed quality hardening
and production-readiness self-assessment evidence across adversarial memory,
Source Library, Knowledge Workspace, and production-ops slices. P5 productization
work may be queued only after main-thread acceptance of that closeout, and only for
the proven local/public workflows named there. Private-corpus quality,
provider-backed quality, hosted managed-memory parity, external adapter parity, and
broad competitor superiority remain unqueued until their own evidence gates pass.

## Competitor Absorption Rules

External projects are references for targeted improvements. They are not hidden
dependencies and are not automatic proof that ELF is weaker or stronger.

| Competitor/reference | Strength to absorb | Claim boundary |
| --- | --- | --- |
| qmd | Transparent expansion, fusion, rerank, top-k, and compact replay ergonomics. | Preserve qmd's debug edge until ELF emits comparable replay artifacts. |
| VectifyAI PageIndex | Long-document tree retrieval and PageIndex MCP ecosystem direction. | No win/tie/loss claim until a same-corpus adapter compares tree artifacts, cited node paths, and MCP readback with ELF source ids and source refs. |
| VectifyAI OpenKB | Compiled Markdown wiki, concept/entity pages, lint, watch, and recompile workflows. | Absorb into Knowledge Workspace only through source-id-mapped wiki, index, lint, and watch/recompile artifacts; derived wiki pages must not become source memory. |
| OpenViking | Filesystem-like context URIs, hierarchy selection, staged trajectory, and recursive expansion. | Keep trajectory/hierarchy claims blocked until same-corpus staged artifacts exist. |
| mem0/OpenMemory | Entity-scoped history, hosted ecosystem, UI/export, and optional graph memory direction. | Separate local SDK history evidence from hosted, UI/export, and optional graph-memory parity. |
| Letta | Core/archive memory split and export/readback model. | No core/archive parity claim until contained Letta export/readback artifacts include source ids. |
| Graphiti/Zep and graph/RAG projects | Temporal graph validity, citation/navigation, and graph retrieval references. | Graph-lite reports are ELF-native evidence, not broad graph/RAG parity. |
| agentmemory and claude-mem | Capture hooks, local viewers, continuity UX, and progressive disclosure. | Improve operator UX and capture audit without dropping evidence, scope, or write-policy gates. |
| memsearch | Markdown-first canonical store, incremental reindex, and local hybrid retrieval. | Treat as workflow inspiration; ELF's source-of-truth remains Postgres plus typed source refs. |

Allowed claims:

- ELF is the strongest measured integrated Agent Knowledge OS product in the June 20,
  2026 checked-in matrix.
- ELF has complete same-repo evidence across the six Agent Knowledge OS layers in
  that matrix.
- Competitor strengths remain optimization inputs and comparison targets.

Disallowed claims:

- ELF broadly beats every competitor on every competitor-owned strength.
- Reference-only, blocked, incomplete, wrong-result, or not-tested evidence is a pass.
- Public-proxy or local fixture evidence proves private-corpus or provider-backed
  production quality.

## Benchmark Metrics

Phase closeout and comparison reports must use the real-world benchmark vocabulary
instead of broad leaderboards.

Required quality dimensions are:

- `answer_correctness`
- `evidence_grounding`
- `trap_avoidance`
- `uncertainty_handling`
- `workflow_helpfulness`

Use optional dimensions when the phase touches them:

- `lifecycle_behavior`
- `debuggability`
- `latency_resource`
- `personalization_fit`

Reports must preserve typed outcomes:

- `pass`
- `wrong_result`
- `lifecycle_fail`
- `incomplete`
- `blocked`
- `not_encoded`
- `unsupported_claim`

Relevant phase reports should also publish expected evidence recall, irrelevant context
ratio, unsupported-claim counts, stale-answer counts, source-ref coverage, citation
coverage, freshness/rationale coverage, proposal lineage completeness, source mutation
count, trace explainability counters, and latency/cost/resource metrics when those
metrics apply to the touched phase.

## Validation

Repository-native validation is authoritative.

- Use `Makefile.toml` as the source of truth for task names.
- For docs-only phase work, run at least `cargo make check-docs` before claiming the
  docs are validation-ready.
- Before a PR handoff or any push that refreshes a PR head, run the registered
  Decodex workflow gate: `cargo make fmt`, `cargo make lint-fix`, then
  `cargo make check`.
- If a phase changes commands, schemas, config, runtime behavior, status semantics,
  or benchmark claims, update the owning docs and include drift evidence as required
  by `docs/policy.md`.

## Phase Closeout Checklist

Every phase closeout must answer these checks before the next phase can be queued:

- Evidence: source refs, artifacts, traces, screenshots, or reports prove the claims
  made by the phase.
- Tests: repo-native validation ran, and failures are either fixed or recorded as
  explicit blockers.
- Benchmark: relevant real-world jobs or typed benchmark reports exist, or untouched
  areas are explicitly `not_encoded` or out of scope.
- Claim boundary: the closeout does not convert blocked, incomplete, wrong-result,
  not-tested, public-proxy, local fixture, or reference-only evidence into parity or
  production claims.
- Next-phase readiness: the next phase has one accepted issue narrow enough for
  Decodex to execute without broad rewrites, and no later issue is queued.
