# Competitor-Strength Evidence Matrix - June 11, 2026

Goal: Define a durable competitor-strength matrix so ELF benchmark claims are tied to
measured evidence classes, typed blockers, and explicit next measurement gates.
Read this when: You need to decide whether ELF can claim a win, tie, loss, gap, or
non-claim against a tracked memory, RAG, or graph project.
Inputs: `docs/guide/benchmarking/2026-06-10-production-adoption-refresh.md`,
`docs/guide/benchmarking/2026-06-10-real-world-comparison-report.md`,
`docs/guide/benchmarking/2026-06-10-live-real-world-sweep-report.md`,
`docs/guide/benchmarking/2026-06-11-measurement-coverage-audit.md`,
`docs/guide/benchmarking/2026-06-11-competitor-strength-adoption-report.md`,
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
  live pass. The fresh ELF sweep produced 40 jobs with 22 pass, 5 wrong_result,
  0 incomplete, 2 blocked, and 11 not_encoded; the fresh qmd sweep produced 17 pass,
  6 wrong_result, 0 incomplete, 2 blocked, and 15 not_encoded.
- ELF fixture evidence is strong: `cargo make real-world-memory` reports 55 jobs
  across 15 suites with 49 pass and 6 blocked production-ops, private-corpus, or
  OpenViking context-trajectory measurement gates. The `core_archival_memory` suite
  contributes 6 fixture-only passes for ELF core-block behavior; it does not create
  an ELF-over-Letta claim. The `memory_summary` suite contributes one fixture-backed
  source-trace pass; it does not create managed-memory parity evidence. The
  `proactive_brief` suite contributes four fixture-backed source-linked suggestion
  passes and one private-corpus blocker; it does not create Pulse or hosted
  managed-memory parity. This proves the fixture contract, not live-service parity.
- qmd is the strongest measured local retrieval-debug comparison, but the current
  evidence still separates its same-corpus/live-retrieval strengths from the full-suite
  live non-pass sweep.
- Most other projects are `live_baseline_only` or `research_gate`. They must not be
  treated as beaten until a comparable scenario is encoded and run.
- Private-corpus and credentialed production-ops checks remain operator-owned
  `blocked` states.

## Current Ledger Summary

The current manifest has 23 adapter records across 16 external projects plus ELF.
Evidence-class counts: 1 `fixture_backed`, 6 `live_baseline_only`, 5
`live_real_world`, and 11 `research_gate`. Overall adapter-status counts: 4 `pass`,
6 `wrong_result`, 1 `lifecycle_fail`, 7 `blocked`, and 5 `not_encoded`.

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
| ELF | Evidence-linked source-of-truth memory service with real-world fixtures and live retrieval sweeps. | `live_real_world`; supporting `fixture_backed`. | `wrong_result` full live sweep: `cargo make real-world-memory-live-adapters`, `tmp/real-world-memory/live-adapters/elf-report.md`; live capture/write-policy suite passes 4/4 with zero redaction leaks. Narrow operator-debug pass: `cargo make real-world-job-operator-ux-live-adapters`, `tmp/real-world-job/operator-ux-live-adapters/elf-report.md`. Fixture contract: `cargo make real-world-memory`, `tmp/real-world-memory/real-world-memory-report.json`. | `blocked`: private manifest and provider credentials; broader live suites remain `wrong_result`, `blocked`, or `not_encoded`; the narrow operator-debug and live capture/write-policy slices now pass. | Full-suite live pass plus separate private-corpus, credentialed production-ops proof, and durable external capture-hook comparisons. | Keep borrowing qmd debug knobs, OpenViking staged trajectory, mem0 history, Letta core memory, agentmemory/claude-mem capture breadth, and graph/RAG navigation. |
| qmd | Local retrieval-debug workflow with transparent CLI indexing, querying, expansion, fusion, and rerank ergonomics. | `live_real_world`; supporting `live_baseline_only` and `research_gate`. | `wrong_result` full live sweep: `cargo make real-world-memory-live-adapters`, `tmp/real-world-memory/live-adapters/qmd-report.md`; targeted retrieval suites pass; the narrow operator-debug slice ties replay commands but is `wrong_result` for trace hydration and candidate-drop visibility. | `not_encoded`: deep profile and non-retrieval live behavior are not encoded; memory_evolution is `wrong_result`. | Keep qmd deep retrieval/debug profiling separate from the narrow operator-debug live slice; no broad ELF-over-qmd or qmd-over-ELF claim is allowed until comparable stage artifacts exist. | Weighted fusion, rerank explanation, local debug knobs, and command-line replay. |
| agentmemory | Coding-agent continuity, MCP/REST packaging, viewer workflow, and durable cross-agent memory lifecycle. | `live_baseline_only`. | `lifecycle_fail`: `ELF_BASELINE_PROJECTS=agentmemory cargo make baseline-live-docker`, `tmp/live-baseline/live-baseline-report.json`. | `blocked`: durable cold-start, capture-hook persistence, and real-world adapter coverage are missing; current Docker baseline uses a process-local StateKV Map and in-memory index. | Durable local adapter with update, delete, cold-start reload, work_resume, capture/write-policy, and lifecycle-staleness jobs. | Cross-agent hooks, packaging, continuity scenarios, and viewer affordances. |
| mem0/OpenMemory | Memory lifecycle, personalization, hosted/OpenMemory UI ergonomics, and optional graph memory. | `live_baseline_only`. | `pass`: fresh scoped run `cargo make openmemory-ui-export-readback`, `tmp/live-baseline/live-baseline-report.json`, with mem0 `8/8` local SDK checks passing; `blocked`: OpenMemory export-helper setup probe emits `tmp/live-baseline/mem0-openmemory-ui-export.json` with `DOCKER_UNAVAILABLE_IN_BASELINE_RUNNER`. | `blocked`: OpenMemory UI/export cannot be compared until a compose/import path loads the same corpus into the product app; `unsupported`: hosted Platform export; `not_encoded`: optional graph memory and real-world prompt adapter coverage. | Add a Docker-contained OpenMemory product app import/export path, then score browser/API readback separately from SDK `get_all`; keep hosted Platform and graph memory opt-in/non-goal unless explicitly enabled. | Entity-scoped history, lifecycle surfaces, async update ergonomics, and OpenMemory inspection UX. |
| memsearch | Markdown-first canonical store with rebuildable local index and practical hybrid retrieval. | `live_baseline_only`; XY-925 `fixture_backed`. | `pass`: fresh scoped run `ELF_BASELINE_PROJECTS=ELF,agentmemory,mem0,memsearch,claude-mem cargo make baseline-live-docker`, `tmp/live-baseline/live-baseline-report.json`, with memsearch `4/4` local checks passing. XY-925 adds fixture-backed source-store and retrieval-debug prompts through `cargo make real-world-first-generation-oss`, `tmp/real-world-memory/first-generation-oss/report.json`. | `not_encoded`: no live memsearch runtime adapter executes real-world prompt scoring; memory-evolution prompt adapters remain not encoded; TTL/expiry is unsupported by the current CLI path. | Promote the fixture-backed source-store and retrieval-debug prompts into a live memsearch real-world adapter before any suite-level win/loss claim; keep TTL/expiry as unsupported unless a comparable path exists. | Canonical markdown store, local reindex clarity, and user-inspectable source files. |
| OpenViking | Filesystem-like context trajectory, hierarchical retrieval, and staged context loading. | `live_baseline_only`; supporting `fixture_backed` and `research_gate`. | `wrong_result`: `ELF_BASELINE_PROJECTS=OpenViking cargo make baseline-live-docker`, `tmp/live-baseline/live-baseline-report.json`; `blocked`: checked-in `context_trajectory` fixtures cover staged retrieval, hierarchy selection, and recursive/context expansion gates. | `blocked`: hierarchical context trajectory is encoded but blocked until same-corpus evidence ids match and staged artifacts are materialized. | Make evidence-bearing same-corpus output pass, then score staged trajectory and hierarchy expansion. | `viking://`-style context model, trajectory readback, and staged retrieval planning. |
| claude-mem | Progressive disclosure, automatic capture loop, repository-local lifecycle, and local viewer workflow. | `live_baseline_only`; XY-925 `fixture_backed`. | `wrong_result`: `ELF_BASELINE_PROJECTS=claude-mem cargo make baseline-live-docker`, `tmp/live-baseline/live-baseline-report.json`. XY-925 adds fixture-backed progressive-disclosure and retrieval-repair prompts through `cargo make real-world-first-generation-oss`, `tmp/real-world-memory/first-generation-oss/report.json`. | `blocked`: hook capture and viewer/operator workflows still lack a Docker-contained runner; retrieval remains `wrong_result`, and the repair prompt lists rerun/inspection targets `tmp/live-baseline/claude-mem.log` and `tmp/live-baseline/claude-mem-checks.json`. | Promote durable repository-backed work_resume, operator_debugging_ux, capture/write-policy, and progressive-disclosure prompts into a live claude-mem adapter before any broader UX claim. | Progressive disclosure, automatic capture review loops, and local viewer/operator comfort. |
| RAGFlow | Full RAG application workflow with document, chunk, and reference evidence handles. | `research_gate`. | `blocked`: `ELF_RAGFLOW_SMOKE_START=1 ELF_RAGFLOW_SMOKE_ACCEPT_RESOURCE_ENVELOPE=1 cargo make ragflow-docker-smoke`, `tmp/real-world-memory/ragflow-smoke/ragflow-smoke.json`. | `blocked`: Docker resource envelope and adapter output mapping still need proof. | XY-885 tiny Docker evidence-smoke adapter mapping `reference.chunks` to scored evidence. | Document/chunk references, resource-envelope reporting, and RAG app evidence handles. |
| LightRAG | Lightweight graph/RAG context export with source file-path citation shape. | `research_gate`. | `blocked`: `ELF_LIGHTRAG_CONTEXT_START=1 cargo make lightrag-docker-context-smoke`, `tmp/real-world-memory/lightrag-context/summary.json`. | `blocked`: Docker service setup and context export are not proven. | XY-886 Docker context-export adapter with explicit provider config and source citation mapping. | Context-only query modes, graph-aware retrieval layout, and file-path citation readback. |
| GraphRAG | GraphRAG indexing, graph summaries, and document/text-unit evidence tables. | `research_gate`. | `blocked`: `ELF_GRAPHRAG_SMOKE_RUN=1 cargo make graphrag-docker-smoke`, `tmp/real-world-memory/graphrag-smoke/summary.json`. | `blocked`: indexing resource envelope and source citation mapping are not proven. | XY-887 cost-bounded Docker adapter over a tiny corpus and scored output tables. | Graph summary artifacts, local/global search separation, and source table evidence mapping. |
| Graphiti/Zep | Temporal graph memory with current, historical, and future fact validity windows. | `research_gate`. | `blocked`: `ELF_GRAPHITI_ZEP_SMOKE_START=1 ELF_GRAPHITI_ZEP_SMOKE_RUN=1 cargo make graphiti-zep-docker-temporal-smoke`, `tmp/real-world-memory/graphiti-zep-smoke/summary.json`. | `blocked`: Docker graph-store and temporal adapter are not proven. | XY-888 Docker-local temporal graph adapter scoring current/historical fact validity. | Temporal fact windows, invalidation/supersession semantics, and graph fact provenance. |
| Letta | Core memory blocks versus archival memory with explicit operating-context surfaces. | `research_gate`. | `blocked`: the selected comparison contract is a Docker-only benchmark-created agent export that returns core block JSON, archival search/readback JSON, and source ids; no materialized export exists yet. | `blocked`: no Letta materializer currently creates the benchmark agent, imports the ELF `core_archival_memory` fixture corpus, or exports comparable core and archival evidence. | Implement and run the contained export/readback adapter before any Letta win, tie, or loss claim; keep personalization and project-decision scenarios blocked or not tested until that evidence exists. | Core memory block ergonomics, archival separation, and shared operating context readback. |
| LangGraph | Checkpoint/replay regression workflow and durable state replay for agent runs. | `research_gate`. | `not_encoded`: `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json`. | `unsupported`: not a standalone memory backend adapter. | Non-goal for direct win/loss until a standalone memory output contract exists; use replay jobs as benchmark infrastructure reference. | Checkpoint replay, deterministic regression, and state-diff evaluation patterns. |
| nanograph | Typed graph schema and query ergonomics for graph-lite developer experience. | `research_gate`. | `not_encoded`: `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json`. | `unsupported`: not a memory backend comparison target. | Non-goal for direct win/loss unless a contained memory-backed target emerges; measure ELF graph-lite DX instead. | Typed relation schema, query ergonomics, and small graph developer experience. |
| llm-wiki | LLM-maintained wiki or knowledge-page workflow with query-save and lint loops. | `research_gate`. | `not_encoded`: `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json`. | `unsupported`: no live service runtime for adapter proof. | Select contained plugin or instruction harness, then score knowledge pages for citations, unsupported claims, rebuild, and stale-source lint. | Maintained wiki workflows, page lint, query-save loops, and topic-scoped navigation. |
| gbrain | Operational knowledge brain with compiled_truth pages, timelines, enrichment, and maintenance loops. | `research_gate`. | `not_encoded`: `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json`. | `blocked`: Docker-local brain repo and database path are missing. | Prove Docker-local repository/database setup, then encode compiled_truth/timeline and operator-continuity jobs. | Compiled truth pages, timeline maintenance, and human-operable knowledge-brain navigation. |
| graphify | Graph-compressed navigation with `graph.json` and `GRAPH_REPORT` evidence outputs. | Scored tiny `live_real_world` smoke; not broad graph-quality proof. | `wrong_result`: `cargo make graphify-docker-graph-report-smoke`, `tmp/real-world-memory/graphify-smoke/graphify-report.json`. | `not_encoded`: broad graph navigation, multimodal, private-corpus, and large-corpus quality remain outside the tiny smoke. | Expand beyond the generated smoke only after graph/report output maps to scored evidence on representative graph/RAG jobs. | Graph compression, source-location graph reports, and navigation hints for large code or document spaces. |

## Scenario Matrix

| Scenario | Current ELF evidence | Strongest competitor/reference | Current competitor evidence | Next measurement before claim |
| --- | --- | --- | --- | --- |
| Retrieval/debug | Fixture retrieval passes; live retrieval passes. | qmd. | qmd live retrieval passes and live baseline passes, but full-suite live status is `wrong_result`. | Run qmd deep profile and ELF/qmd trace-level replay with expansion, fusion, rerank, and candidate-drop diagnostics. |
| Work resume | Fixture and live work_resume pass. | agentmemory, claude-mem, OpenViking. | agentmemory `lifecycle_fail`; claude-mem work_resume remains `not_encoded` pending a durable repository-backed adapter; OpenViking work_resume is `not_encoded`. | Encode durable work_resume adapters or keep each blocked with lifecycle/setup evidence. |
| Project decisions | Fixture and live project_decisions pass; the ELF core-archival fixture also scores project-decision recovery through core routing plus archival rationale. | qmd, Letta. | qmd live project_decisions pass; Letta project-decision recovery is `research_gate` `not_tested` or `blocked` until the contained export path exists. | Run the Letta core/archival export/readback contract before treating project-decision recovery as a comparable scenario. |
| Source-of-truth | Fixture and live trust_source_of_truth pass. | memsearch. | memsearch canonical-store, reindex, delete, and reload smoke passes; XY-925 fixture-backed source-of-truth prompts now cover the canonical Markdown rebuild/reload boundary, but no live memsearch prompt adapter pass is claimed. | Promote memsearch source-of-truth rebuild/reload prompts into a live adapter before any suite-level win/loss claim. |
| Temporal/current-vs-historical memory | Fixture memory_evolution passes; live memory_evolution is `wrong_result`. | Graphiti/Zep, mem0/OpenMemory. | Graphiti/Zep is `research_gate` `blocked`; mem0/OpenMemory local OSS preference history, entity scope, deletion audit, and SDK `get_all` now pass; OpenMemory UI/export is blocked by the export-helper setup probe; graph-memory scenarios are `not_encoded`. | Fix ELF/qmd live memory_evolution evidence links, add OpenMemory product app import/export readback, and run XY-888. |
| Consolidation | Fixture consolidation passes; XY-934 adds ELF live service-backed proposal scoring with lineage, confidence/usefulness, unsupported-claim flags, and apply/defer/discard audit. | managed dreaming, Always-On Memory Agent patterns, agentmemory, llm-wiki. | No direct live competitor runner emits comparable consolidation artifacts; qmd remains `not_encoded`. | Keep competitor comparisons reference-only until a contained runner emits source ids, confidence, unsupported-claim flags, and review-action audit artifacts. |
| Knowledge pages | Fixture knowledge_compilation passes; live knowledge_compilation is `not_encoded`. | llm-wiki, gbrain, GraphRAG, graphify. | llm-wiki and gbrain are `research_gate` `not_encoded` or `blocked`; GraphRAG is `blocked`; graphify has a tiny scored smoke `wrong_result`. | Encode live derived-page rebuild/lint scoring and run contained knowledge/RAG adapters only after setup proof. |
| Operator debugging | Fixture operator_debugging_ux passes, and the narrow live operator-debug slice passes for trace hydration, candidate-drop visibility, selected-but-not-narrated evidence, replay-command availability, and repair-action clarity. | qmd, claude-mem, OpenMemory. | qmd ties replay-command availability and repair-action clarity but is `wrong_result` for trace hydration, candidate-drop stage visibility, and selected-but-not-narrated evidence. XY-925 adds claude-mem progressive-disclosure and retrieval-repair prompt coverage, while claude-mem viewer/operator and OpenMemory UI/export remain blocked. | Add bounded OpenMemory and claude-mem UI/export or viewer runners before any broader operator-UX claim. |
| Capture/write policy | Fixture capture_integration passes; ELF live capture_integration passes 4/4 with zero redaction leaks, source ids, write-policy audit, and evidence binding. | agentmemory, claude-mem. | agentmemory and claude-mem hook capture remain `blocked` until Docker-contained hook observations and write-policy/viewer readback artifacts exist. | Run durable agentmemory and claude-mem capture-hook jobs proving redaction, exclusion, evidence binding, source ids, and no secret leakage. |
| Production ops | Fixture production_ops has 4 pass and 2 blocked; live production_ops is `blocked`; production adoption has provider/backfill/restore evidence. | ELF production gate, qmd, RAG/RAGFlow resource gates. | qmd live production_ops is `blocked`; RAG/resource gates are `research_gate` `blocked`. | Rerun private-corpus and credentialed gates only when operator-owned manifest and credentials exist. |
| Personalization | Fixture and live personalization pass. | mem0/OpenMemory, Letta. | mem0/OpenMemory local OSS entity-scoped personalization now passes, so scoped preference behavior is a measured tie; OpenMemory UI/export remains blocked, hosted Platform export is non-goal, optional graph memory remains outside local OSS scoring, and Letta personalization is `research_gate` `not_encoded`. | Add OpenMemory product app import/export and contained Letta scoped-preference readback before broader personalization superiority claims. |
| Context trajectory | ELF has trace direction but no comparable staged trajectory scenario. | OpenViking. | OpenViking setup is pinned, same-corpus retrieval is `wrong_result`, and staged/hierarchy/recursive trajectory jobs are encoded as `blocked`. | Make OpenViking evidence-bearing retrieval pass, then score staged context trajectory outputs. |
| Core-vs-archival memory | Fixture `core_archival_memory` passes 6/6 and scores core block attachment, scope, provenance, stale-core detection, archival fallback, and project-decision recovery separately from archival note search. | Letta. | Letta is `research_gate` `blocked`/`not_tested` until the selected contained export/readback artifact exists. | Implement the Letta export/readback adapter, then compare only scenarios whose core block JSON, archival search/readback JSON, and source ids are present. |
| Graph/RAG navigation | ELF relation context is not enough to claim graph/RAG navigation parity. | RAGFlow, LightRAG, GraphRAG, Graphiti/Zep, graphify. | RAGFlow, LightRAG, GraphRAG, and Graphiti/Zep remain `research_gate` blocked/incomplete without explicit setup; graphify has only a tiny scored smoke `wrong_result`. | Run larger contained graph/RAG adapters with evidence-linked outputs before any ELF graph/RAG win, tie, or loss claim. |

## Parallelizable Benchmark Follow-Ups

These workstreams can proceed after this matrix lands because the claim boundaries are
now explicit:

| Workstream | Issue or candidate | Parallelizable | Blocked by | Measurement |
| --- | --- | --- | --- | --- |
| qmd deep retrieval/debug profile | New benchmark issue | yes | None after this matrix lands. | Stress profile plus trace-level retrieval-debug artifacts for qmd and ELF. |
| agentmemory durable lifecycle adapter | `[ELF benchmark P0] Make external adapters lifecycle-durable and fail-typed` | yes | Durable local adapter path selection. | Update, delete, cold-start reload, work_resume, and capture/write-policy jobs. |
| agentmemory/claude-mem capture-hook breadth | Follow-up after XY-933 | yes | Docker-contained hook/viewer capture path with durable artifacts. | Source ids, redaction/exclusion audit, evidence-bound output, and typed blocker reporting. |
| mem0/OpenMemory history and UI coverage | New adapter repair issue | yes | Comparable local OSS path for history/UI/readback evidence. | Preference/entity history, deletion audit readback, personalization, OpenMemory inspection/export, and optional graph-context jobs. |
| memsearch source-of-truth live adapter coverage | New adapter repair issue | yes | Fixture-backed source-store and retrieval-debug prompts are encoded by XY-925; live prompt execution remains missing. | Runtime adapter execution for the existing source-of-truth rebuild/reload and retrieval-debug prompt jobs without converting baseline smoke into suite pass claims. |
| OpenViking context trajectory | XY-928 encoded blocked fixtures | yes | Evidence-bearing same-corpus retrieval output and staged artifacts. | Hierarchical expansion, staged trajectory, recursive/context expansion, and comparable ELF trace/session evidence jobs. |
| claude-mem hook/viewer runtime coverage | New adapter issue | yes | Fixture-backed progressive-disclosure and retrieval-repair prompts are encoded by XY-925; hook capture and viewer/operator workflows remain blocked. | Work resume, operator debugging, capture/write-policy, viewer/operator, and live progressive-disclosure adapter execution. |
| RAGFlow evidence smoke | XY-885 | yes | Resource envelope accepted for tiny Docker smoke. | `reference.chunks` to benchmark evidence mapping. |
| LightRAG context export | XY-886 | yes | Docker service setup and explicit provider config. | Retrieved context export and source file-path citations. |
| GraphRAG cost-bounded adapter | XY-887 | yes | Tiny corpus cost/resource envelope. | Document, text-unit, graph-summary, and citation output tables. |
| Graphiti/Zep temporal graph adapter | XY-888 | yes | Docker-local graph store setup. | Current/historical/future fact validity and evidence ids. |
| graphify graph report adapter | XY-889 plus post-XY-900 expansion | yes | Representative graph/RAG jobs beyond the tiny scored smoke. | `graph.json` and `GRAPH_REPORT` evidence mapped to scored graph navigation and knowledge synthesis ids. |
| Private corpus and credentialed production ops | Operator-owned benchmark gates | no | Sanitized private manifest and routed provider credentials. | Private-corpus retrieval quality and credentialed production-ops evidence. |
| Letta, LangGraph, nanograph, llm-wiki direct adapters | Letta export artifact blocked; others research-only until output contract | no | Letta needs the selected contained export/readback artifact; the others need a non-memory-backend comparability contract. | Run only after comparable output exists; otherwise keep as product-reference evidence. |

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
