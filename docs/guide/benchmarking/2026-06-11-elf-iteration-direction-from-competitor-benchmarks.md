# ELF Iteration Direction From Competitor Benchmarks - June 11, 2026

Goal: Convert the current benchmark evidence and competitor-strength matrix into an
iteration direction for ELF without overstating wins.
Read this when: You need to decide what ELF should learn from adjacent memory,
RAG, graph, and agent-continuity projects.
Inputs: `2026-06-11-competitor-strength-evidence-matrix.md`,
`2026-06-10-live-real-world-sweep-report.md`,
`2026-06-10-production-adoption-refresh.md`,
`2026-06-10-real-world-comparison-report.md`,
`apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`,
and `docs/guide/research/external_memory_improvement_plan.md`.
Depends on: `docs/spec/real_world_agent_memory_benchmark_v1.md`.
Outputs: Current measured data, scenario gaps, and a prioritized optimization
direction for future ELF work.

## Executive Judgment

ELF is a credible personal-production foundation for a high-trust memory service, but
the current evidence does not prove broad superiority over all tracked projects.

The strongest current statement is:

- ELF is ahead on source-of-truth discipline, evidence-bound writes, rebuildable
  derived indexes, typed failure reporting, and checked-in production-operation
  evidence.
- ELF and qmd are tied on the encoded live retrieval, work-resume, and
  project-decision slices. ELF does not yet beat qmd's local retrieval-debug
  ergonomics, but ELF now has a narrow live operator-debug win over qmd on trace
  hydration and candidate-drop visibility.
- Many competitor strengths are still undermeasured: OpenViking context trajectory,
  mem0/OpenMemory entity history and UI, agentmemory and claude-mem capture breadth,
  Letta core-vs-archival memory, Graphiti/Zep temporal graph behavior, and
  llm-wiki/gbrain/graphify knowledge workflows.
- The right next strategy is not to replace ELF with any one project. It is to keep
  ELF's evidence-bound core and absorb the best measured or plausible product
  patterns behind benchmark gates.

## Current Measured Data

### Fixture-Backed ELF Aggregate

`cargo make real-world-memory` currently reports:

| Metric | Value |
| --- | ---: |
| Jobs | `40` |
| Encoded suites | `11` |
| Pass | `38` |
| Blocked | `2` |
| Wrong result | `0` |
| Lifecycle fail | `0` |
| Incomplete | `0` |
| Not encoded | `0` |
| Unsupported claim | `0` |
| Mean score | `0.950` |
| Evidence coverage | `88/88` |
| Expected evidence recall | `80/80` |

This proves the fixture contract is broad and well controlled. It does not prove that
every live adapter or every competitor runtime passes those scenarios.

### Live Real-World Sweep

`cargo make real-world-memory-live-adapters` produced comparable full-suite live
sweeps for ELF and qmd:

| Adapter | Jobs | Pass | Wrong result | Incomplete | Blocked | Not encoded | Mean score | Evidence recall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ELF live service adapter | `40` | `22` | `5` | `0` | `2` | `11` | `0.599` | `50/80` |
| qmd live CLI adapter | `40` | `17` | `6` | `0` | `2` | `15` | `0.461` | `38/80` |

Interpretation:

- ELF is five passes ahead in this full live sweep because qmd misses the delete/TTL
  tombstone job and keeps the capture/write-policy suite typed `not_encoded`.
- Both pass `trust_source_of_truth`, `work_resume`, `project_decisions`,
  `retrieval`, and `personalization`.
- Both fail most `memory_evolution` live conflict evidence with `wrong_result`.
- ELF now passes live `capture_integration`; qmd keeps that suite `not_encoded`.
  Both leave consolidation, knowledge compilation, and production-ops operator
  boundaries as `not_encoded` or `blocked`. Operator debugging has a separate narrow
  live slice: ELF passes it, while qmd remains `wrong_result` for trace hydration and
  candidate-drop stage visibility.

### Production Evidence

ELF has the strongest production-operation evidence among the tracked systems:

| Run | Scope | Result |
| --- | --- | --- |
| Provider synthetic | 8 documents, 6 queries, Qwen3-Embedding-8B, 4096 dimensions | `8/8`, `pass`, 59 seconds |
| Provider stress | 480 generated documents, 16 queries | `9/9`, `pass`, 779 seconds |
| Provider backfill | 2,000 generated documents, 16 queries, resume 1,000 -> 2,000 | `9/9`, `pass`, 2,804 seconds |
| Restore proof | Docker Compose backup/restore plus Qdrant rebuild | restored note searchable, zero rebuild errors |
| Private production corpus | operator-owned manifest required | failed closed, no pass claimed |

This is enough to support personal production use with bounded caveats. It is not a
private-corpus quality proof.

### External Adapter Ledger

The current adapter manifest records 23 adapter records across 17 projects:

| Evidence class | Count | Meaning |
| --- | ---: | --- |
| `fixture_backed` | `1` | ELF real-world fixture scoring. |
| `live_baseline_only` | `6` | Docker same-corpus or lifecycle evidence without real-world job scoring. |
| `live_real_world` | `5` | ELF and qmd full-suite live sweeps, graphify's tiny scored Docker smoke, and the narrow ELF/qmd operator-debug live slice. |
| `research_gate` | `11` | Source/setup/resource/output-contract evidence only. |

Overall adapter statuses:

| Status | Count |
| --- | ---: |
| `pass` | `4` |
| `wrong_result` | `6` |
| `lifecycle_fail` | `1` |
| `blocked` | `6` |
| `not_encoded` | `6` |

The ledger is intentionally not a leaderboard. It prevents fixture evidence,
same-corpus checks, research gates, and live real-world runs from being collapsed into
one misleading score.

## Scenario Conclusions

| Scenario | Current position | What ELF should learn next |
| --- | --- | --- |
| Retrieval/debug | ELF and qmd are tied on encoded live retrieval; qmd remains the stronger debug UX reference. | Add trace-level replay, expansion/fusion/rerank knobs, candidate-drop diagnosis, and command-line replay. |
| Work resume | ELF live work-resume passes; continuity-oriented competitors are undermeasured. | Borrow agentmemory/claude-mem capture breadth and OpenViking staged context, but require durable adapter proof. |
| Project decisions | ELF and qmd live project-decision suites pass; ELF fixture-backed `core_archival_memory` also scores project-decision recovery, while Letta remains blocked without export evidence. | Run the Letta core/archival export/readback contract before treating project-decision recovery as comparable. |
| Source of truth | ELF has the strongest measured source-of-truth evidence. | Borrow memsearch's local canonical-store ergonomics without making files or vectors authoritative. |
| Temporal memory | ELF fixture passes, but live memory evolution is wrong_result. | Prioritize current-vs-historical evidence links and Graphiti/Zep-style validity windows. |
| Consolidation | ELF fixture passes, but live proposal generation is not encoded. | Build reviewable derived proposals with source refs, confidence, unsupported-claim flags, and apply/defer/discard audit. |
| Knowledge pages | ELF fixture pages pass; live knowledge generation is not encoded. | Borrow llm-wiki lint/query-save loops, gbrain timelines, and graphify reports behind rebuild/lint benchmarks. |
| Operator debugging | Fixture UX passes and the narrow live trace/viewer slice is scored: ELF passes, qmd ties replay/repair clarity but is wrong_result for trace hydration and candidate-drop visibility. | Expand coverage to OpenMemory and claude-mem UI/export or viewer runners before any broader operator-UX claim. |
| Capture/write policy | ELF live capture/write-policy self-check passes with zero redaction leaks; qmd is `not_encoded`; agentmemory is `blocked`; claude-mem is `not_encoded`. | Borrow agentmemory/claude-mem capture breadth only after durable local hook/viewer evidence exists, while preserving redaction and evidence binding. |
| Production ops | ELF has the strongest checked-in evidence, with private/credential gates blocked. | Keep Docker-first production proof and add private corpus only when an operator-owned manifest exists. |
| Personalization | ELF live personalization passes; mem0/OpenMemory is not encoded and Letta scoped preference readback remains not tested until its contained export path exists. | Add entity-scoped preference history and UI readback before claiming stronger personalization. |
| Context trajectory | Not comparable yet; OpenViking remains the reference. | Score staged retrieval, hierarchy expansion, and trajectory readback. |
| Core-vs-archival | ELF fixture-backed `core_archival_memory` passes 6/6, but Letta remains blocked/not tested because no contained export artifact exists. | Borrow Letta's core memory block shape while keeping any win/tie/loss claim gated on exported core block, archival readback, and source-id evidence. |
| Graph/RAG navigation | RAGFlow, LightRAG, GraphRAG, and Graphiti/Zep remain research gates; graphify has a tiny scored `wrong_result` smoke. | Run larger contained graph/RAG adapters before any broad graph-navigation claim. |

## Project Guidance Matrix

| Project | Current evidence | User-facing strength | ELF direction |
| --- | --- | --- | --- |
| ELF | `fixture_backed` plus `live_real_world`; live full sweep is `wrong_result`; live capture/write-policy self-check passes. | Evidence-linked memory service, strict provenance, rebuildable Qdrant, production backfill/restore proof. | Keep this as the core; do not weaken source-of-truth, write-policy, or typed failure semantics while adding product ergonomics. |
| qmd | `live_real_world` plus `live_baseline_only`; targeted retrieval passes, full sweep is `wrong_result`. | Local retrieval-debug workflow, transparent CLI, weighted fusion, rerank, replayable commands. | Treat qmd as the retrieval-debug bar. ELF should match its introspection and local replay without becoming CLI-only. |
| agentmemory | `live_baseline_only`; current status is `lifecycle_fail`; capture breadth comparison is blocked by process-local StateKV Map and in-memory index. | Coding-agent continuity, hooks, MCP/REST packaging, viewer/console observability. | Borrow capture breadth and continuity UX, but require durable lifecycle and capture artifact proof before claims. |
| mem0/OpenMemory | `live_baseline_only`; basic local smoke now passes, while entity/preference history, hosted ecosystem, graph memory, and OpenMemory UI remain untested locally. | Entity-scoped memory, lifecycle/history surfaces, hosted ecosystem, OpenMemory UI. | Add entity/preference history and UI readback patterns, while keeping hosted claims out of local OSS benchmarks. |
| memsearch | `live_baseline_only`; canonical Markdown reindex/reload smoke now passes, while real-world source-of-truth prompts remain unencoded. | Markdown-first canonical store and local reindex clarity. | Borrow local inspectability and canonical-file ergonomics, not file-as-authority semantics. |
| OpenViking | `live_baseline_only` plus `research_gate`; current status is `wrong_result`. | Filesystem-like context model, hierarchy, staged context trajectory. | Add staged retrieval and trajectory scoring after same-corpus evidence output is correct. |
| claude-mem | `live_baseline_only`; current status is `wrong_result`; hook/viewer capture breadth is not encoded. | Progressive disclosure, automatic capture, local viewer workflow. | Borrow progressive disclosure and viewer comfort; benchmark capture and operator-debugging live paths before claims. |
| RAGFlow | `research_gate`; current status is `blocked`. | Full RAG application workflow with document/chunk/reference handles. | Use as a resource-aware RAG adapter benchmark, not as a current ELF competitor win/loss. |
| LightRAG | `research_gate`; current status is `blocked`. | Lightweight graph/RAG context export and source-path citation shape. | Borrow context-export ideas for graph/RAG navigation after Docker proof. |
| GraphRAG | `research_gate`; current status is `blocked`. | Graph summaries, document/text-unit tables, local/global search separation. | Borrow graph summary artifacts for knowledge pages and graph navigation after cost-bounded output proof. |
| Graphiti/Zep | `research_gate`; current status is `blocked`. | Temporal graph facts, validity windows, current-vs-historical answers. | Use as the semantic model for ELF temporal memory and relation validity benchmarks. |
| Letta | `research_gate`; current status is `blocked` until the selected contained export/readback artifact exists. | Core memory blocks versus archival memory. | Keep ELF's fixture-backed core block coverage separate from Letta comparison claims; compare Letta only after exported core and archival evidence exists. |
| LangGraph | `research_gate`; current status is `not_encoded` or `unsupported` as a direct memory backend. | Checkpoint, replay, fork, and regression debugging for agent state. | Borrow replay/regression patterns for benchmark infrastructure, not as direct memory parity. |
| nanograph | `research_gate`; current status is `not_encoded` or `unsupported` as a full memory backend. | Typed graph schema and query ergonomics. | Borrow graph-lite DX and typed relation query ideas. |
| llm-wiki | `research_gate`; current status is `not_encoded`. | Maintained wiki pages, query-save, lint, and repair loops. | Use as a reference for rebuildable, cited knowledge pages. |
| gbrain | `research_gate`; current status is `not_encoded` and setup-blocked. | Compiled truth pages, timelines, and human-operable knowledge navigation. | Borrow current-truth plus timeline presentation after Docker-local setup proof exists. |
| graphify | `live_real_world`; tiny scored smoke is `wrong_result`. | `graph.json`, `GRAPH_REPORT`, source-location graph navigation. | Treat the tiny smoke as bounded non-pass evidence and expand only after representative graph/RAG jobs map to evidence ids. |

## Optimization Direction

### P0 - Close Measured Quality Gaps

These are the highest leverage because current evidence already shows an ELF gap, a
close competitor surface, or a still-unmeasured product strength.

1. Live memory evolution correctness
   - Current state: fixture pass, live `wrong_result`.
   - Borrow from: Graphiti/Zep validity windows, mem0 history, ELF ingest-decision
     audit rows.
   - Target: live answers cite both current and historical conflict evidence, not only
     current retrieved text.
   - Benchmark gate: live `memory_evolution` pass for ELF before superiority claims.

2. qmd-level retrieval debugging
   - Current state: ELF and qmd tie on encoded results; qmd remains stronger in
     local debug ergonomics.
   - Borrow from: qmd weighted fusion, rerank explanation, local replay commands.
   - Target: every wrong result can be traced through expansion, dense retrieval,
     sparse retrieval, fusion, rerank, graph context, and final selection.
   - Benchmark gate: qmd deep profile plus ELF/qmd trace-level replay report.

3. Live operator debugging UX
   - Current state: fixture pass; narrow live ELF/qmd slice scored with ELF `pass`
     and qmd `wrong_result`.
   - Borrow from: claude-mem viewer, OpenMemory inspector, qmd command output.
   - Target: no raw SQL needed to explain a bad memory result, across service traces,
     CLI replay, and bounded local viewer surfaces.
   - Benchmark gate: add OpenMemory and claude-mem UI/export or viewer runners before
     claiming broader operator-debug UX superiority.

### P1 - Turn ELF Into A Better Daily Memory Product

These improve day-to-day usefulness while preserving ELF's evidence-bound core.

1. Capture and continuity
   - Borrow from: agentmemory hook breadth and claude-mem automatic capture review.
   - Current state: ELF live capture/write-policy self-check passes; agentmemory is
     blocked and claude-mem is not encoded for capture breadth.
   - ELF shape: live ingestion must continue to preserve redaction, excluded spans,
     source ids, and write-policy audit.
   - Benchmark gate: durable agentmemory and claude-mem capture-hook runners with
     no secret leakage and evidence-bound output.

2. Reviewable consolidation
   - Borrow from: managed memory dreaming and Always-On Memory Agent scheduling.
   - ELF shape: derived proposals only; source notes are not silently rewritten.
   - Benchmark gate: consolidation proposals include lineage, confidence,
     unsupported-claim flags, and apply/defer/discard audit.

3. Knowledge pages
   - Borrow from: llm-wiki, gbrain, graphify, and GraphRAG.
   - ELF shape: project/entity/concept pages are rebuilt from authoritative notes and
     linted for unsupported or stale sections.
   - Benchmark gate: live knowledge-page rebuild/lint report, not fixture-only proof.

4. Core memory blocks
   - Borrow from: Letta core memory versus archival memory.
   - ELF shape: scoped read-only blocks with provenance and attachment rules, separate
     from archival search.
   - Benchmark gate: ELF fixture jobs now prove attachment, scope, provenance,
     stale-core detection, archival fallback, and project-decision recovery; Letta
     comparison remains gated on exported core block, archival readback, and source-id
     evidence.

### P2 - Expand External Comparison Without Fake Wins

These are needed for broad credibility but should not block personal production use.

1. RAG and graph adapters
   - Current state: RAGFlow, LightRAG, GraphRAG, and Graphiti/Zep remain typed
     research gates; graphify has a tiny scored `wrong_result` smoke.
   - Benchmark gate: Docker-contained adapters must emit evidence-linked outputs
     before any live pass claim.

2. OpenViking context trajectory
   - Current state: setup is pinned, same-corpus retrieval is `wrong_result`, and
     staged trajectory is `not_encoded`.
   - Benchmark gate: evidence-bearing retrieval pass, then staged hierarchy/trajectory
     scoring.

3. mem0/OpenMemory and memsearch coverage
   - Current state: both now pass the basic local OSS smoke, but their strongest
     real-world scenarios remain unencoded.
   - Benchmark gate: score mem0/OpenMemory entity history and UI readback, plus
     memsearch source-of-truth and retrieval-debug workflows.

## What Not To Claim Yet

Do not claim:

- ELF beats qmd overall. ELF is five passes ahead in the fresh aggregate because qmd
  misses the delete/TTL tombstone job and keeps capture/write-policy jobs
  `not_encoded`, but neither adapter has full-suite live pass evidence and qmd still
  owns stronger local retrieval-debug ergonomics.
- ELF has full-suite live real-world pass evidence. It does not.
- ELF has private-corpus production quality proof. The private profile currently
  fails closed without an operator-owned manifest.
- ELF beats OpenViking on context trajectory. That scenario is not encoded.
- ELF beats mem0/OpenMemory on hosted memory, entity history, UI, or optional graph
  memory. Those scenarios are not encoded; the operator-debug win is only against
  qmd on a narrow trace/replay slice.
- ELF beats Letta on core-vs-archival memory. ELF has fixture-backed coverage, but
  Letta remains blocked/not tested until the selected contained export/readback path
  produces comparable source-id-mapped evidence.
- ELF beats RAGFlow, LightRAG, GraphRAG, Graphiti/Zep, or graphify on graph/RAG
  navigation. Current evidence is research-gate or blocked except graphify's tiny
  non-pass smoke.

## Suggested Report Cadence

Use this cadence for future benchmark-driven iteration:

1. Keep `2026-06-11-competitor-strength-evidence-matrix.md` as the claim gate.
2. Keep this report as the optimization direction.
3. For each new adapter or suite, publish a dated benchmark report only when the run
   changes a README-level claim or a production-adoption decision.
4. Every report must classify evidence as `fixture_backed`, `live_baseline_only`,
   `live_real_world`, or `research_gate`.
5. Do not promote a reference project into a win/loss claim until the relevant
   scenario is encoded and run at a comparable evidence class.

## Recommended Next Reports

The next reporting work should be ordered by decision value:

1. ELF/qmd retrieval-debug deep profile.
2. ELF live memory-evolution repair report.
3. OpenMemory and claude-mem operator-debug UI/export runners.
4. agentmemory and claude-mem capture-hook breadth report.
5. OpenViking context-trajectory report after evidence-bearing retrieval works.
6. RAG/graph adapter pack report after Docker-contained outputs map to evidence ids.

These are report and measurement directions, not implementation commitments.
