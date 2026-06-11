# External Memory Project Comparison

Goal: Provide a detailed, evidence-backed comparison between ELF and adjacent memory projects.
Read this when: You are evaluating architecture directions, positioning claims, or adoption trade-offs.
Inputs: Current ELF docs/code and public documentation for the compared external projects.
Depends on: `docs/spec/system_elf_memory_service_v2.md` and `docs/guide/research/research_projects_inventory.md`.
Outputs: A comparison matrix and trade-off summary suitable for follow-up design decisions.

Scope note: This document is intentionally detailed and source-heavy. Keep `README.md` concise and link here for full analysis.
For a full list of reviewed and pending projects, see `docs/guide/research/research_projects_inventory.md`.
For the June 2026 agentmemory and dreaming decision run, see
`docs/research/2026-06-08-agent-memory-selection.json`.
For the June 2026 real-world benchmark-dimension refresh, see
`docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json`.

Comparison focuses on shared capabilities, ELF distinctives, and objective trade-offs. These projects solve adjacent problems, but their primary storage units and default workflows differ.

Legend:

- `✅`: Built-in and explicitly documented.
- `⚠️`: Partial, optional, transport-specific, or plugin-level support.
- `—`: Not explicitly documented in public docs/readme (as of February 17, 2026).

## Research Method And Confidence

- This comparison is documentation-grounded, not benchmark-grounded.
- ELF claims are code-grounded against this repository; peer claims are documentation-grounded.
- Primary evidence is limited to official public READMEs and official docs from each project.
- A capability is marked `✅` only when explicitly documented as first-class behavior.
- A capability is marked `⚠️` when it exists but is optional, transport-specific, plugin-scoped, or requires extra configuration.
- A capability is marked `—` when no explicit public documentation was found during this review window.
- Snapshot date for all claims in this section: February 17, 2026.

Note: In this section, mem0 refers to the Mem0 ecosystem, including OpenMemory (an MCP memory server with a built-in UI).
OpenViking is included as a newly reviewed project with mechanism-level analysis.

## June 2026 Real-World Benchmark-Dimension Map

Snapshot date for this subsection: June 9, 2026.

This map translates the existing external-project research into benchmark dimensions
for the real-world agent memory suite. It does not add new adapter pass/fail evidence.
Use the evidence class before making claims:

- `benchmark-grounded`: ELF's Docker benchmark has runnable adapter evidence for this
  project and dimension. Read the exact report before quoting a pass/fail result.
- `docs-grounded`: official docs or READMEs indicate a likely strength, but ELF has not
  reproduced the behavior in the benchmark runner.
- `watch`: the project remains D0 or otherwise pending; do not assign strength claims
  until a deep dive or adapter run exists.

Current benchmark-grounded scope is narrow. The June 9, 2026 all-project smoke run
proved encoded same-corpus/lifecycle behavior only for the then-current adapters: ELF
and qmd passed their encoded smoke checks; agentmemory passed same-corpus retrieval but
failed or could not prove durable lifecycle behavior; memsearch, mem0, OpenViking, and
claude-mem retained `incomplete`, wrong-result, or not-encoded states. Later June 11
follow-ups promote scoped local mem0/OpenMemory and memsearch baseline paths, while
OpenMemory UI/export, hosted Platform behavior, optional graph memory, broader
memsearch prompt/TTL coverage, OpenViking staged trajectory, and claude-mem hook/viewer
capture remain blocked, unsupported, not encoded, or wrong-result. All broader suite
fit below is research guidance, not a benchmark result.

The real-world job runner now carries a separate external adapter coverage manifest:
`apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`.
That manifest is a contract and evidence ledger, not a leaderboard. It records which
projects only have `live_baseline_only` Docker retrieval/lifecycle evidence, which
capabilities are `mocked`, `blocked`, `unsupported`, `incomplete`, `wrong_result`, or
`lifecycle_fail`, and which real-world suites remain `not_encoded`. The manifest now
includes full-suite `live_real_world` sweep records for ELF and qmd through
`cargo make real-world-memory-live-adapters`; both retain targeted live pass evidence
for `work_resume`, `retrieval`, and `project_decisions`, but neither is a full-suite
live pass. It also includes `research_gate` records for RAGFlow, LightRAG, GraphRAG,
Graphiti/Zep, Letta, LangGraph, nanograph, llm-wiki, gbrain, graphify, and deeper
qmd/OpenViking profiles. Research gates carry source/setup/runtime/resource/retry
metadata for future adapter work, but they are not fixture-backed, live-baseline-only,
or live-real-world evidence. Other external projects remain live-baseline-only,
incomplete, blocked, or not encoded until their own `real_world_job` adapters run.

XY-882 adds D1/D2 feasibility verdicts for the RAG and graph-memory research gates.
`adapter_candidate` means an implementation follow-up is justified because a scoped
Docker boundary and evidence-linked output contract exist. It does not mean a Docker
adapter has run, and it does not change the `research_gate` evidence class.

Benchmark suite labels:

| Suite | Real-world job shape |
| ----- | -------------------- |
| `rw.resume-evidence` | Resume a stalled agent task, recover the right prior decision, cite required evidence, and avoid negative traps. |
| `rw.lifecycle-staleness` | Update, delete, expire, cold-start, and contradiction cases where stale facts must stop winning. |
| `rw.operator-continuity` | Capture session observations, inspect memory state, and support day-to-day agent continuity with low friction. |
| `rw.retrieval-debug` | Explain query expansion, hybrid retrieval, fusion, rerank, and wrong-result causes. |
| `rw.context-trajectory` | Navigate multi-stage or hierarchical context before selecting final evidence. |
| `rw.knowledge-synthesis` | Compile durable project/entity/concept pages from memory and keep them lintable or repairable. |
| `rw.consolidation-review` | Run background consolidation while keeping derived output reviewable and evidence-linked. |
| `rw.graph-temporal` | Track facts, entities, relations, validity windows, and current-versus-historical answers. |
| `rw.core-archival` | Separate always-loaded operating memory from retrieval-only archival memory. |
| `rw.replay-regression` | Replay, fork, or checkpoint agent state to debug memory-assisted work and regression failures. |
| `rw.graph-navigation` | Use graph-compressed corpus structure to guide agents before raw retrieval or file inspection. |

Project-to-suite map:

| Project | Best-fit real-world suites | Why this project matters for that suite | Fair adapter evidence before claims | Evidence class and confidence | Current ELF position |
| ------- | -------------------------- | -------------------------------------- | ---------------------------------- | ----------------------------- | -------------------- |
| agentmemory | `rw.operator-continuity`, `rw.resume-evidence`, `rw.lifecycle-staleness` | Cross-agent hooks, MCP/REST packaging, viewer, lifecycle/consolidation claims, and coding-agent continuity focus make it the right reference for daily agent memory ergonomics. | Use durable upstream storage rather than the current in-memory mock; ingest realistic agent sessions through the public hook/API path; prove restart, update/supersede, delete, and viewer/trace readback. | Mixed: benchmark-grounded only for current same-corpus retrieval; current lifecycle evidence is a failure/blocker, while hooks/viewer/consolidation are docs-grounded. Confidence: medium for suite fit, low for durable adapter quality. | ELF is stronger on evidence-bound writes and source-of-truth discipline; agentmemory remains the reference for capture breadth and agent-continuity UX. |
| qmd | `rw.retrieval-debug`, `rw.lifecycle-staleness`, `rw.resume-evidence` | Its local CLI, structured JSON query output, expansion modes, hybrid routing, weighted fusion, rerank, update, delete, and cold-start path make it the strongest local retrieval-debug baseline. | Run `qmd` over the real-world corpus, capture query JSON, then rewrite/delete corpus files and rerun update/embed/query in fresh processes. | Benchmark-grounded for current smoke retrieval/update/delete/cold-start pass; docs-grounded for deeper query planning ergonomics. Confidence: high for local adapter baseline. | ELF is not yet stronger on local CLI debug ergonomics; treat qmd as the retrieval-debug reference while keeping ELF's service/provenance model. |
| claude-mem | `rw.operator-continuity`, `rw.resume-evidence`, `rw.retrieval-debug` | Progressive-disclosure search, auto-capture hooks, local viewer, and observation/timeline workflows are directly aligned with real agent resumption jobs. | Exercise a real local repository with hook-driven capture, then evaluate `search -> timeline -> observations` behavior after restart; do not rely on mocked storage. | Docs-grounded for progressive disclosure/viewer; current benchmark adapter evidence is incomplete/wrong-result and mostly not encoded for lifecycle. Confidence: medium for product reference, low for current adapter claims. | ELF has stronger provenance and service boundaries, but claude-mem remains a reference for operator workflow and progressive disclosure UX. |
| mem0 / OpenMemory | `rw.lifecycle-staleness`, `rw.graph-temporal`, `rw.operator-continuity`, `rw.resume-evidence` | Entity-scoped memory, memory history, expiration, hosted/OSS surfaces, OpenMemory UI, and optional graph memory make it the broadest lifecycle and ecosystem comparison target. | Separate OSS local FastEmbed/Qdrant evidence from hosted Platform claims; prove add/update/delete/history, entity-scoped retrieval, expiration exclusion, OpenMemory UI readback, and optional graph context on the same corpus. | Benchmark-grounded for scoped local OSS same-corpus retrieval, update/delete/reload, history, entity filters, local `get_all` readback, and deletion audit; OpenMemory product UI/export remains blocked, hosted Platform is a non-goal, and optional graph plus broader prompt coverage remain not encoded. Confidence: medium for suite fit and scoped local adapter quality, low for product UI/hosted/graph claims. | ELF is stronger on deterministic evidence-bound writes; mem0/OpenMemory remains the reference for ecosystem reach, entity-scoped history, hosted option, and optional graph UX, with local preference-correction history currently measured as an ELF loss. |
| memsearch | `rw.lifecycle-staleness`, `rw.retrieval-debug`, `rw.resume-evidence` | Markdown as canonical memory plus incremental/content-addressed reindexing is a useful model for source transparency and rebuildable derived indexes. | Index a real-world Markdown corpus, mutate/delete files, rerun index/search from fresh processes, and record Milvus mode so Lite/Server/Cloud behavior is not conflated. | Benchmark-grounded for local same-corpus retrieval, reindex/update/delete, and cold-start reload smoke; no real-world prompt adapter is encoded, so Markdown-first behavior remains baseline scenario evidence rather than suite pass evidence. Confidence: medium for design pattern and scoped local adapter evidence, low for broad real-world adapter coverage. | ELF already owns source-of-truth plus rebuildable index at service level; memsearch remains a reference for simple local canonical-store ergonomics and transparent local reindexing. |
| OpenViking | `rw.context-trajectory`, `rw.resume-evidence`, `rw.retrieval-debug` | `viking://` context organization, intent analysis, hierarchical retrieval, staged find/search behavior, and session compression are relevant to multi-hop agent context jobs. | Use the pinned Docker local embedding path, then evaluate `add_resource`/`find`/`search` over multi-stage jobs with stage output, hierarchy, and session memory evidence. | Docs-grounded for mechanism; current benchmark adapter reaches local embedding setup and `add_resource`/`find`, but remains `wrong_result` because same-corpus evidence terms are missed. Confidence: medium for architecture reference, low for runnable adapter quality. | ELF has first-class traces and evidence-bound notes, but OpenViking is the reference for hierarchical context trajectory and filesystem-like organization. |
| llm-wiki | `rw.knowledge-synthesis`, `rw.resume-evidence` | Query/save/lint flows and topic-scoped wiki pages are a useful reference for turning retrieved memory into maintained project knowledge. | Run a corpus-to-wiki job, ask resume/decision questions, require page citations back to source memory, then mutate a stale source and prove lint/repair catches it. | Docs-grounded D1; no benchmark adapter evidence. Confidence: medium for derived-knowledge fit. | ELF is not yet stronger on derived knowledge pages; llm-wiki should inform rebuildable, evidence-cited dossiers rather than core storage. |
| gbrain | `rw.knowledge-synthesis`, `rw.operator-continuity` | `compiled_truth`, timeline sections, backlinks, primary-home routing, and enrichment workflows model a living operational brain for project work. | Build or update pages from the real-world corpus, require current-truth plus timeline answers, and prove enrichment/backlink maintenance does not hide unsupported claims. | Docs-grounded D1; no benchmark adapter evidence. Confidence: medium for operator knowledge UX. | ELF should keep source notes authoritative; gbrain is a reference for presentation, enrichment, and maintenance loops. |
| Always-On Memory Agent | `rw.consolidation-review`, `rw.operator-continuity` | The file/API/dashboard ingest loop and timer-based consolidation show how background memory formation becomes a user-visible product surface. | Run scheduled consolidation on a fixed corpus, record source rows and output insights, then score whether consolidation is reviewable, repeatable, and bounded against unsupported claims. | Docs-grounded D1; no benchmark adapter evidence. Confidence: medium for consolidation workflow reference. | ELF should borrow scheduling and operator controls while keeping deterministic writes and reviewable derived outputs. |
| graphify | `rw.graph-navigation`, `rw.knowledge-synthesis`, `rw.resume-evidence` | Deterministic code extraction, LLM-assisted graph building, honesty tags, graph reports, and assistant hooks are strong references for graph-compressed navigation over large corpora. | Generate graph/report artifacts from the benchmark corpus, require answers to use graph structure plus source evidence, and prove rebuild behavior after corpus edits. | Scored tiny `live_real_world` smoke: `cargo make graphify-docker-graph-report-smoke` records a Docker-only generated-corpus graph/report artifact and currently scores `wrong_result`; the checked-in manifest does not claim broad graph quality, rebuild strength, or production-quality graph navigation. Confidence: medium for adapter feasibility, low for production-quality graph navigation. | ELF is stronger as a memory service; graphify is now a runnable reference for derived graph reports and pre-search guidance, but not yet a stronger end-to-end memory system. |
| Letta | `rw.core-archival`, `rw.operator-continuity` | Core memory blocks, archival memory, and shared/read-only memory blocks map directly to always-loaded operating context versus retrievable memory. | Build a multi-agent job where core blocks must be attached/detached/shared read-only, while archival memory is retrieved separately and audited. | Docs-grounded D1; no benchmark adapter evidence. Confidence: medium for memory-semantics reference. | ELF has scoped notes but not first-class core/archival block ergonomics; Letta is the reference dimension. |
| LangGraph | `rw.replay-regression`, `rw.resume-evidence` | Thread checkpoints, durable execution, replay, fork, and time travel define a strong model for debugging agent-state and memory-regression behavior. | Run an agent job with memory reads across checkpoints, replay/fork the thread after a stale-memory failure, and verify side-effect boundaries. | Docs-grounded D1; no benchmark adapter evidence. Confidence: medium for replay workflow reference. | ELF traces are useful but do not replace full agent checkpoint replay; LangGraph is the reference for replay-regression jobs. |
| Graphiti / Zep | `rw.graph-temporal`, `rw.resume-evidence` | Temporal entities, relations, fact triples, validity windows, and graph search directly target stale/contradictory factual memory. | Add fact triples with validity changes, query current and historical answers, and score invalidation/append behavior under contradiction traps. | Docs-grounded D1; no benchmark adapter evidence. Confidence: medium-high for temporal-graph dimension. | ELF graph-lite covers evidence-linked validity windows and current/historical relation context; Graphiti/Zep remains the reference for broader temporal graph workflows. |
| nanograph | `rw.graph-temporal`, `rw.retrieval-debug` | Typed schema and typed query ergonomics are relevant to making ELF graph-lite interactions inspectable and hard to misuse. | Define typed graph schemas and queries for the same fact set, then score developer-visible validation, query shape, and explainability rather than retrieval quality alone. | Docs-grounded D1; no benchmark adapter evidence. Confidence: medium for DX reference, low for memory-system comparison. | ELF should borrow typed graph ergonomics without treating nanograph as a full memory backend. |

XY-882 feasibility verdicts for RAG and graph-memory gates:

| Project | Verdict | Docker boundary | Evidence-linked output contract | Follow-up |
| ------- | ------- | --------------- | ------------------------------- | --------- |
| RAGFlow | `adapter_candidate` | Official Docker Compose path, but the first adapter must use a tiny CPU corpus and record the 4 CPU / 16 GB RAM / 50 GB disk envelope, image size, `vm.max_map_count`, provider needs, and retry behavior. | OpenAI-compatible and agent completion responses can include `reference.chunks` with chunk id, document id/name, metadata, dataset id, positions, and similarity fields. | [XY-885](https://linear.app/hack-ink/issue/XY-885/elf-benchmark-adapter-implement-ragflow-docker-evidence-smoke-adapter); no live pass claim. |
| LightRAG | `adapter_candidate` | Docker Compose server with explicit LLM, embedding, rerank, storage, workspace, and data-volume configuration. | Context-only query modes can return the context prepared for the LLM; core APIs can insert documents with ids and source file paths. | [XY-886](https://linear.app/hack-ink/issue/XY-886/elf-benchmark-adapter-implement-lightrag-docker-context-export-adapter); no live pass claim. |
| GraphRAG | `adapter_candidate` | Cost-bounded Docker Python CLI/API run over a generated tiny corpus with container-local parquet artifacts. | Output tables contain generated UUIDs, human-readable ids, source documents, text units, community reports, and text-unit links for graph summaries and relationships. | [XY-887](https://linear.app/hack-ink/issue/XY-887/elf-benchmark-adapter-implement-graphrag-cost-bounded-docker-adapter); no live pass claim. |
| Graphiti / Zep | `adapter_candidate` | Docker-local FalkorDB or Neo4j plus Python SDK runner with provider config captured under benchmark artifacts. | Search results and fact triples expose UUIDs, fact text, and validity windows (`valid_at` / `invalid_at`) that map to memory-evolution scoring. | [XY-888](https://linear.app/hack-ink/issue/XY-888/elf-benchmark-adapter-implement-graphitizep-temporal-graph-adapter); no live pass claim. |
| graphify | `adapter_candidate` | Docker-only CLI/materializer using `pip install graphifyy` over a mounted corpus; host-global assistant hooks are out of scope. | `graph.json`, `GRAPH_REPORT.md`, and graph query output include edge types, confidence tags, source files, and source locations. | [XY-889](https://linear.app/hack-ink/issue/XY-889/elf-benchmark-adapter-implement-graphify-docker-graph-report-adapter) adds `cargo make graphify-docker-graph-report-smoke`; XY-900 promotes the tiny generated smoke to scored `live_real_world` `wrong_result` evidence while still avoiding broad quality claims. |
| Letta | `research_only` | Docker server exists, but current docs require explicit embedding configuration and steer Letta Code evaluation toward non-Docker local/frontier-model exploration. | Core/archival memory and shared blocks remain useful semantics, but no contained evidence export is selected for this adapter batch. | No implementation issue. |
| LangGraph | `research_only` | A Docker harness is possible, but the project is an agent-state/checkpoint framework rather than a standalone memory adapter. | Store search and checkpoints are references for replay-regression jobs, not a direct external memory output contract here. | No implementation issue. |
| nanograph | `research_only` | Official positioning is one CLI / one folder / no server / no Docker. | Typed schema, query, CDC, and search ergonomics remain graph-lite DX inspiration. | No implementation issue. |
| llm-wiki | `research_only` | Plugin or instruction-file workflow would require a contained harness before scoring; host-global plugin installs are not proof. | Wiki compile/query/lint/audit workflows are derived-knowledge references, not current adapter outputs. | No implementation issue. |
| gbrain | `blocked` | A Docker-local brain repo and database setup path was not proven in this lane. | Compiled truth, timeline, and source attribution are strong, but not enough for implementation without contained setup proof. | No implementation issue until Docker setup is proven. |

## Where ELF Is Not Yet The Reference

| Benchmark dimension | Current reference project(s) | ELF gap to test before claiming strength |
| ------------------- | ---------------------------- | ---------------------------------------- |
| Local retrieval debugging and CLI transparency | qmd | ELF needs equally fast local knobs/readback for expansion, hybrid fusion, rerank, and wrong-result diagnosis. |
| Turn-by-turn agent capture and daily continuity | agentmemory, claude-mem, OpenMemory | ELF has service and viewer surfaces, but not the same turnkey hook breadth or session-continuity product ergonomics. |
| Progressive disclosure UX | claude-mem, OpenViking | ELF has L0/L1/L2 shaping and traces, but the operator workflow still needs better search-session navigation. |
| Entity-scoped history and managed ecosystem reach | mem0/OpenMemory | ELF has ingest decisions and versions, but not the same hosted option, SDK reach, or first-class memory history surface. |
| Core memory versus archival memory | Letta | ELF scopes notes well, but lacks attachable/read-only core memory blocks as a distinct user-facing layer. |
| Temporal graph validity | Graphiti/Zep | ELF graph-lite now persists validity windows and labels current versus historical relation context, while Graphiti/Zep remains the broader reference for temporal graph workflows. |
| Agent replay and forkable regression debugging | LangGraph | ELF traces are replay evidence for retrieval, not full persisted agent-state replay with side-effect boundaries. |
| Derived knowledge pages and lint/repair loops | llm-wiki, gbrain | ELF does not yet ship rebuildable entity/project pages with unsupported-claim lint as a first-class workflow. |
| Scheduled consolidation as a product surface | Always-On Memory Agent | ELF's target should be reviewable derived consolidation, but the scheduling/operator-control workflow is not implemented. |
| Graph-compressed navigation over large corpora | graphify, GraphRAG/LightRAG adapter candidates | ELF relation context is bounded and evidence-linked, but broader graph report/navigation workflows remain future work. |

## June 2026 Agentmemory And Dreaming Refresh

Snapshot date for this subsection: June 8, 2026.

This refresh re-evaluates ELF after the June 2026 hardening work and after the
appearance of [agentmemory](https://github.com/rohitg00/agentmemory) as a high-velocity
coding-agent memory project. It also records the current vendor direction around
dreaming-style background memory consolidation.

### Current ELF Position

ELF remains strongest as a high-trust memory service rather than a turnkey coding-agent
continuity plugin. The current main branch has:

- evidence-linked fact writes and quote-bound provenance;
- deterministic `add_note` separated from LLM-driven `add_event`;
- Postgres as source of truth and Qdrant as a rebuildable derived index;
- scoped HTTP/MCP service semantics, TTL/lifecycle policy, graph-lite relation context,
  and retrieval evaluation tooling;
- recently restored local gates, stricter config presence, generated OpenAPI/Scalar docs,
  and Docker Compose service dependencies.

### agentmemory

agentmemory is now important enough to track as a first-class comparison target. Its
public README advertises cross-agent support for Claude Code, Codex CLI, Cursor, Gemini
CLI, OpenCode, and generic MCP clients; MCP/REST access; hook-based capture; hybrid
BM25/vector/graph retrieval; consolidation/lifecycle behavior; a local viewer on `:3113`;
and iii console observability for traces, KV state, triggers, queues, and streams. Its
roadmap still lists benchmark CI, session replay UI, governance baseline, enterprise trust
features, and a v1.0 stability freeze as future work.

ELF implication: do not replace ELF with agentmemory. Treat it as:

- an optional capture/import adapter for coding-agent session observations;
- a benchmark and UX baseline for local continuity workflows;
- a source of product ideas around hooks, viewer, replay, audit, and tool breadth.

### Dreaming And Background Consolidation

OpenAI frames dreaming as background curation that synthesizes memory state, applies
preferences, and keeps memory current over time. Anthropic Claude Dreams is the strongest
safety reference: a dream reads an input memory store plus 1-100 sessions, produces a
separate output memory store, never modifies the input store, and leaves the output
reviewable, attachable, discardable, archivable, or deletable. Google examples add two
operator patterns: Always-On Memory Agent runs scheduled consolidation, while Gemini CLI
Auto Memory mines idle transcripts but writes reviewable patches and skill drafts to an
inbox before anything is applied.

ELF implication: dreaming should be a reviewed derived layer over authoritative evidence,
not a destructive rewrite path. The target shape is:

- immutable observations, notes, events, traces, and source pointers as input;
- asynchronous consolidation jobs that produce candidate derived memories, pages, graph
  views, or skills;
- explicit lineage, diff, confidence, contradiction/staleness markers, and review/apply
  controls;
- rebuildable outputs that can be discarded without corrupting source-of-truth memory.

### Current Recommendation

Continue building ELF. Do not directly adopt agentmemory or managed dreaming as the core
backend. The next work should prioritize:

1. a reviewable derived consolidation pipeline;
2. read-only viewer plus retrieval/consolidation observability;
3. optional agentmemory import/baseline adapter;
4. graph-lite typed query and derived knowledge pages with provenance/lint.

This ordering reuses the existing vNext planning surface instead of starting a parallel
roadmap: [XY-286](https://linear.app/hack-ink/issue/XY-286/knowledge-memory-derived-entityconceptproject-pages-with-provenance),
[XY-19](https://linear.app/hack-ink/issue/XY-19/add-a-read-only-web-viewer-for-sessions-and-traces),
[XY-27](https://linear.app/hack-ink/issue/XY-27/viewer-add-retrieval-observability-panels-on-top-of-the-read-only),
and [XY-70](https://linear.app/hack-ink/issue/XY-70/graph-lite-dx-typed-schema-typed-query-nanograph-inspired)
remain the right backbone.

Primary sources for this refresh:

- https://github.com/rohitg00/agentmemory
- https://raw.githubusercontent.com/rohitg00/agentmemory/main/ROADMAP.md
- https://openai.com/index/chatgpt-memory-dreaming/
- https://platform.claude.com/docs/en/managed-agents/dreams
- https://github.com/GoogleCloudPlatform/generative-ai/tree/main/gemini/agents/always-on-memory-agent
- https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/auto-memory.md

## Scope And Intended Use

| Aspect             | ELF                                                   | [memsearch](https://github.com/zilliztech/memsearch) | [qmd](https://github.com/tobi/qmd) | [claude-mem](https://github.com/thedotmack/claude-mem) | [mem0](https://github.com/mem0ai/mem0) |
| ------------------ | ----------------------------------------------------- | ---------------------------------------------------- | ---------------------------------- | ------------------------------------------------------ | -------------------------------------- |
| Primary artifact   | Evidence-bound notes                                  | Markdown memory files + Milvus index                | Local Markdown index (chunks)      | Session observations and summaries                      | User, session, and agent memories      |
| Default write path | HTTP `POST /v2/notes/ingest` / `POST /v2/events/ingest` | CLI hooks + Python API (Markdown-first)             | CLI index + search                 | Auto-capture via Claude Code plugin hooks              | SDK/API (LLM-assisted)                 |
| Default deployment | API + worker + MCP server                             | Local package + Milvus (Lite/Server/Cloud) + plugin | Local CLI + MCP server             | Local plugin + worker + UI + MCP tools                 | SDK + hosted option; OpenMemory MCP server + UI |

## Interfaces And Integration

| Capability                      | ELF | memsearch | qmd | claude-mem | mem0 |
| ------------------------------- | --- | --------- | --- | ---------- | ---- |
| Local-first, self-hosted memory | ✅  | ✅        | ✅  | ✅         | ✅ (OpenMemory) |
| MCP integration                 | ✅  | ⚠️        | ✅  | ✅         | ✅ (OpenMemory) |
| HTTP API service                | ✅  | —         | ⚠️  | ✅         | ✅ (SDK/API) |
| CLI-first workflow              | —   | ✅        | ✅  | ⚠️         | —    |
| Web UI viewer                   | —   | —         | —   | ✅         | ✅ (OpenMemory) |
| Hosted option                   | —   | —         | —   | —          | ✅    |

## Retrieval Pipeline

| Capability                                  | ELF | memsearch | qmd | claude-mem | mem0 |
| ------------------------------------------- | --- | --------- | --- | ---------- | ---- |
| Full-text search (BM25/FTS/keyword modes)  | ✅  | ✅        | ✅  | ✅         | ⚠️   |
| Vector semantic search                      | ✅  | ✅        | ✅  | ✅         | ✅    |
| Hybrid dense + sparse fusion                | ✅  | ✅        | ✅  | ✅         | ⚠️   |
| LLM reranking stage                         | ✅  | —         | ✅  | —          | ⚠️   |
| Query expansion or query rewriting          | ✅  | —         | ✅  | —          | ⚠️   |
| Progressive disclosure workflow             | ✅  | ⚠️        | —   | ✅         | —    |

## Quality, Safety, And Memory Semantics

| Capability                                    | ELF | memsearch | qmd | claude-mem | mem0 |
| --------------------------------------------- | --- | --------- | --- | ---------- | ---- |
| Evidence-bound notes (verbatim quotes)        | ✅  | —         | —   | —          | —    |
| Deterministic vs LLM ingestion separation     | ✅  | —         | —   | —          | —    |
| Source-of-truth storage with rebuildable index | ✅  | ✅        | —   | —          | —    |
| Multi-tenant scoping                          | ✅  | —         | —   | —          | ✅    |
| TTL and lifecycle policies                    | ✅  | —         | —   | —          | ✅    |
| First-class graph memory mode                | ⚠️ (graph-lite via `POST /v2/graph/query`) | — | — | — | ✅ (optional) |
| Redaction or write-time exclusion controls    | ✅  | —         | —   | ⚠️         | ⚠️   |

## Operations And Evaluation

| Capability               | ELF | memsearch | qmd | claude-mem | mem0 |
| ------------------------ | --- | --------- | --- | ---------- | ---- |
| Retrieval evaluation CLI | ✅  | —         | —   | —          | —    |
| Structured JSON outputs  | ✅  | ⚠️        | ✅  | ✅         | ✅    |

Capability notes:

- qmd HTTP support is MCP Streamable HTTP (`POST /mcp`) rather than a separate REST memory API ([source](https://github.com/tobi/qmd?tab=readme-ov-file#streamable-http)).
- memsearch integration is currently plugin/CLI-centric; no standalone MCP server is documented ([source](https://github.com/zilliztech/memsearch)).
- memsearch progressive disclosure is described in the Claude plugin workflow docs, not as a generic service contract ([source](https://github.com/zilliztech/memsearch/tree/main/ccplugin)).
- ELF graph mode is intentionally graph-lite: scoped temporal facts are queried through `POST /v2/graph/query`, with optional explain payload `elf.graph_query/v1` and evidence-linked fact rows.
- mem0 graph memory is optional and requires an OpenAI-compatible LLM setup ([source](https://docs.mem0.ai/platform/features/graph-memory)).
- mem0 search docs describe optional reranking, query optimization, and keyword-search toggles ([source](https://docs.mem0.ai/platform/features/search-filters)).
- mem0 lifecycle docs describe `expiration_date` and automatic exclusion of expired memories from retrieval ([source](https://docs.mem0.ai/cookbooks/essentials/memory-expiration-short-and-long-term)).
- claude-mem supports `<private>` tags to exclude selected content from storage ([source](https://github.com/thedotmack/claude-mem?tab=readme-ov-file#memory-privacy-controls)).

## Project Strengths And Trade-offs

- [memsearch](https://github.com/zilliztech/memsearch): Strong Markdown-first transparency, smart dedup, and live file-watch sync. Trade-off: integration is centered on plugin/CLI workflows rather than a general MCP + HTTP service surface.
- [qmd](https://github.com/tobi/qmd): Strong local-first retrieval quality (BM25 + vector + rerank + query expansion) with practical CLI and MCP tooling. Trade-off: focused on document retrieval workflows more than memory-specific safety/lifecycle semantics.
- [claude-mem](https://github.com/thedotmack/claude-mem): Strong automatic capture and progressive disclosure UX, plus a practical local web viewer for inspection. Trade-off: optimized for Claude session continuity, with fewer explicit deterministic ingestion boundaries.
- [mem0](https://github.com/mem0ai/mem0): Strong ecosystem reach (SDK + hosted + OpenMemory), multi-entity scoping, and lifecycle controls like `expiration_date`. Trade-off: ingestion and retrieval behavior depends heavily on configurable LLM-assisted flows, which can be less deterministic by default.
- [OpenViking](https://github.com/volcengine/OpenViking): Strong context filesystem paradigm (`viking://`), hierarchical retrieval, and session-centric context iteration. Trade-off: relation model is URI-link based (not property graph), and adoption still requires adapting patterns into ELF's evidence-bound note contract.
- [llm-wiki](https://github.com/nvk/llm-wiki): Strong LLM-maintained wiki pattern, topic-scoped knowledge bases, and explicit query/save/lint flows. Trade-off: wiki pages are the primary interface, so ELF-grade provenance and trust boundaries must remain layered above it.
- [gbrain](https://github.com/garrytan/gbrain): Strong operational knowledge-brain shape with primary-home routing, `compiled_truth` + timeline pages, and explicit maintenance/enrichment workflows. Trade-off: page-first ontology and personal-brain workflow assumptions would over-couple ELF core to one UI/content model if copied directly.
- [Always-On Memory Agent](https://github.com/GoogleCloudPlatform/generative-ai/tree/main/gemini/agents/always-on-memory-agent): Strong always-on ingest/consolidate/query loop with multimodal inbox, timer-driven consolidation, simple SQLite persistence, and a lightweight dashboard/API. Trade-off: memory formation is LLM-first, so it does not preserve ELF-style deterministic write boundaries or evidence-bound fact contracts.
- [graphify](https://github.com/safishamsi/graphify): Strong multimodal graph compression with deterministic AST extraction for code, explicit `EXTRACTED`/`INFERRED`/`AMBIGUOUS` relation tagging, and always-on assistant hooks. Trade-off: it is closer to a graph-guided corpus understanding skill than a multi-tenant memory service, so its graph artifact should be treated as a derived operator surface rather than a source-of-truth memory backend.
- [nanograph](https://github.com/nanograph/nanograph): Strong typed schema + typed query developer ergonomics. Trade-off: focuses on graph-first DX patterns rather than ELF's evidence-bound notes + multi-tenant service contract.

## nanograph Snapshot (New)

Snapshot date for this subsection: March 4, 2026.

- nanograph's docs emphasize typed schema and typed query surfaces for working with structured graph data.
- Relevance for ELF: a concrete reference for making graph-lite interaction feel like a first-class API (schema + query + explain), while ELF remains evidence-bound and scope-governed.

Primary references:

- [nanograph](https://github.com/nanograph/nanograph)
- [Schema docs](https://github.com/nanograph/nanograph/blob/main/docs/user/schema.md)
- [Query docs](https://github.com/nanograph/nanograph/blob/main/docs/user/queries.md)

## LLM Wiki And Operational Brain Snapshot (New)

Snapshot date for this subsection: April 16, 2026.

| Project | Primary knowledge unit | Relevant mechanism | Implication for ELF |
| ------- | ---------------------- | ------------------ | ------------------- |
| [llm-wiki](https://github.com/nvk/llm-wiki) | Topic-scoped wiki pages maintained as the working knowledge base | Query-answer-save loop, lint/repair workflow, and explicit inspiration from Karpathy's LLM Wiki framing | Strong reference for a derived knowledge-memory layer and operator-friendly compiled knowledge workflow; should sit above ELF core facts and evidence rather than replace them |
| [gbrain](https://github.com/garrytan/gbrain) | Slugged brain pages with one primary home, `compiled_truth`, timeline, and backlinks | Resolver-based routing, schema-guided page types, enrichment as a shared service, hybrid search with compiled-truth boost, and explicit maintenance commands | Strong reference for turning memory into an operational knowledge base; should inform ELF knowledge-memory UX and maintenance loops, not its source-of-truth contract |

Key takeaways for ELF from this snapshot:

- Both projects reinforce a useful framing: knowledge is maintained memory, not a separate system.
- Both are more valuable as references for ELF's future knowledge-memory layer than for ELF core ingestion semantics.
- Both treat maintenance as first-class product surface area through lint, enrich, backlink, query-save, or repair flows rather than as a side task.

## Always-On Memory And Graphify Snapshot (New)

Snapshot date for this subsection: April 17, 2026.

| Project | Primary artifact | Relevant mechanism | Implication for ELF |
| ------- | ---------------- | ------------------ | ------------------- |
| [Always-On Memory Agent](https://github.com/GoogleCloudPlatform/generative-ai/tree/main/gemini/agents/always-on-memory-agent) | SQLite-backed memories plus timer-generated consolidation insights | Multimodal inbox/file-watcher ingest, scheduled consolidation pass, simple query API, and lightweight dashboard | Strong reference for productizing background memory formation and manual/automatic consolidation triggers, but ELF should keep evidence-bound facts and deterministic note paths instead of making every write LLM-first |
| [graphify](https://github.com/safishamsi/graphify) | Persistent `graph.json` + `GRAPH_REPORT.md` + optional wiki derived from a multimodal corpus | Deterministic AST extraction for code, LLM extraction for docs/media, graph-topology clustering, explicit honesty tags, and always-on assistant hooks | Strong reference for derived graph/wiki operator surfaces and graph-guided navigation over large corpora, but the graph should remain a rebuildable derived view over ELF notes/docs rather than the authoritative store |

Key takeaways for ELF from this snapshot:

- Always-on consolidation is a product surface, not just an agent prompt pattern.
- A compressed graph/report layer can materially improve how assistants navigate large corpora before they touch raw files.
- Both projects are strongest when treated as derived layers above a trustworthy base store, not as replacements for ELF core memory semantics.

## OpenViking Deep Dive (New)

Snapshot date for this subsection: February 17, 2026.

| Aspect | OpenViking observation | Implication for ELF |
| ------ | ---------------------- | ------------------- |
| Core paradigm | Filesystem-oriented context model (`viking://`) unifying resource, memory, and skill directories | Useful for retrieval organization and payload shaping; does not require graph database adoption |
| Storage design | Dual-layer storage: AGFS as content source-of-truth + vector index for semantic retrieval | Aligns with ELF's current SoT + derived index principle |
| Retrieval flow | Intent analysis -> hierarchical recursive retrieval -> rerank -> structured result | High-value blueprint for improving complex-query quality in ELF |
| Relation model | Explicit URI relation table via `.relations.json` and link/unlink APIs | Indicates graph-like utility can be achieved without Neo4j-first architecture |
| Session iteration | Session commit/compress + memory extraction loop | Useful reference for memory evolution and operational observability |
| Neo4j signal | No first-class Neo4j dependency or property-graph backend in published architecture | Does not support prioritizing Neo4j for ELF at current stage |

## Mechanism-Level Deep Dive (Beyond README)

Snapshot date for this subsection: February 17, 2026.

| Project | Ingestion and update semantics | Retrieval internals | Consistency and reliability model | Operational profile |
| ------- | ------------------------------ | ------------------- | --------------------------------- | ------------------- |
| [OpenViking](https://github.com/volcengine/OpenViking) | Session-centric commit/compress and memory extraction; relation writes are explicit URI links | Intent analyzer + hierarchical recursive retrieval + optional rerank | Clear stage decomposition and traceable retrieval trajectory concept | Strong context-organization patterns; requires adaptation to ELF evidence-bound semantics |
| [mem0](https://github.com/mem0ai/mem0) | `add()` can run LLM-guided `ADD/UPDATE/DELETE/NONE`; history events are persisted; optional graph extraction runs alongside vector memory | Dense retrieval is core; rerank/filter are optional; graph mode adds relation retrieval as an extra context channel | OSS sync mode waits for processing completion; Platform API is async-by-default with event queue semantics | Rich hosted + OSS surface; stronger built-in feedback/events, but more tuning knobs and potential latency/cost variance |
| [memsearch](https://github.com/zilliztech/memsearch) | Markdown is canonical; reindex is incremental/content-addressed; stale chunks are removed by hash-based reconciliation | Milvus hybrid search (dense + BM25 sparse) with RRF fusion | Plugin hook workflow favors practical continuity; failures are mostly handled operationally rather than through strict policy contracts | Very pragmatic local workflow; Milvus Lite/Server/Cloud flexibility, but capability envelope depends on Milvus mode |
| [qmd](https://github.com/tobi/qmd) | Content-addressed SQLite model; `qmd update` reactivates/upserts and deactivates missing documents | Typed query expansion (`lex/vec/hyde`), hybrid routing, weighted RRF, then rerank blend by rank bands | Strong deterministic local index behavior with schema self-healing for vector tables | Excellent local-first control and explainability; less focused on multi-tenant memory governance semantics |
| [claude-mem](https://github.com/thedotmack/claude-mem) | Hook-driven capture tied to Claude Code lifecycle; queue-backed worker persists pending tasks | Progressive-disclosure retrieval is explicit (`search -> timeline -> get_observations`); hybrid local stack (SQLite + Chroma) | Deliberate fail-open handler behavior reduces workflow interruption but may accept occasional capture gaps | Best-in-class local operator ergonomics (viewer/SSE/logs), centered on Claude-centric usage patterns |
| [llm-wiki](https://github.com/nvk/llm-wiki) | Topic-specific wiki artifacts persisted as the working knowledge base | Query-answer-save loop over wiki state, lint/repair workflow, and an explicit LLM Wiki model | Strong practical workflow for compiled knowledge, but the wiki itself is the primary artifact rather than a strictly derived view | Useful model for ELF-derived dossiers/concept pages and memory linting, not for replacing evidence-bound facts as authoritative state |
| [gbrain](https://github.com/garrytan/gbrain) | Page-first brain with schema-guided slugs/types/tiering and `compiled_truth` + timeline sections | Hybrid search with compiled-truth boosting, resolver-based primary-home routing, and shared enrichment service callable from multiple ingest paths | Strong operator workflow for maintaining a living knowledge base, but trust/provenance depends on page upkeep discipline | Useful model for ELF knowledge-memory presentation and enrichment loops if pages remain derived and pointer-backed |
| [Always-On Memory Agent](https://github.com/GoogleCloudPlatform/generative-ai/tree/main/gemini/agents/always-on-memory-agent) | Always-on memory loop over local SQLite rows and consolidation insights | File watcher/dashboard/API ingest, timer-based consolidation, and lightweight local query surface over multimodal inputs | Operationally simple and product-legible, but memory formation is LLM-first and does not separate deterministic note writes from derived synthesis | Useful model for adding first-class consolidation scheduling and operator controls without relaxing ELF write-path invariants |
| [graphify](https://github.com/safishamsi/graphify) | Derived knowledge graph plus graph report/wiki built from code and multimodal corpus inputs | Deterministic AST extraction, LLM-assisted relation extraction, topology-based clustering, and hook-driven assistant guidance | Excellent for graph-guided corpus navigation, but not a general memory contract and not scoped around multi-tenant storage semantics | Useful model for ELF-derived graph reports, graph-guided query surfaces, and assistant hooks over rebuildable derived artifacts |

Key takeaways for ELF from this deeper pass:

- mem0 demonstrates that graph context can be additive instead of replacing vector retrieval.
- qmd shows retrieval quality gains from explicit routing heuristics and transparent score fusion.
- memsearch validates a strong pattern: canonical primary store + rebuildable derived index.
- claude-mem demonstrates how much adoption improves when operator inspection is first-class.
- OpenViking reinforces that context organization and retrieval trajectory can deliver large gains without Neo4j-first architecture.
- llm-wiki reinforces the value of a query/save/lint workflow around compiled knowledge artifacts rather than treating every answer as ephemeral.
- gbrain reinforces that a useful knowledge base often looks like maintained entity/project pages with current truth plus timeline, not just a bag of retrieved chunks.
- Always-On Memory Agent reinforces that scheduled consolidation and manual consolidation triggers are product-level features, not just internal implementation details.
- graphify reinforces that graph-compressed corpus views and pre-search graph guidance can meaningfully reduce raw-file thrash for assistants.

## Where ELF Is Currently Weaker (Objective Gaps)

- ELF now has a local admin viewer and retrieval observability surfaces, but
  claude-mem, OpenMemory, and agentmemory remain stronger references for turnkey
  memory-inspection and session-continuity ergonomics.
- No hosted/cloud product option (mem0 provides managed deployment).
- Graph support is currently graph-lite (`POST /v2/graph/query`) and does not yet include multi-hop/global graph reasoning patterns used by GraphRAG-focused projects.
- Less turnkey for zero-config local plugin workflows than memsearch/claude-mem defaults.
- Supports explicit `quick_find` vs `planned_search` split through `POST /v2/searches` mode.
- Stage-level retrieval trajectory summary is now first-class on `/v2/searches` responses (`search_retrieval_trajectory/v1`), but operator-facing trajectory inspection ergonomics are still evolving.

## Extended Deep-Dive Comparison (Reference Only)

Snapshot date for this subsection: February 17, 2026.

| Project | Distinct memory model | High-value mechanism | Known trade-off | Optional takeaway for ELF |
| ------- | --------------------- | -------------------- | --------------- | -------------------------- |
| [mem0](https://github.com/mem0ai/mem0) | Entity-scoped memories (`user_id`/`agent_id`/`app_id`/`run_id`) with optional graph augmentation | Async ingestion + webhooks, explicit memory history events, optional graph relations context | Async default introduces read-after-write complexity; graph path adds cost and provider coupling | Add first-class memory update events and stronger entity-scoped query semantics; keep graph context additive first |
| [Letta](https://github.com/letta-ai/letta) | Explicit split between core memory blocks and archival memory | Attachable/detachable blocks with `read_only` sharing for multi-agent coordination | Requires clear policy boundaries between always-loaded context and retrieval-only context | Add `core` vs `archival` memory layers in ELF without replacing note storage |
| [LangGraph](https://docs.langchain.com/oss/python/langgraph/persistence) | Threaded checkpoints + replay/fork over persisted state | Deterministic replay model (`thread_id` + checkpoint lineage) for debugging and regression analysis | Replay safety requires idempotent side-effect boundaries | Elevate trace replay and ranking compare to hard regression gates in CI |
| [Graphiti / Zep](https://help.getzep.com/graphiti/core-concepts/temporal-awareness) | Temporal knowledge graph (entities/relations/facts) with explicit validity windows | Invalidate-and-append fact updates (`valid_at`/`invalid_at`) instead of destructive overwrite | Full graph backends add operational complexity and traversal cost | Implement Postgres-first graph-lite with temporal fact validity before introducing graph infra |
| [qmd](https://github.com/tobi/qmd) + [claude-mem](https://github.com/thedotmack/claude-mem) | Retrieval UX and operator workflow focus | Progressive-disclosure search + local inspection/debug loops | Less emphasis on strict deterministic ingestion contracts | Productize ELF debug loop (viewer, status, explain-first inspection) |
| [llm-wiki](https://github.com/nvk/llm-wiki) + [gbrain](https://github.com/garrytan/gbrain) | Compiled knowledge artifacts and maintained knowledge pages | Query-save flows, `compiled_truth` + timeline page shape, backlink/enrichment maintenance, and wiki/brain repair loops | Page-first systems can blur source-of-truth boundaries unless provenance is explicit and rebuildable | Add a derived knowledge-memory layer in ELF with note/doc pointers, recompile rules, and lint/repair loops |
| [Always-On Memory Agent](https://github.com/GoogleCloudPlatform/generative-ai/tree/main/gemini/agents/always-on-memory-agent) + [graphify](https://github.com/safishamsi/graphify) | Background consolidation and graph-compressed operator context | Scheduled consolidation loops, multimodal inbox flow, derived graph/report surfaces, and always-on assistant guidance before raw search | LLM-first consolidation and graph artifacts can drift unless tied back to authoritative evidence and rebuild rules | Add optional consolidation schedulers and derived graph/report surfaces in ELF while keeping Postgres notes/docs authoritative |

## Extended Source Map

- RAGFlow:
  - https://ragflow.io/docs/
  - https://github.com/infiniflow/ragflow/blob/main/docker/README.md
  - https://raw.githubusercontent.com/infiniflow/ragflow/main/docs/references/http_api_reference.md
- LightRAG:
  - https://github.com/HKUDS/LightRAG
  - https://raw.githubusercontent.com/HKUDS/LightRAG/main/docs/DockerDeployment.md
  - https://raw.githubusercontent.com/HKUDS/LightRAG/main/docs/LightRAG-API-Server.md
  - https://raw.githubusercontent.com/HKUDS/LightRAG/main/docs/ProgramingWithCore.md
- GraphRAG:
  - https://microsoft.github.io/graphrag/
  - https://microsoft.github.io/graphrag/index/inputs/
  - https://microsoft.github.io/graphrag/index/outputs/
  - https://microsoft.github.io/graphrag/query/local_search/
- mem0:
  - https://docs.mem0.ai/platform/features/entity-scoped-memory
  - https://docs.mem0.ai/platform/features/graph-memory
  - https://docs.mem0.ai/core-concepts/memory-operations/add
  - https://docs.mem0.ai/open-source/features/async-memory
  - https://docs.mem0.ai/platform/features/advanced-retrieval
  - https://docs.mem0.ai/platform/features/async-mode-default-change
  - https://docs.mem0.ai/platform/features/webhooks
  - https://docs.mem0.ai/open-source/features/custom-update-memory-prompt
  - https://github.com/mem0ai/mem0/blob/main/mem0/memory/main.py
  - https://github.com/mem0ai/mem0/blob/main/mem0/memory/graph_memory.py
- Letta:
  - https://docs.letta.com/concepts/memory/blocks/
  - https://docs.letta.com/concepts/memory/archival-memory/
  - https://docs.letta.com/concepts/memory/shared-memory/
- LangGraph:
  - https://docs.langchain.com/oss/python/langgraph/persistence
  - https://docs.langchain.com/oss/python/langgraph/durable-execution
  - https://docs.langchain.com/oss/python/langgraph/use-time-travel
- Graphiti / Zep:
  - https://help.getzep.com/graphiti/core-concepts/temporal-awareness
  - https://help.getzep.com/graphiti/working-with-data/adding-fact-triples
  - https://help.getzep.com/graphiti/working-with-data/searching-the-graph
- memsearch:
  - https://github.com/zilliztech/memsearch/blob/main/docs/architecture.md
  - https://github.com/zilliztech/memsearch/blob/main/docs/claude-plugin.md
  - https://github.com/zilliztech/memsearch/blob/main/src/memsearch/core.py
  - https://github.com/zilliztech/memsearch/blob/main/src/memsearch/store.py
- OpenViking:
  - https://github.com/volcengine/OpenViking/blob/main/README.md
  - https://github.com/volcengine/OpenViking/blob/main/docs/en/concepts/01-architecture.md
  - https://github.com/volcengine/OpenViking/blob/main/docs/en/concepts/05-storage.md
  - https://github.com/volcengine/OpenViking/blob/main/docs/en/concepts/07-retrieval.md
  - https://github.com/volcengine/OpenViking/blob/main/docs/en/concepts/08-session.md
  - https://github.com/volcengine/OpenViking/blob/main/openviking/storage/viking_fs.py
  - https://github.com/volcengine/OpenViking/blob/main/openviking/retrieve/hierarchical_retriever.py
  - https://github.com/volcengine/OpenViking/blob/main/openviking/service/relation_service.py
  - https://github.com/volcengine/OpenViking/blob/main/pyproject.toml
- qmd / claude-mem:
  - https://github.com/tobi/qmd
  - https://github.com/tobi/qmd/blob/main/src/store.ts
  - https://github.com/tobi/qmd/blob/main/src/llm.ts
  - https://github.com/tobi/qmd/blob/main/src/mcp.ts
  - https://docs.claude-mem.ai/user-guide/progressive-disclosure-search
  - https://docs.claude-mem.ai/user-guide/view-memory
  - https://github.com/thedotmack/claude-mem/blob/main/src/servers/mcp-server.ts
  - https://github.com/thedotmack/claude-mem/blob/main/src/services/worker/http/routes/ViewerRoutes.ts
- llm-wiki:
  - https://github.com/nvk/llm-wiki
  - https://github.com/nvk/llm-wiki/blob/main/README.md
  - https://llm-wiki.net/
  - https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f
- gbrain:
  - https://github.com/garrytan/gbrain
  - https://github.com/garrytan/gbrain/blob/master/README.md
  - https://github.com/garrytan/gbrain/blob/master/docs/ENGINES.md
  - https://github.com/garrytan/gbrain/blob/master/docs/GBRAIN_RECOMMENDED_SCHEMA.md
  - https://github.com/garrytan/gbrain/blob/master/src/schema.sql
  - https://github.com/garrytan/gbrain/blob/master/src/core/search/hybrid.ts
  - https://github.com/garrytan/gbrain/blob/master/src/core/enrichment-service.ts
- Always-On Memory Agent:
  - https://github.com/GoogleCloudPlatform/generative-ai/tree/main/gemini/agents/always-on-memory-agent
  - https://raw.githubusercontent.com/GoogleCloudPlatform/generative-ai/main/gemini/agents/always-on-memory-agent/README.md
- graphify:
  - https://github.com/safishamsi/graphify
  - https://github.com/safishamsi/graphify/blob/v3/README.md
  - https://github.com/safishamsi/graphify/blob/v3/README.zh-CN.md

## ELF Distinctives (Code-Verified)

- Evidence binding with verbatim quote checks.
- Postgres is the source of truth; vector index is fully rebuildable.
- Deterministic `add_note` and LLM-only `add_event` semantics.
- Query expansion modes (`off`, `always`, `dynamic`) for cost/latency control.
- Dedicated evaluation CLI to measure retrieval quality.

## Potential Directions (Reference, Not Commitments)

Expanded research snapshot date for this section: February 17, 2026.

This list is for architectural comparison only. It is not a product commitment and should not be read as a roadmap.

1. Temporal Graph-Lite facts in Postgres
   - Borrow from Graphiti's temporal fact model (`valid_at`/`invalid_at`) and invalidation-overwrite semantics.
   - Add `entities` + `facts` as append-only, evidence-linked rows with temporal windows.
   - Keep graph storage in Postgres first; avoid introducing a graph database in the first iteration.

2. Core memory blocks vs archival memory
   - Borrow from Letta's memory blocks + archival memory split.
   - Add first-class, attachable per-agent memory blocks (for stable identity/instructions) while keeping notes as archival memory.
   - Support read-only shared blocks for multi-agent coordination.

3. First-class memory evolution and history semantics
   - Borrow from mem0's explicit `ADD`/`UPDATE`/`DELETE` event model and history APIs.
   - Standardize update decisions and reasons in the API contract so behavior is auditable and reproducible.

4. Replay-first ranking and regression gates
   - Borrow from LangGraph's checkpoint/replay mindset.
   - Promote trace replay and policy comparison to a CI quality gate to prevent silent retrieval regressions.

5. Developer observability workflow
   - Borrow from qmd/claude-mem operator workflows (viewer + status + logs + troubleshooting loop).
   - Add a lightweight inspection surface and stronger local debugging commands to reduce tuning/debug cycle time.

6. Search mode split and retrieval trajectory
   - Borrow from OpenViking's `find()` vs `search()` separation and staged retrieval flow.
   - Keep quick/planned split and stage-level trajectory outputs in place on `/v2/searches`, then improve operator visibility (`GET /v2/searches/{search_id}` ergonomics and optional local timeline tooling).

7. Unified evidence-to-knowledge memory layer
   - Borrow from llm-wiki's query/save/lint workflow and gbrain's `compiled_truth` + timeline page shape.
   - Add optional derived knowledge-memory pages in ELF (entity pages, concept pages, dossiers, project overviews) that compile from notes/docs and can be rebuilt.
   - Keep notes and evidence pointers authoritative so derived knowledge remains inspectable, invalidatable, and lintable instead of becoming a second hidden source of truth.

8. First-class background consolidation workflow
   - Borrow from Always-On Memory Agent's multimodal inbox, scheduled consolidation pass, and explicit manual consolidation trigger.
   - Add first-class scheduling and operator control surfaces for consolidation/rebuild jobs, while keeping ELF note writes and provenance rules deterministic where required.

9. Graph-compressed navigation over rebuildable derived views
   - Borrow from graphify's deterministic code extraction, explicit confidence/honesty tagging, graph report, and assistant hook surfaces.
   - Add optional graph-derived reports, graph query surfaces, or agent-facing pre-search guidance over ELF notes/docs without treating the graph as a new source of truth.

Current planning surface for these research-backed directions:

- Linear project: [ELF vNext: Evidence-to-Knowledge Memory](https://linear.app/hack-ink/project/elf-vnext-evidence-to-knowledge-memory-d7a9dd3f3e86)
- Active workstreams:
  - [XY-286](https://linear.app/hack-ink/issue/XY-286/knowledge-memory-derived-entityconceptproject-pages-with-provenance) knowledge-memory layer
  - [XY-19](https://linear.app/hack-ink/issue/XY-19/add-a-read-only-web-viewer-for-sessions-and-traces) and [XY-27](https://linear.app/hack-ink/issue/XY-27/viewer-add-retrieval-observability-panels-on-top-of-the-read-only) operator workflow
  - [XY-70](https://linear.app/hack-ink/issue/XY-70/graph-lite-dx-typed-schema-typed-query-nanograph-inspired) graph-lite DX

Research sources for this section:
- Graphiti/Zep:
  - https://help.getzep.com/graphiti/core-concepts/temporal-awareness
  - https://help.getzep.com/graphiti/working-with-data/adding-fact-triples
  - https://help.getzep.com/graphiti/working-with-data/searching-the-graph
- Letta:
  - https://docs.letta.com/concepts/memory/blocks/
  - https://docs.letta.com/concepts/memory/archival-memory/
  - https://docs.letta.com/concepts/memory/shared-memory/
- mem0:
  - https://docs.mem0.ai/platform/features/graph-memory
  - https://docs.mem0.ai/platform/features/entity-scoped-memory
  - https://docs.mem0.ai/open-source/features/custom-update-memory-prompt
- LangGraph:
  - https://docs.langchain.com/oss/python/langgraph/persistence
  - https://docs.langchain.com/oss/python/langgraph/durable-execution
- qmd / claude-mem:
  - https://github.com/tobi/qmd
  - https://docs.claude-mem.ai/user-guide/view-memory
- OpenViking:
  - https://github.com/volcengine/OpenViking/blob/main/README.md
  - https://github.com/volcengine/OpenViking/blob/main/docs/en/concepts/01-architecture.md
  - https://github.com/volcengine/OpenViking/blob/main/docs/en/concepts/07-retrieval.md
- Always-On Memory Agent:
  - https://github.com/GoogleCloudPlatform/generative-ai/tree/main/gemini/agents/always-on-memory-agent
  - https://raw.githubusercontent.com/GoogleCloudPlatform/generative-ai/main/gemini/agents/always-on-memory-agent/README.md
- graphify:
  - https://github.com/safishamsi/graphify
  - https://github.com/safishamsi/graphify/blob/v3/README.md
  - https://github.com/safishamsi/graphify/blob/v3/README.zh-CN.md
