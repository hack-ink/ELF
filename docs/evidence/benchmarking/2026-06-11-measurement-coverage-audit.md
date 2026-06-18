---
type: Evidence
title: "ELF Benchmark Measurement Coverage Audit - June 11, 2026"
description: "Checked-in benchmark evidence record: ELF Benchmark Measurement Coverage Audit - June 11, 2026."
resource: docs/evidence/benchmarking/2026-06-11-measurement-coverage-audit.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-18
tags:
  - docs
  - evidence
  - benchmarking
---
# ELF Benchmark Measurement Coverage Audit - June 11, 2026

Goal: Record what is actually measured today, where competitor comparisons are still
not comparable, and which measurement reports should guide future ELF iteration.
Read this when: You need to answer whether ELF has enough empirical evidence to
claim a win, tie, loss, or non-claim against tracked memory, RAG, graph, and
agent-continuity projects.
Inputs: Fresh local runs of `cargo make real-world-memory-core-archival`,
`cargo make real-world-memory`, and retained XY-933
`cargo make real-world-memory-live-adapters` evidence after XY-927
core-vs-archival fixture coverage, XY-928 OpenViking context-trajectory fixture
encoding, and live capture/write-policy scoring, plus
`apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`,
`2026-06-11-competitor-strength-evidence-matrix.md`, and
`2026-06-11-elf-iteration-direction-from-competitor-benchmarks.md`.
Outputs: Fresh measured counters, scenario coverage, project coverage, and the next
measurement reports needed before stronger ELF claims.

## Executive Judgment

The benchmark program is useful and already prevents misleading claims, but the
current measured comparison is not complete enough to say ELF beats or ties every
tracked project's strongest scenario.

What is proven today:

- ELF has a strong fixture-backed real-world benchmark contract: 49 jobs across 13
  suites, 44 pass, 5 blocked operator or measurement-gate boundaries, and no wrong
  results in the fixture aggregate. The new `core_archival_memory` suite contributes
  6 passing jobs for core block attachment, scope, provenance, stale-core detection,
  archival fallback, and project-decision recovery. The added XY-928
  `context_trajectory` jobs are blocked OpenViking staged/hierarchy/recursive gates,
  not ELF wins.
- ELF and qmd have comparable full-suite live real-world sweeps, but neither has a
  full-suite live pass. ELF is five passes ahead in the fresh aggregate because qmd
  misses the memory-evolution delete/TTL tombstone job and the capture/write-policy
  suite is now ELF-only live evidence.
- ELF now has live capture/write-policy self-check evidence for redaction, exclusions,
  source ids, evidence binding, and no secret leakage. This is not a broad
  capture-hook win over agentmemory or claude-mem: agentmemory comparison is blocked
  by mocked/in-memory storage, and claude-mem hook/viewer capture remains blocked
  until Docker-contained hook/viewer evidence exists.
- ELF is ahead on production-operation evidence among tracked systems because it has
  checked-in provider synthetic, stress, backfill, backup/restore, and Qdrant rebuild
  evidence.
- The current comparison still undermeasures most competitor strengths. OpenViking
  trajectory, mem0/OpenMemory entity history and UI, Letta product export/readback
  for core-vs-archival memory, Graphiti/Zep temporal graph behavior, graph/RAG
  navigation, agentmemory and claude-mem continuity/capture breadth, and knowledge-page
  workflows remain non-claims.
  The separate XY-932 operator-debug live slice now scores ELF against qmd for trace
  hydration and candidate-drop visibility, but does not cover OpenMemory or
  claude-mem UI flows.

So the current adoption decision can remain "credible for bounded personal
production," but the competitiveness objective remains open.

## Fresh Runs

These commands were run in the current benchmark lanes after adapter-report
consistency repairs, the XY-927 core-vs-archival fixture update, the XY-928
OpenViking context-trajectory fixture update, and XY-933 live capture/write-policy
scoring:

| Command | Result | Runtime |
| --- | --- | ---: |
| `cargo make real-world-memory-core-archival` | pass | 12.14 seconds |
| `cargo make real-world-memory` | pass | 11.09 seconds |
| `cargo make real-world-memory-live-adapters` | pass | 137.66 seconds |

The live adapter run emitted repeated Qdrant client/server compatibility warnings, but
the command completed successfully and produced ELF and qmd JSON/Markdown reports.
Treat that warning as a measurement-harness risk to keep visible, not as a current run
failure.

## Fixture Aggregate

`cargo make real-world-memory` produced:

| Metric | Value |
| --- | ---: |
| Jobs | `49` |
| Encoded suites | `13` |
| Pass | `44` |
| Blocked | `5` |
| Wrong result | `0` |
| Lifecycle fail | `0` |
| Incomplete | `0` |
| Not encoded | `0` |
| Unsupported claim | `0` |
| Mean score | `0.898` |
| Mean latency | `3.940 ms` |
| Expected evidence recall | `100/100` |
| Evidence coverage | `111/111` |
| Source-ref coverage | `111/111` |
| Quote coverage | `111/111` |

This proves fixture contract breadth and scoring behavior. It does not prove every
live adapter or competitor runtime can complete those jobs.

## Live ELF/qmd Sweep

`cargo make real-world-memory-live-adapters` produced:

XY-934 update: the June 11 consolidation row below is superseded for ELF by
`docs/evidence/benchmarking/2026-06-16-live-consolidation-proposal-scoring-report.md`.
ELF now has live service-backed consolidation proposal scoring for the 4 checked-in
consolidation jobs; qmd remains typed `not_encoded` for this suite.

| Adapter | Jobs | Pass | Wrong result | Blocked | Not encoded | Mean score | Mean latency | Evidence recall | Evidence coverage |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ELF live service adapter | `40` | `22` | `5` | `2` | `11` | `0.599` | `6.980 ms` | `50/80` | `58/88` |
| qmd live CLI adapter | `40` | `17` | `6` | `2` | `15` | `0.461` | `792.543 ms` | `38/80` | `45/88` |

This supports an ELF lead in the current full live sweep count, but not a broad
ELF-over-qmd claim. The lead is concentrated in the ELF-only capture/write-policy
self-check plus the delete/TTL tombstone case. qmd remains the stronger retrieval-debug
UX reference, and its deep profile is still not encoded.

### Live Suite Breakdown

ELF and qmd have the same status shape outside `memory_evolution` and
`capture_integration`. The memory-evolution difference is
`memory-evolution-delete-ttl-001`: ELF passes that job while qmd reports
`wrong_result`, leaving ELF at five memory-evolution wrong results and qmd at six. The
capture difference is that ELF now executes the capture/write-policy jobs through its
service runtime, while qmd keeps those jobs typed `not_encoded`.

| Suite | Jobs | ELF breakdown | qmd breakdown |
| --- | ---: | --- | --- |
| `trust_source_of_truth` | `1` | `pass:1` | `pass:1` |
| `work_resume` | `5` | `pass:5` | `pass:5` |
| `retrieval` | `5` | `pass:5` | `pass:5` |
| `project_decisions` | `5` | `pass:5` | `pass:5` |
| `personalization` | `1` | `pass:1` | `pass:1` |
| `memory_evolution` | `6` | `pass:1`, `wrong_result:5` | `wrong_result:6` |
| `capture_integration` | `4` | `pass:4` | `not_encoded:4` |
| `consolidation` | `4` | `not_encoded:4` | `not_encoded:4` |
| `knowledge_compilation` | `2` | `not_encoded:2` | `not_encoded:2` |
| `operator_debugging_ux` | `1` | `not_encoded:1` | `not_encoded:1` |
| `production_ops` | `6` | `blocked:2`, `not_encoded:4` | `blocked:2`, `not_encoded:4` |

The ELF live wrong results are five memory-evolution jobs. qmd has those same conflict
evidence failures plus the delete/TTL tombstone miss. The live adapters retrieve
current evidence in several cases but do not yet provide the required historical
conflict evidence links for current-vs-historical reasoning.

## External Adapter Ledger

The checked-in manifest records 23 adapter records across 17 unique project names.

| Evidence class | Adapter records | Meaning |
| --- | ---: | --- |
| `fixture_backed` | `1` | ELF fixture scoring only. |
| `live_baseline_only` | `6` | Docker same-corpus or lifecycle evidence without real-world job scoring. |
| `live_real_world` | `5` | ELF and qmd live real-world sweeps, graphify's tiny scored Docker smoke, and the narrow ELF/qmd operator-debug live slice. |
| `research_gate` | `11` | Setup, source, resource, or output-contract gate only. |

| Overall status | Adapter records |
| --- | ---: |
| `pass` | `4` |
| `wrong_result` | `6` |
| `lifecycle_fail` | `1` |
| `blocked` | `7` |
| `not_encoded` | `5` |

The generated JSON report emits `external_project_count: 16`, matching the unique
non-ELF project-name count from the manifest. The companion audit JSON separately
records `unique_project_names: 17` for the full project list including ELF.

## Project Coverage

| Project | Best current evidence | Current measured state | Strongest unproven scenario | Next measurement before claim |
| --- | --- | --- | --- | --- |
| ELF | `fixture_backed` plus `live_real_world` | Fixture aggregate passes except 5 blocked operator or measurement-gate boundaries; live full sweep is `wrong_result`; live capture/write-policy, live consolidation proposal scoring, and narrow operator-debug slices pass. | Full live memory evolution, live knowledge pages, live production ops, competitor capture hooks, OpenViking staged trajectory artifacts, and broader operator UI runners. | Memory-evolution diagnostic report, then knowledge reports plus agentmemory/claude-mem capture, OpenViking staged trajectory artifacts, and OpenMemory/claude-mem UI runners. |
| qmd | `live_real_world` plus `live_baseline_only` | Fresh full sweep is five passes behind ELF because qmd misses the delete/TTL tombstone job and keeps capture/write-policy jobs typed `not_encoded`; same-corpus baseline passes; narrow operator-debug live slice ties replay commands but is `wrong_result` for trace hydration and candidate-drop visibility. | Deep retrieval-debug ergonomics and trace replay beyond the narrow operator-debug slice. | qmd/ELF deep retrieval-debug profile with expansion, fusion, rerank, and dropped-candidate traces. |
| agentmemory | `live_baseline_only` | `lifecycle_fail`; capture comparison is `blocked` because the Docker baseline uses a process-local StateKV Map and in-memory index, with no durable local session/capture path for source ids, exclusions, write-policy audit, or evidence-bound output. | Durable coding-agent continuity and capture hooks. | Durable lifecycle and work-resume/capture adapter report. |
| mem0/OpenMemory | `live_baseline_only` | Basic local smoke and local OSS history/readback pass; OpenMemory UI/export is blocked, hosted Platform export is a non-goal, and optional graph plus broader prompt coverage remain `not_encoded`. | Entity history, lifecycle UI, OpenMemory inspection. | Entity-history, deletion-audit, and UI/export readback report. |
| memsearch | `live_baseline_only`; XY-925 `fixture_backed` | Basic canonical Markdown reindex/reload smoke passes, and XY-925 adds fixture-backed source-store and retrieval-debug prompts without claiming a live memsearch adapter pass. | Markdown canonical store and local reindex clarity. | Runtime source-of-truth and retrieval-debug adapter execution over the existing prompt jobs. |
| OpenViking | `live_baseline_only` plus `fixture_backed` and `research_gate` | Same-corpus retrieval is `wrong_result`; staged retrieval, hierarchy selection, and recursive/context expansion are encoded as blocked fixtures. | Hierarchical staged context trajectory. | Evidence-bearing retrieval fix, then materialized staged trajectory report. |
| claude-mem | `live_baseline_only`; XY-925 `fixture_backed` | Same-corpus retrieval remains `wrong_result`; XY-925 adds fixture-backed progressive-disclosure and retrieval-repair prompts, with hook capture and viewer/operator workflows still blocked. | Progressive disclosure and automatic capture review. | Work-resume, operator-debugging, capture/write-policy, and viewer/operator runtime report. |
| RAGFlow | `research_gate` | `blocked`. | RAG app workflow with document/chunk references. | Tiny Docker evidence-smoke with `reference.chunks` mapped to evidence ids. |
| LightRAG | `research_gate` | `blocked`. | Graph/RAG context export with source-path citations. | Docker context-export report with explicit provider config and source citation mapping. |
| GraphRAG | `research_gate` | `blocked`. | Graph summaries and document/text-unit evidence tables. | Cost-bounded Docker adapter report over a tiny corpus. |
| Graphiti/Zep | `research_gate` | `blocked`. | Temporal graph facts and validity windows. | Docker-local temporal graph adapter report for current and historical facts. |
| Letta | `research_gate` | `blocked` for the selected contained export/readback path; scenario rows remain `not_tested` or `blocked`. | Core memory blocks versus archival memory. | Implement the Docker-only export/readback adapter before any Letta win/tie/loss claim. |
| LangGraph | `research_gate` | `not_encoded`; direct memory backend is unsupported. | Checkpoint replay and fork/regression debugging. | Treat as benchmark-infra reference unless a memory-output contract emerges. |
| nanograph | `research_gate` | `not_encoded`; full memory backend is unsupported. | Typed graph schema and query ergonomics. | Typed relation query report only if evidence ids can be emitted. |
| llm-wiki | `research_gate` | `not_encoded`. | Wiki/page generation, query-save, lint and repair loops. | Contained page-generation report with citation and unsupported-claim lint. |
| gbrain | `research_gate` | `not_encoded`; setup path is blocked. | Compiled truth pages, timelines, and brain navigation. | Docker-local brain repo setup proof, then compiled-truth/timeline report. |
| graphify | `live_real_world` | Tiny scored smoke is `wrong_result`. | Graph-compressed navigation with `graph.json` and `GRAPH_REPORT`. | Expand beyond the generated smoke only after graph/report output maps to scored evidence on representative graph/RAG jobs. |

## Scenario Coverage And Claims

| Scenario | Current measured position | Claim allowed today | Missing measurement |
| --- | --- | --- | --- |
| Retrieval/debug | ELF and qmd live retrieval pass; qmd same-corpus baseline passes. | Tie on encoded live retrieval; no ELF-over-qmd UX claim. | qmd/ELF deep trace replay and debug ergonomics scoring. |
| Work resume | ELF and qmd live pass. | ELF is credible on encoded work resume. | agentmemory, claude-mem, and OpenViking comparable continuity adapters. |
| Project decisions | ELF and qmd live pass; ELF fixture coverage also passes core routing plus archival rationale recovery. | ELF is credible on encoded project-decision recovery. | Letta core/archival decision memory export and scoring. |
| Source of truth | ELF and qmd live pass; ELF has stronger production restore/rebuild evidence. | ELF has strongest measured source-of-truth discipline. | memsearch source-of-truth reindex/reload evidence. |
| Memory evolution | ELF live fails 5/6 jobs; qmd live fails 6/6 jobs after missing the delete/TTL tombstone evidence; fixture aggregate passes. | No broad live superiority claim. | Historical conflict evidence links and Graphiti/Zep temporal comparison. |
| Consolidation | Fixture aggregate passes; XY-934 adds ELF live service-backed proposal scoring, while qmd remains `not_encoded`. | ELF self-check claim only; no direct competitor win. | Contained competitor/reference runners only when they emit source ids, confidence, unsupported-claim flags, and review-action audit. |
| Knowledge pages | Fixture aggregate passes; live adapters are not encoded. | Fixture-only claim. | Live page rebuild/lint plus llm-wiki, gbrain, GraphRAG, and graphify comparisons. |
| Operator debugging | Fixture aggregate passes; narrow ELF/qmd live operator-debug slice is scored with ELF `pass` and qmd `wrong_result`. | Narrow ELF/qmd live claim only: ELF wins trace hydration, candidate-drop visibility, and selected-but-not-narrated evidence; replay-command and repair-action clarity are tied. | OpenMemory and claude-mem UI/export or viewer runners before any broader operator-UX claim. |
| Capture/write policy | Fixture aggregate passes; ELF live service adapter passes 4/4 capture jobs with zero redaction leaks; qmd is `not_encoded`; agentmemory is `blocked`; claude-mem hook/viewer capture is `blocked`. | ELF has live self-check evidence for redaction, exclusions, source ids, evidence binding, and no secret leakage. Against agentmemory/claude-mem capture breadth, the comparison remains blocked until durable hook/viewer evidence exists. | Durable agentmemory and claude-mem capture-hook runners with evidence-bound output. |
| Production ops | ELF has separate production-provider/backfill/restore evidence; live sweep is not a full production-ops pass. | Bounded personal-production adoption claim with caveats. | Private corpus manifest and credentialed provider gates. |
| Personalization | ELF and qmd live pass one scoped preference job. | Narrow encoded pass only. | mem0/OpenMemory and Letta entity/preference history comparison. |
| Context trajectory | Not comparable. | No claim. | OpenViking staged hierarchy/trajectory scoring. |
| Core-vs-archival memory | ELF fixture suite passes 6/6; Letta comparison is blocked until export/readback evidence exists. | Fixture-only ELF core-block claim; no ELF-over-Letta claim. | Letta contained export/readback artifact with core block JSON, archival search/readback JSON, and source ids. |
| Graph/RAG navigation | RAGFlow, LightRAG, GraphRAG, and Graphiti/Zep remain typed research gates; graphify has a tiny scored `wrong_result` smoke. | No graph/RAG parity claim; only graphify's bounded non-pass smoke can be cited. | Larger contained RAG/graph adapters with evidence-linked outputs before any ELF graph/RAG win, tie, or loss claim. |

## Next Measurement Reports

Order these by decision value, not implementation convenience:

1. ELF/qmd retrieval-debug deep profile
   - Why: qmd is the closest measured live competitor and still stronger as a
     debugging reference.
   - Output: trace-level comparison of expansion, dense/sparse retrieval, fusion,
     rerank, dropped candidates, and command-line replay.

2. ELF/qmd live memory-evolution diagnostic
   - Why: ELF currently fails 5/6 live memory-evolution jobs and qmd fails 6/6,
     including the delete/TTL tombstone case.
   - Output: per-job evidence-link failure analysis for current-vs-historical facts,
     supersession, and relation temporal validity.

3. External capture-hook report for agentmemory and claude-mem
   - Why: ELF now has a live capture/write-policy self-check, but the strongest
     agentmemory and claude-mem capture-breadth claims are still blocked.
   - Output: durable local capture artifacts, source ids, redaction/exclusion audit,
     and typed blocker reasons when hooks or viewer capture cannot run in Docker.

4. Continuity and context-trajectory report
   - Why: agentmemory, claude-mem, and OpenViking represent real user expectations
     around automatic capture, progressive disclosure, and staged context.
   - Output: comparable work-resume/capture/trajectory jobs or typed blockers.

5. Personalization and core-memory report
   - Why: mem0/OpenMemory and Letta represent product expectations ELF should absorb
     before claiming better personalization or operating context.
   - Output: entity history, preference correction, UI/readback, core-vs-archival,
     and project-decision scoring.

6. Knowledge and graph/RAG report pack
   - Why: llm-wiki, gbrain, graphify, GraphRAG, LightRAG, RAGFlow, and Graphiti/Zep
     cover knowledge synthesis and graph navigation that ELF currently cannot claim.
   - Output: Docker-contained artifacts mapped to evidence ids, or typed setup and
     resource blockers.

Before publishing the next aggregate report, keep `external_project_count` aligned
with unique non-ELF project names so readers do not confuse project coverage with
adapter-record coverage.

## Fail Criteria

Use these criteria for future reports:

- `pass`: comparable scenario is encoded, run, and evidence-backed.
- `wrong_result`: the system ran but answered with wrong, stale, unsupported, or
  insufficiently evidenced memory.
- `not_encoded`: the runner does not yet exercise the scenario. This is not a win or
  loss.
- `blocked`: safe measurement needs missing credentials, private data, resource
  envelope acceptance, setup proof, or an export contract.
- `unsupported`: the project shape is not a direct memory-system comparison target.
- Fixture evidence cannot be promoted into live runtime evidence.
- Live baseline evidence cannot be promoted into real-world job evidence.
- Research-gate evidence cannot be promoted into pass/fail product quality evidence.

## Bottom Line

ELF is on a strong path because its benchmark methodology is stricter than a normal
leaderboard, and its production evidence is unusually concrete. The next work is not
to declare victory. The next work is to measure the strongest user-facing patterns in
adjacent projects, then decide which ones ELF should absorb behind fresh benchmark
gates.
