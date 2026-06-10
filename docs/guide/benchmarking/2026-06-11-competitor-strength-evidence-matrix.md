# Competitor-Strength Evidence Matrix - June 11, 2026

Goal: Define a durable competitor-strength matrix so ELF benchmark claims are tied to
measured evidence classes, typed blockers, and explicit next measurement gates.
Read this when: You need to decide whether ELF can claim a win, tie, loss, gap, or
non-claim against a tracked memory, RAG, or graph project.
Inputs: `docs/guide/benchmarking/2026-06-10-production-adoption-refresh.md`,
`docs/guide/benchmarking/2026-06-10-real-world-comparison-report.md`,
`docs/guide/benchmarking/2026-06-10-live-real-world-sweep-report.md`,
`docs/guide/research/external_memory_improvement_plan.md`,
`docs/guide/research/research_projects_inventory.md`,
`apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`,
and `Makefile.toml`.
Depends on: `docs/spec/real_world_agent_memory_benchmark_v1.md`,
`docs/guide/benchmarking/live_baseline_benchmark.md`, and the current external adapter
manifest.
Outputs: Human-readable matrix, claim boundaries, scenario next-measurement gates,
and the machine-readable companion file
`docs/research/2026-06-11-xy-897-competitor-strength-matrix.json`.

## Decision Boundary

Do not claim that ELF beats, ties, or loses to a competitor unless the named scenario
is encoded and run at a comparable evidence class.

Current boundary:

- ELF and qmd have full-suite `live_real_world` sweeps, but neither has a full-suite
  live pass. Each sweep produced 38 jobs with 18 pass, 5 wrong_result, 1 incomplete,
  2 blocked, and 12 not_encoded.
- ELF fixture evidence is strong: `cargo make real-world-memory` reports 38 jobs
  across 11 suites with 36 pass and 2 blocked production-ops operator boundaries.
  That proves the fixture contract, not live-service parity.
- qmd is the strongest measured local retrieval-debug comparison, but the current
  evidence still separates its same-corpus/live-retrieval strengths from the full-suite
  live non-pass sweep.
- Most other projects are `live_baseline_only` or `research_gate`. They must not be
  treated as beaten until a comparable scenario is encoded and run.
- Private-corpus and credentialed production-ops checks remain operator-owned
  `blocked` states.

## Current Ledger Summary

The current manifest has 21 adapter records across 17 projects. Evidence-class counts:
1 `fixture_backed`, 6 `live_baseline_only`, 2 `live_real_world`, and 12
`research_gate`. Overall adapter-status counts: 1 `pass`, 6 `wrong_result`, 1
`lifecycle_fail`, 6 `blocked`, and 7 `not_encoded`.

## State Taxonomy

This report uses the benchmark's snake_case state names. Hyphenated prose names map
directly to these states: fixture-backed -> `fixture_backed`,
live-baseline -> `live_baseline_only`, live-real-world -> `live_real_world`,
research-gate -> `research_gate`, wrong-result -> `wrong_result`,
lifecycle-fail -> `lifecycle_fail`, and not-encoded -> `not_encoded`.

| State | Meaning | Claim boundary |
| --- | --- | --- |
| `fixture_backed` | Checked-in real-world jobs or fixture responses are scored by the benchmark runner. | Useful for contract coverage, not live runtime proof. |
| `live_baseline_only` | Docker same-corpus or lifecycle checks ran, but no real-world job suite was scored for that project. | Cannot imply real-world job parity. |
| `live_real_world` | A runtime or CLI adapter materialized and scored real-world job records. | Can support scenario claims only for the encoded suite statuses. |
| `research_gate` | Source, setup, resource, retry, or output-contract metadata exists. | Follow-up routing only; not pass evidence. |
| `blocked` | Safe measurement needs unavailable credentials, private data, setup proof, or external dependency. | Keep typed until the missing input exists. |
| `unsupported` | Capability is outside the project shape or requires a non-comparable path. | Do not turn into a loss. |
| `wrong_result` | The system ran but missed expected memory, answer, or evidence terms. | Behavioral non-pass. |
| `lifecycle_fail` | Retrieval may work, but update/delete/reload/persistence/cold-start behavior fails. | Lifecycle non-pass, not a retrieval win. |
| `incomplete` | The run did not reach the behavioral check because setup or runtime failed. | Setup/runtime non-pass, not quality evidence. |
| `not_encoded` | The scenario is not currently covered. | No pass/fail claim is allowed. |

## Project Matrix

| Project | Strongest user-facing scenario | Current evidence | Measured status and proof | Unsupported or blocked status | Required benchmark before ELF claim | Borrow if stronger |
| --- | --- | --- | --- | --- | --- | --- |
| ELF | Evidence-linked source-of-truth memory service with real-world fixtures and live retrieval sweeps. | `live_real_world`; supporting `fixture_backed`. | `wrong_result` full live sweep: `cargo make real-world-memory-live-adapters`, `tmp/real-world-memory/live-adapters/elf-report.md`. Fixture contract: `cargo make real-world-memory`, `tmp/real-world-memory/real-world-memory-report.json`. | `blocked`: private manifest and provider credentials; broader live suites remain `wrong_result`, `incomplete`, or `not_encoded`. | Full-suite live pass plus separate private-corpus and credentialed production-ops proof. | Keep borrowing qmd debug knobs, OpenViking staged trajectory, mem0 history, Letta core memory, and graph/RAG navigation. |
| qmd | Local retrieval-debug workflow with transparent CLI indexing, querying, expansion, fusion, and rerank ergonomics. | `live_real_world`; supporting `live_baseline_only` and `research_gate`. | `wrong_result` full live sweep: `cargo make real-world-memory-live-adapters`, `tmp/real-world-memory/live-adapters/qmd-report.md`; targeted retrieval suites pass. | `not_encoded`: deep profile and non-retrieval live behavior are not encoded; memory_evolution is `wrong_result`. | qmd deep retrieval/debug profile plus full-suite live replay with trace-level diagnostics. | Weighted fusion, rerank explanation, local debug knobs, and command-line replay. |
| agentmemory | Coding-agent continuity, MCP/REST packaging, viewer workflow, and durable cross-agent memory lifecycle. | `live_baseline_only`. | `lifecycle_fail`: `ELF_BASELINE_PROJECTS=agentmemory cargo make baseline-live-docker`, `tmp/live-baseline/live-baseline-report.json`. | `blocked`: durable cold-start and real-world adapter coverage are missing. | Durable local adapter with update, delete, cold-start reload, work_resume, capture/write-policy, and lifecycle-staleness jobs. | Cross-agent hooks, packaging, continuity scenarios, and viewer affordances. |
| mem0/OpenMemory | Memory lifecycle, personalization, hosted/OpenMemory UI ergonomics, and optional graph memory. | `live_baseline_only`. | `wrong_result`: `ELF_BASELINE_PROJECTS=mem0 cargo make baseline-live-docker`, `tmp/live-baseline/live-baseline-report.json`. | `not_encoded`: OpenMemory UI, hosted claims, and real-world personalization coverage are not encoded. | Fix local same-corpus result, then encode memory_evolution, personalization, UI readback, and optional graph-context jobs. | Entity-scoped history, lifecycle surfaces, async update ergonomics, and OpenMemory inspection UX. |
| memsearch | Markdown-first canonical store with rebuildable local index and practical hybrid retrieval. | `live_baseline_only`. | `wrong_result`: `ELF_BASELINE_PROJECTS=memsearch cargo make baseline-live-docker`, `tmp/live-baseline/live-baseline-report.json`. | `incomplete`: source-of-truth and real-world reindex behavior are not cleanly scored. | Fix Docker same-corpus retrieval and reindex/update/delete reload evidence, then score source-of-truth and retrieval-debug jobs. | Canonical markdown store, local reindex clarity, and user-inspectable source files. |
| OpenViking | Filesystem-like context trajectory, hierarchical retrieval, and staged context loading. | `live_baseline_only`; supporting `research_gate`. | `wrong_result`: `ELF_BASELINE_PROJECTS=OpenViking cargo make baseline-live-docker`, `tmp/live-baseline/live-baseline-report.json`. | `not_encoded`: hierarchical context trajectory is not encoded; same-corpus output still misses expected evidence. | Make evidence-bearing same-corpus output pass, then score staged trajectory and hierarchy expansion. | `viking://`-style context model, trajectory readback, and staged retrieval planning. |
| claude-mem | Progressive disclosure, automatic capture loop, repository-local lifecycle, and local viewer workflow. | `live_baseline_only`. | `wrong_result`: `ELF_BASELINE_PROJECTS=claude-mem cargo make baseline-live-docker`, `tmp/live-baseline/live-baseline-report.json`. | `not_encoded`: progressive-disclosure real-world jobs are not encoded. | Durable repository-backed work_resume, operator_debugging_ux, capture/write-policy, and progressive-disclosure jobs. | Progressive disclosure, automatic capture review loops, and local viewer/operator comfort. |
| RAGFlow | Full RAG application workflow with document, chunk, and reference evidence handles. | `research_gate`. | `blocked`: `ELF_RAGFLOW_SMOKE_START=1 ELF_RAGFLOW_SMOKE_ACCEPT_RESOURCE_ENVELOPE=1 cargo make ragflow-docker-smoke`, `tmp/real-world-memory/ragflow-smoke/ragflow-smoke.json`. | `blocked`: Docker resource envelope and adapter output mapping still need proof. | XY-885 tiny Docker evidence-smoke adapter mapping `reference.chunks` to scored evidence. | Document/chunk references, resource-envelope reporting, and RAG app evidence handles. |
| LightRAG | Lightweight graph/RAG context export with source file-path citation shape. | `research_gate`. | `blocked`: `ELF_LIGHTRAG_CONTEXT_START=1 cargo make lightrag-docker-context-smoke`, `tmp/real-world-memory/lightrag-context/summary.json`. | `blocked`: Docker service setup and context export are not proven. | XY-886 Docker context-export adapter with explicit provider config and source citation mapping. | Context-only query modes, graph-aware retrieval layout, and file-path citation readback. |
| GraphRAG | GraphRAG indexing, graph summaries, and document/text-unit evidence tables. | `research_gate`. | `blocked`: `ELF_GRAPHRAG_SMOKE_RUN=1 cargo make graphrag-docker-smoke`, `tmp/real-world-memory/graphrag-smoke/summary.json`. | `blocked`: indexing resource envelope and source citation mapping are not proven. | XY-887 cost-bounded Docker adapter over a tiny corpus and scored output tables. | Graph summary artifacts, local/global search separation, and source table evidence mapping. |
| Graphiti/Zep | Temporal graph memory with current, historical, and future fact validity windows. | `research_gate`. | `blocked`: `ELF_GRAPHITI_ZEP_SMOKE_START=1 ELF_GRAPHITI_ZEP_SMOKE_RUN=1 cargo make graphiti-zep-docker-temporal-smoke`, `tmp/real-world-memory/graphiti-zep-smoke/summary.json`. | `blocked`: Docker graph-store and temporal adapter are not proven. | XY-888 Docker-local temporal graph adapter scoring current/historical fact validity. | Temporal fact windows, invalidation/supersession semantics, and graph fact provenance. |
| Letta | Core memory blocks versus archival memory with explicit operating-context surfaces. | `research_gate`. | `not_encoded`: `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json`. | `blocked`: contained evidence export path is not selected. | Select contained export contract, then encode core-vs-archival, personalization, and project-decision jobs. | Core memory block ergonomics, archival separation, and shared operating context readback. |
| LangGraph | Checkpoint/replay regression workflow and durable state replay for agent runs. | `research_gate`. | `not_encoded`: `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json`. | `unsupported`: not a standalone memory backend adapter. | Non-goal for direct win/loss until a standalone memory output contract exists; use replay jobs as benchmark infrastructure reference. | Checkpoint replay, deterministic regression, and state-diff evaluation patterns. |
| nanograph | Typed graph schema and query ergonomics for graph-lite developer experience. | `research_gate`. | `not_encoded`: `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json`. | `unsupported`: not a memory backend comparison target. | Non-goal for direct win/loss unless a contained memory-backed target emerges; measure ELF graph-lite DX instead. | Typed relation schema, query ergonomics, and small graph developer experience. |
| llm-wiki | LLM-maintained wiki or knowledge-page workflow with query-save and lint loops. | `research_gate`. | `not_encoded`: `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json`. | `unsupported`: no live service runtime for adapter proof. | Select contained plugin or instruction harness, then score knowledge pages for citations, unsupported claims, rebuild, and stale-source lint. | Maintained wiki workflows, page lint, query-save loops, and topic-scoped navigation. |
| gbrain | Operational knowledge brain with compiled_truth pages, timelines, enrichment, and maintenance loops. | `research_gate`. | `not_encoded`: `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json`. | `blocked`: Docker-local brain repo and database path are missing. | Prove Docker-local repository/database setup, then encode compiled_truth/timeline and operator-continuity jobs. | Compiled truth pages, timeline maintenance, and human-operable knowledge-brain navigation. |
| graphify | Graph-compressed navigation with `graph.json` and `GRAPH_REPORT` evidence outputs. | `research_gate`. | `blocked`: `cargo make graphify-docker-graph-report-smoke`, `tmp/real-world-memory/graphify-smoke/graphify-smoke.json`. | `blocked`: Docker CLI graph/report generation is not proven; host-global assistant hooks are out of scope. | XY-889 Docker-only graph/report adapter over `graph.json` and `GRAPH_REPORT.md`. | Graph compression, source-location graph reports, and navigation hints for large code or document spaces. |

## Scenario Matrix

| Scenario | Current ELF evidence | Strongest competitor/reference | Current competitor evidence | Next measurement before claim |
| --- | --- | --- | --- | --- |
| Retrieval/debug | Fixture retrieval passes; live retrieval passes. | qmd. | qmd live retrieval passes and live baseline passes, but full-suite live status is `wrong_result`. | Run qmd deep profile and ELF/qmd trace-level replay with expansion, fusion, rerank, and candidate-drop diagnostics. |
| Work resume | Fixture and live work_resume pass. | agentmemory, claude-mem, OpenViking. | agentmemory `lifecycle_fail`, claude-mem `wrong_result`, OpenViking work_resume `not_encoded`. | Encode durable work_resume adapters or keep each blocked with lifecycle/setup evidence. |
| Project decisions | Fixture and live project_decisions pass. | qmd, Letta. | qmd live project_decisions pass; Letta is `research_gate` `not_encoded`. | Add Letta core/archival decision jobs only after a contained export path exists. |
| Source-of-truth | Fixture and live trust_source_of_truth pass. | memsearch. | memsearch canonical-store evidence exists, but source-of-truth is `incomplete` and retrieval is `wrong_result`. | Fix memsearch reindex/retrieval evidence and score source-of-truth rebuild/reload jobs. |
| Temporal/current-vs-historical memory | Fixture memory_evolution passes; live memory_evolution is `wrong_result`. | Graphiti/Zep, mem0/OpenMemory. | Graphiti/Zep is `research_gate` `blocked`; mem0/OpenMemory is `wrong_result`. | Fix ELF/qmd live memory_evolution evidence links and run XY-888. |
| Consolidation | Fixture consolidation passes; live consolidation is `not_encoded`. | agentmemory, managed-memory references, llm-wiki. | No manifest project has live consolidation scoring. | Run reviewable consolidation proposal generation with source refs, unsupported-claim flags, and audit transitions. |
| Knowledge pages | Fixture knowledge_compilation passes; live knowledge_compilation is `not_encoded`. | llm-wiki, gbrain, GraphRAG, graphify. | llm-wiki and gbrain are `research_gate` `not_encoded` or `blocked`; GraphRAG and graphify are `blocked`. | Encode live derived-page rebuild/lint scoring and run contained knowledge/RAG adapters only after setup proof. |
| Operator debugging | Fixture operator_debugging_ux passes; live operator_debugging_ux is `not_encoded`. | qmd, claude-mem, OpenMemory. | qmd has debug strengths but operator_debugging_ux is `not_encoded`; claude-mem and OpenMemory UX are `not_encoded`. | Score trace hydration, stage attribution, raw-SQL avoidance, and repair-action clarity through live artifacts. |
| Capture/write policy | Fixture capture_integration passes; live capture_integration is `not_encoded`. | agentmemory, claude-mem. | agentmemory capture is `blocked`; claude-mem capture is `not_encoded`. | Run live capture/write-policy jobs proving redaction, exclusion, evidence binding, and no secret leakage. |
| Production ops | Fixture production_ops has 4 pass and 2 blocked; live production_ops is `incomplete`; production adoption has provider/backfill/restore evidence. | ELF production gate, qmd, RAG/RAGFlow resource gates. | qmd live production_ops is `incomplete`; RAG/resource gates are `research_gate` `blocked`. | Rerun private-corpus and credentialed gates only when operator-owned manifest and credentials exist. |
| Personalization | Fixture and live personalization pass. | mem0/OpenMemory, Letta. | mem0/OpenMemory and Letta personalization are `not_encoded`. | Encode scoped preference readback for mem0/OpenMemory and Letta before personalization superiority claims. |
| Context trajectory | ELF has trace direction but no comparable staged trajectory scenario. | OpenViking. | OpenViking setup is pinned, same-corpus retrieval is `wrong_result`, and hierarchy trajectory is `not_encoded`. | Make OpenViking evidence-bearing retrieval pass, then score staged context trajectory outputs. |
| Core-vs-archival memory | ELF core-block semantics exist in the service contract, but comparative benchmark coverage is not encoded here. | Letta. | Letta is `research_gate` `not_encoded` until contained export proof exists. | Add ELF core-block versus archival-search jobs; compare Letta only after contained export proof. |
| Graph/RAG navigation | ELF relation context is not enough to claim graph/RAG navigation parity. | RAGFlow, LightRAG, GraphRAG, Graphiti/Zep, graphify. | All named RAG/graph projects are `research_gate` `blocked` or `not_encoded`. | Run XY-885 through XY-889 Docker-contained adapters with evidence-linked outputs. |

## Parallelizable Benchmark Follow-Ups

These workstreams can proceed after this matrix lands because the claim boundaries are
now explicit:

| Workstream | Issue or candidate | Parallelizable | Blocked by | Measurement |
| --- | --- | --- | --- | --- |
| qmd deep retrieval/debug profile | New benchmark issue | yes | None after this matrix lands. | Stress profile plus trace-level retrieval-debug artifacts for qmd and ELF. |
| agentmemory durable lifecycle adapter | `[ELF benchmark P0] Make external adapters lifecycle-durable and fail-typed` | yes | Durable local adapter path selection. | Update, delete, cold-start reload, work_resume, and capture/write-policy jobs. |
| mem0/OpenMemory local and UI coverage | New adapter repair issue | yes | Comparable local OSS path for UI/readback evidence. | Same-corpus fix plus memory_evolution, personalization, and OpenMemory inspection jobs. |
| memsearch source-of-truth and reindex coverage | New adapter repair issue | yes | Docker same-corpus retrieval and reindex correctness. | Canonical markdown store, rebuild/reindex, retrieval, update/delete/reload jobs. |
| OpenViking context trajectory | New benchmark issue after evidence output fix | yes | Evidence-bearing same-corpus retrieval output. | Hierarchical expansion, staged trajectory, and resume/retrieval evidence jobs. |
| claude-mem progressive disclosure | New adapter issue | yes | Durable repository path and progressive-disclosure output contract. | Work resume, operator debugging, capture/write-policy, and progressive disclosure jobs. |
| RAGFlow evidence smoke | XY-885 | yes | Resource envelope accepted for tiny Docker smoke. | `reference.chunks` to benchmark evidence mapping. |
| LightRAG context export | XY-886 | yes | Docker service setup and explicit provider config. | Retrieved context export and source file-path citations. |
| GraphRAG cost-bounded adapter | XY-887 | yes | Tiny corpus cost/resource envelope. | Document, text-unit, graph-summary, and citation output tables. |
| Graphiti/Zep temporal graph adapter | XY-888 | yes | Docker-local graph store setup. | Current/historical/future fact validity and evidence ids. |
| graphify graph report adapter | XY-889 | yes | Docker CLI graph/report generation proof. | `graph.json` and `GRAPH_REPORT` evidence for graph navigation and knowledge synthesis. |
| Private corpus and credentialed production ops | Operator-owned benchmark gates | no | Sanitized private manifest and routed provider credentials. | Private-corpus retrieval quality and credentialed production-ops evidence. |
| Letta, LangGraph, nanograph, llm-wiki direct adapters | Research-only until output contract | no | Contained evidence export or non-memory-backend comparability contract. | Run only after each has a comparable output contract; otherwise keep as product-reference evidence. |

## Validation Contract

Consistency checks for this report should verify:

- The Markdown project matrix includes every project currently present in
  `memory_projects_manifest.json`: ELF, qmd, agentmemory, mem0/OpenMemory, memsearch,
  OpenViking, claude-mem, RAGFlow, LightRAG, GraphRAG, Graphiti/Zep, Letta, LangGraph,
  nanograph, llm-wiki, gbrain, and graphify.
- The machine-readable matrix has the same project set and includes every required
  scenario id: `retrieval_debug`, `work_resume`, `project_decisions`,
  `source_of_truth`, `temporal_current_historical`, `consolidation`,
  `knowledge_pages`, `operator_debugging`, `capture_write_policy`, `production_ops`,
  `personalization`, `context_trajectory`, `core_vs_archival_memory`, and
  `graph_rag_navigation`.
- Evidence states remain typed. Do not collapse `research_gate`, `blocked`,
  `unsupported`, `wrong_result`, `lifecycle_fail`, `incomplete`, or `not_encoded`
  into pass/fail aggregates.

## Claim Rules

- A project can be called stronger only for a named scenario with comparable measured
  evidence.
- `research_gate` plus setup metadata can justify a follow-up adapter issue, not a
  product win.
- A blocked measurement is not a hidden loss. Keep the typed reason and rerun only when
  the missing operator or setup input exists.
- If a project remains stronger on user-facing workflow but lacks comparable measured
  evidence, record what ELF should borrow and add a benchmark gate before changing any
  README-level claim.
