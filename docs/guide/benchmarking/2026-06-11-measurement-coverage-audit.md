# ELF Benchmark Measurement Coverage Audit - June 11, 2026

Goal: Record what is actually measured today, where competitor comparisons are still
not comparable, and which measurement reports should guide future ELF iteration.
Read this when: You need to answer whether ELF has enough empirical evidence to
claim a win, tie, loss, or non-claim against tracked memory, RAG, graph, and
agent-continuity projects.
Inputs: Fresh local runs of `cargo make real-world-memory` and
`cargo make real-world-memory-live-adapters` in the current XY-898 lane after
adapter-report consistency repairs, plus
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

- ELF has a strong fixture-backed real-world benchmark contract: 38 jobs, 36 pass,
  2 blocked operator boundaries, and no wrong results in the fixture aggregate.
- ELF and qmd have comparable full-suite live real-world sweeps. The latest generated
  artifacts are close but no longer identical: ELF has 38 jobs with 18 pass,
  5 wrong_result, 2 blocked, and 13 not_encoded, while qmd has 17 pass,
  6 wrong_result, 2 blocked, and 13 not_encoded.
- ELF is ahead on production-operation evidence among tracked systems because it has
  checked-in provider synthetic, stress, backfill, backup/restore, and Qdrant rebuild
  evidence.
- The current comparison still undermeasures most competitor strengths. OpenViking
  trajectory, mem0/OpenMemory entity history and UI, Letta core-vs-archival memory,
  Graphiti/Zep temporal graph behavior, graph/RAG navigation, agentmemory and
  claude-mem capture/continuity, and knowledge-page workflows remain non-claims.

So the current adoption decision can remain "credible for bounded personal
production," but the competitiveness objective remains open.

## Fresh Runs

These commands were run from an isolated report worktree based on `origin/main`:

| Command | Result | Runtime |
| --- | --- | ---: |
| `cargo make real-world-memory` | pass | 42.38 seconds |
| `cargo make real-world-memory-live-adapters` | pass | 121.93 seconds |

The live adapter run emitted repeated Qdrant client/server compatibility warnings, but
the command completed successfully and produced ELF and qmd JSON/Markdown reports.
Treat that warning as a measurement-harness risk to keep visible, not as a current run
failure.

## Fixture Aggregate

`cargo make real-world-memory` produced:

| Metric | Value |
| --- | ---: |
| Jobs | `38` |
| Encoded suites | `11` |
| Pass | `36` |
| Blocked | `2` |
| Wrong result | `0` |
| Lifecycle fail | `0` |
| Incomplete | `0` |
| Not encoded | `0` |
| Unsupported claim | `0` |
| Mean score | `0.947` |
| Mean latency | `4.411 ms` |
| Expected evidence recall | `77/77` |
| Evidence coverage | `84/84` |
| Source-ref coverage | `84/84` |
| Quote coverage | `84/84` |

This proves fixture contract breadth and scoring behavior. It does not prove every
live adapter or competitor runtime can complete those jobs.

## Live ELF/qmd Sweep

`cargo make real-world-memory-live-adapters` produced:

| Adapter | Jobs | Pass | Wrong result | Blocked | Not encoded | Mean score | Mean latency | Evidence recall | Evidence coverage |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ELF live service adapter | `38` | `18` | `5` | `2` | `13` | `0.525` | `6.823 ms` | `41/77` | `48/84` |
| qmd live CLI adapter | `38` | `17` | `6` | `2` | `13` | `0.486` | `819.626 ms` | `38/77` | `45/84` |

This supports a narrow tie on the currently encoded live real-world suite shape. It
does not support a broad ELF-over-qmd claim because qmd remains the stronger
retrieval-debug UX reference and its deep profile is still not encoded.

### Live Suite Breakdown

ELF and qmd had the same suite status shape:

| Suite | Jobs | Status breakdown |
| --- | ---: | --- |
| `trust_source_of_truth` | `1` | `pass:1` |
| `work_resume` | `5` | `pass:5` |
| `retrieval` | `5` | `pass:5` |
| `project_decisions` | `5` | `pass:5` |
| `personalization` | `1` | `pass:1` |
| `memory_evolution` | `6` | `pass:1`, `wrong_result:5` |
| `capture_integration` | `2` | `not_encoded:2` |
| `consolidation` | `4` | `not_encoded:4` |
| `knowledge_compilation` | `2` | `not_encoded:2` |
| `operator_debugging_ux` | `1` | `not_encoded:1` |
| `production_ops` | `6` | `blocked:2`, `not_encoded:4` |

The five live wrong results are all memory-evolution jobs. The live adapters retrieve
current evidence but do not yet provide the required historical conflict evidence
links for current-vs-historical reasoning.

## External Adapter Ledger

The checked-in manifest records 21 adapter records across 17 unique project names.

| Evidence class | Adapter records | Meaning |
| --- | ---: | --- |
| `fixture_backed` | `1` | ELF fixture scoring only. |
| `live_baseline_only` | `6` | Docker same-corpus or lifecycle evidence without real-world job scoring. |
| `live_real_world` | `2` | ELF and qmd live real-world sweeps. |
| `research_gate` | `12` | Setup, source, resource, or output-contract gate only. |

| Overall status | Adapter records |
| --- | ---: |
| `pass` | `3` |
| `wrong_result` | `4` |
| `lifecycle_fail` | `1` |
| `blocked` | `6` |
| `not_encoded` | `7` |

The generated JSON report now emits `external_project_count` as the distinct non-ELF
project-name count. The manifest still has 21 adapter records across 17 unique project
names, of which 16 are external projects.

## Project Coverage

| Project | Best current evidence | Current measured state | Strongest unproven scenario | Next measurement before claim |
| --- | --- | --- | --- | --- |
| ELF | `fixture_backed` plus `live_real_world` | Fixture aggregate passes except 2 blocked operator boundaries; live full sweep is `wrong_result`. | Full live memory evolution, live consolidation, live knowledge pages, live capture, live production ops. | Memory-evolution diagnostic report, then live operator/capture/consolidation reports. |
| qmd | `live_real_world` plus `live_baseline_only` | Same live sweep shape as ELF; same-corpus baseline passes. | Deep retrieval-debug ergonomics and trace replay. | qmd/ELF deep retrieval-debug profile with expansion, fusion, rerank, and dropped-candidate traces. |
| agentmemory | `live_baseline_only` | `lifecycle_fail`. | Durable coding-agent continuity and capture hooks. | Durable lifecycle and work-resume/capture adapter report. |
| mem0/OpenMemory | `live_baseline_only` | Basic local smoke now passes; history/UI/hosted/graph behavior remains `not_encoded`. | Entity history, lifecycle UI, OpenMemory inspection. | Entity-history, deletion-audit, and UI/export readback report. |
| memsearch | `live_baseline_only` | Basic canonical Markdown reindex/reload smoke now passes; real-world prompt coverage remains `not_encoded`. | Markdown canonical store and local reindex clarity. | Source-of-truth and retrieval-debug real-world adapter report. |
| OpenViking | `live_baseline_only` plus `research_gate` | Same-corpus retrieval is `wrong_result`; trajectory is `not_encoded`. | Hierarchical staged context trajectory. | Evidence-bearing retrieval fix, then staged trajectory report. |
| claude-mem | `live_baseline_only` | `wrong_result`. | Progressive disclosure and automatic capture review. | Work-resume, operator-debugging, and capture/write-policy report. |
| RAGFlow | `research_gate` | `blocked`. | RAG app workflow with document/chunk references. | Tiny Docker evidence-smoke with `reference.chunks` mapped to evidence ids. |
| LightRAG | `research_gate` | `blocked`. | Graph/RAG context export with source-path citations. | Docker context-export report with explicit provider config and source citation mapping. |
| GraphRAG | `research_gate` | `blocked`. | Graph summaries and document/text-unit evidence tables. | Cost-bounded Docker adapter report over a tiny corpus. |
| Graphiti/Zep | `research_gate` | `blocked`. | Temporal graph facts and validity windows. | Docker-local temporal graph adapter report for current and historical facts. |
| Letta | `research_gate` | `not_encoded`. | Core memory blocks versus archival memory. | Contained export contract, then core-vs-archival and decision-memory report. |
| LangGraph | `research_gate` | `not_encoded`; direct memory backend is unsupported. | Checkpoint replay and fork/regression debugging. | Treat as benchmark-infra reference unless a memory-output contract emerges. |
| nanograph | `research_gate` | `not_encoded`; full memory backend is unsupported. | Typed graph schema and query ergonomics. | Typed relation query report only if evidence ids can be emitted. |
| llm-wiki | `research_gate` | `not_encoded`. | Wiki/page generation, query-save, lint and repair loops. | Contained page-generation report with citation and unsupported-claim lint. |
| gbrain | `research_gate` | `not_encoded`; setup path is blocked. | Compiled truth pages, timelines, and brain navigation. | Docker-local brain repo setup proof, then compiled-truth/timeline report. |
| graphify | `research_gate` | `blocked`. | Graph-compressed navigation with `graph.json` and `GRAPH_REPORT`. | Docker graph/report output report mapped to benchmark evidence ids. |

## Scenario Coverage And Claims

| Scenario | Current measured position | Claim allowed today | Missing measurement |
| --- | --- | --- | --- |
| Retrieval/debug | ELF and qmd live retrieval pass; qmd same-corpus baseline passes. | Tie on encoded live retrieval; no ELF-over-qmd UX claim. | qmd/ELF deep trace replay and debug ergonomics scoring. |
| Work resume | ELF and qmd live pass. | ELF is credible on encoded work resume. | agentmemory, claude-mem, and OpenViking comparable continuity adapters. |
| Project decisions | ELF and qmd live pass. | ELF is credible on encoded project-decision recovery. | Letta core/archival decision memory comparison. |
| Source of truth | ELF and qmd live pass; ELF has stronger production restore/rebuild evidence. | ELF has strongest measured source-of-truth discipline. | memsearch source-of-truth reindex/reload evidence. |
| Memory evolution | ELF and qmd live fail 5/6 jobs; fixture aggregate passes. | No live superiority claim. | Historical conflict evidence links and Graphiti/Zep temporal comparison. |
| Consolidation | Fixture aggregate passes; live adapters are not encoded. | Fixture-only claim. | Live proposal generation with lineage, confidence, and review-action audit. |
| Knowledge pages | Fixture aggregate passes; live adapters are not encoded. | Fixture-only claim. | Live page rebuild/lint plus llm-wiki, gbrain, GraphRAG, and graphify comparisons. |
| Operator debugging | Fixture aggregate passes; live adapters are not encoded. | Fixture-only claim. | Trace hydration, stage attribution, dropped-candidate, and repair-action scoring. |
| Capture/write policy | Fixture aggregate passes; live adapters are not encoded. | Fixture-only claim. | agentmemory/claude-mem style capture with redaction and evidence binding. |
| Production ops | ELF has separate production-provider/backfill/restore evidence; live sweep is not a full production-ops pass. | Bounded personal-production adoption claim with caveats. | Private corpus manifest and credentialed provider gates. |
| Personalization | ELF and qmd live pass one scoped preference job. | Narrow encoded pass only. | mem0/OpenMemory and Letta entity/preference history comparison. |
| Context trajectory | Not comparable. | No claim. | OpenViking staged hierarchy/trajectory scoring. |
| Core-vs-archival memory | Not comparable. | No claim. | Letta contained export and ELF core-block benchmark. |
| Graph/RAG navigation | Research gates and blocked adapters only. | No claim. | RAGFlow, LightRAG, GraphRAG, Graphiti/Zep, and graphify Docker reports. |

## Next Measurement Reports

Order these by decision value, not implementation convenience:

1. ELF/qmd retrieval-debug deep profile
   - Why: qmd is the closest measured live competitor and still stronger as a
     debugging reference.
   - Output: trace-level comparison of expansion, dense/sparse retrieval, fusion,
     rerank, dropped candidates, and command-line replay.

2. ELF/qmd live memory-evolution diagnostic
   - Why: both systems currently fail 5/6 live memory-evolution jobs.
   - Output: per-job evidence-link failure analysis for current-vs-historical facts,
     supersession, and relation temporal validity.

3. Live operator-debugging and capture/write-policy report
   - Why: these are daily-use agent-memory qualities, currently fixture-only or
     not_encoded in live sweeps.
   - Output: trace hydration, raw-SQL avoidance, redaction, exclusion, write-policy,
     and repair-action scoring.

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

Keep the generated `external_project_count` field aligned with unique non-ELF project
names so readers do not confuse adapter records with project coverage.

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
