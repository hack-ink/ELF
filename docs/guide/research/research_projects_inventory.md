# External Project Research Inventory

Goal: Maintain a single, auditable inventory of external memory/context projects reviewed for ELF architecture decisions.
Read this when: You need to know which external projects have already been reviewed or still need a deep dive.
Inputs: Existing research notes, open architecture questions, and tracked adoption threads.
Depends on: `docs/guide/research/comparison_external_projects.md`.
Outputs: A current inventory of reviewed and pending external projects.

Last updated: June 10, 2026.

## Legend

- `D2`: Mechanism-level deep dive (docs + code pointers + operational trade-offs).
- `D1`: Docs-level deep dive (architecture/features/scope compared, limited code inspection).
- `D0`: Mention-level only in discussions; not yet deeply reviewed.

## Inventory

| Project | Research depth | Current status | Benchmark dimension role | Why it matters to ELF | Primary reference |
| ------- | -------------- | -------------- | ------------------------ | --------------------- | ----------------- |
| [agentmemory](https://github.com/rohitg00/agentmemory) | D1 | Reviewed | `rw.operator-continuity`, `rw.resume-evidence`, `rw.lifecycle-staleness` | Cross-agent coding-memory hooks, MCP/REST surface, viewer, consolidation lifecycle, and external benchmark target | `docs/guide/research/comparison_external_projects.md`; `docs/research/2026-06-08-agent-memory-selection.json`; `docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json` |
| [OpenAI ChatGPT Memory Dreaming](https://openai.com/index/chatgpt-memory-dreaming/) | D1 | Reviewed | `rw.consolidation-review` | Background memory synthesis and staleness repair as a product direction | `docs/research/2026-06-08-agent-memory-selection.json` |
| [Claude Managed Agents Dreams](https://platform.claude.com/docs/en/managed-agents/dreams) | D1 | Reviewed | `rw.consolidation-review` | Reviewable derived memory-store output over past sessions; strong safety shape for ELF consolidation | `docs/research/2026-06-08-agent-memory-selection.json` |
| [Gemini CLI Auto Memory](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/auto-memory.md) | D1 | Reviewed | `rw.consolidation-review`, `rw.operator-continuity` | Background session mining with project-local review inbox for memory patches and skills | `docs/research/2026-06-08-agent-memory-selection.json` |
| [mem0](https://github.com/mem0ai/mem0) | D2 | Reviewed | `rw.lifecycle-staleness`, `rw.graph-temporal`, `rw.operator-continuity` | Graph memory as additive context, memory history and async mode trade-offs | `docs/guide/research/comparison_external_projects.md`; `docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json` |
| [memsearch](https://github.com/zilliztech/memsearch) | D2 | Reviewed | `rw.lifecycle-staleness`, `rw.retrieval-debug`, `rw.resume-evidence` | Markdown-first SoT + rebuildable index pattern | `docs/guide/research/comparison_external_projects.md`; `docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json` |
| [qmd](https://github.com/tobi/qmd) | D2 | Reviewed | `rw.retrieval-debug`, `rw.lifecycle-staleness`, `rw.resume-evidence` | Retrieval routing, weighted fusion, and local-first explainability | `docs/guide/research/comparison_external_projects.md`; `docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json` |
| [claude-mem](https://github.com/thedotmack/claude-mem) | D2 | Reviewed | `rw.operator-continuity`, `rw.resume-evidence`, `rw.retrieval-debug` | Progressive disclosure and strong operator workflow | `docs/guide/research/comparison_external_projects.md`; `docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json` |
| [OpenViking](https://github.com/volcengine/OpenViking) | D2 | Reviewed | `rw.context-trajectory`, `rw.resume-evidence`, `rw.retrieval-debug` | Filesystem context paradigm, hierarchical retrieval, trajectory observability | `docs/guide/research/comparison_external_projects.md`; `docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json` |
| [llm-wiki](https://github.com/nvk/llm-wiki) | D1 | Reviewed; XY-882 verdict `research_only` | `rw.knowledge-synthesis`, `rw.resume-evidence` | LLM-maintained wiki pattern, topic-scoped knowledge bases, query-save and lint workflows | `docs/guide/research/comparison_external_projects.md`; `docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json`; `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json` |
| [gbrain](https://github.com/garrytan/gbrain) | D1 | Reviewed; XY-882 verdict `blocked` | `rw.knowledge-synthesis`, `rw.operator-continuity` | Operational knowledge brain, `compiled_truth` + timeline pages, enrichment and maintenance loops; blocked on Docker-local brain repo and database proof | `docs/guide/research/comparison_external_projects.md`; `docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json`; `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json` |
| [Always-On Memory Agent](https://github.com/GoogleCloudPlatform/generative-ai/tree/main/gemini/agents/always-on-memory-agent) | D1 | Reviewed | `rw.consolidation-review`, `rw.operator-continuity` | Always-on multimodal ingest + scheduled consolidation loop with simple local ops surface | `docs/guide/research/comparison_external_projects.md`; `docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json` |
| [graphify](https://github.com/safishamsi/graphify) | D1 | Reviewed; XY-882 verdict `adapter_candidate`; XY-889 adds Docker graph/report smoke | `rw.graph-navigation`, `rw.knowledge-synthesis`, `rw.resume-evidence` | Multimodal graph compression, deterministic code extraction, and graph/report outputs with source-file/source-location references; current ELF evidence is a generated-corpus Docker smoke, not broad graph-quality proof | `docs/guide/research/comparison_external_projects.md`; `docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json`; `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json`; `apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json` |
| [Letta](https://github.com/letta-ai/letta) | D1 | Reviewed; XY-882 verdict `research_only` | `rw.core-archival`, `rw.operator-continuity` | Core vs archival memory split, shared blocks; not an implementation candidate until a supported contained server path can export evidence | `docs/guide/research/comparison_external_projects.md`; `docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json`; `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json` |
| [LangGraph](https://docs.langchain.com/oss/python/langgraph/persistence) | D1 | Reviewed; XY-882 verdict `research_only` | `rw.replay-regression`, `rw.resume-evidence` | Checkpoint/replay mindset for quality regression workflows; not a standalone memory backend adapter | `docs/guide/research/comparison_external_projects.md`; `docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json`; `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json` |
| [Graphiti / Zep](https://help.getzep.com/graphiti/core-concepts/temporal-awareness) | D1 | Reviewed; XY-882 verdict `adapter_candidate` | `rw.graph-temporal`, `rw.resume-evidence` | Temporal fact validity model with Docker-local graph-store options and UUID/fact/validity-window output | `docs/guide/research/comparison_external_projects.md`; `docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json`; `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json` |
| [nanograph](https://github.com/nanograph/nanograph) | D1 | Reviewed; XY-882 verdict `research_only` | `rw.graph-temporal`, `rw.retrieval-debug` | Typed schema + typed query ergonomics for graph-lite developer experience; official shape is no server/no Docker | `docs/guide/research/comparison_external_projects.md`; `docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json`; `apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`; `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json` |
| [RAGFlow](https://github.com/infiniflow/ragflow) | D2 feasibility gate | Research gate remains; XY-882 verdict `adapter_candidate` | Candidate `rw.resume-evidence`, `rw.graph-navigation`, `rw.retrieval-debug`; no live strength claim | Docker setup is resource-heavy but documented; API references expose document/chunk evidence handles for a tiny-corpus adapter smoke | `apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`; `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json` |
| [LightRAG](https://github.com/HKUDS/LightRAG) | D2 feasibility gate | Research gate remains; XY-882 verdict `adapter_candidate` | Candidate `rw.graph-navigation`, `rw.graph-temporal`, `rw.retrieval-debug`; no live strength claim | Docker compose path, context-only query modes, and source file-path citation shape support an implementation follow-up | `apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`; `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json` |
| [GraphRAG](https://github.com/microsoft/graphrag) | D2 feasibility gate | Research gate remains; XY-882 verdict `adapter_candidate` | Candidate `rw.graph-navigation`, `rw.knowledge-synthesis`, `rw.retrieval-debug`; no live strength claim | Cost-bounded CLI/API path and parquet output tables expose document, text-unit, and graph-summary handles for evidence mapping | `apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`; `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json` |

## June 10, 2026 Adapter Feasibility Verdicts

XY-882 resolved the D1/D2 feasibility gate for the RAG and graph-memory
`research_gate` records. These verdicts do not change any project into live adapter
evidence by themselves; they only decide whether an implementation follow-up is
justified. XY-900 later promotes graphify's generated-corpus Docker smoke into a
scored tiny `live_real_world` non-pass record, but not broad graph-quality proof.

| Project | Verdict | Follow-up rule |
| ------- | ------- | -------------- |
| RAGFlow | `adapter_candidate` | Follow-up issue: [XY-885](https://linear.app/hack-ink/issue/XY-885/elf-benchmark-adapter-implement-ragflow-docker-evidence-smoke-adapter), a tiny Docker evidence-smoke adapter that records the resource envelope and maps `reference.chunks` to benchmark evidence. |
| LightRAG | `adapter_candidate` | Follow-up issue: [XY-886](https://linear.app/hack-ink/issue/XY-886/elf-benchmark-adapter-implement-lightrag-docker-context-export-adapter), a Docker context-export adapter using explicit LLM/embedding config and source file-path citations. |
| GraphRAG | `adapter_candidate` | Follow-up issue: [XY-887](https://linear.app/hack-ink/issue/XY-887/elf-benchmark-adapter-implement-graphrag-cost-bounded-docker-adapter), a cost-bounded Docker CLI/API adapter over a tiny corpus and parquet output tables. |
| Graphiti / Zep | `adapter_candidate` | Follow-up issue: [XY-888](https://linear.app/hack-ink/issue/XY-888/elf-benchmark-adapter-implement-graphitizep-temporal-graph-adapter), a Docker-local temporal graph adapter that scores current/historical fact validity. |
| graphify | `adapter_candidate` | Follow-up issue: [XY-889](https://linear.app/hack-ink/issue/XY-889/elf-benchmark-adapter-implement-graphify-docker-graph-report-adapter), a Docker-only CLI/materializer adapter over `graph.json` and `GRAPH_REPORT.md`; host-global assistant hooks remain out of scope. XY-900 promotes the checked-in graphify row to a scored tiny Docker smoke with `wrong_result`; it is still not broad graph-navigation quality proof. |
| Letta | `research_only` | Keep as a core/archival memory reference until a supported contained path can export archival-memory evidence for scoring. |
| LangGraph | `research_only` | Keep as a checkpoint/replay regression reference, not a standalone external memory adapter. |
| nanograph | `research_only` | Keep as typed graph DX inspiration; official shape is no server/no Docker. |
| llm-wiki | `research_only` | Keep as a derived knowledge-page workflow reference; host-global plugin installs are not adapter proof. |
| gbrain | `blocked` | Revisit only after a Docker-local brain repo and database path can be proven without operator-owned state. |

## June 2026 Activity Snapshot

GitHub API snapshot time: 2026-06-08T06:01:57Z.

The monitored project set is still moving quickly. Recent push activity was observed for
agentmemory, mem0, qmd, claude-mem, OpenViking, gbrain, graphify, LangGraph, Graphiti,
RAGFlow, LightRAG, and GraphRAG. Notable current scale signals:

- agentmemory: 21,783 stars, latest release `v0.9.27`, pushed 2026-06-07.
- mem0: 58,005 stars, latest release `cli-node-v0.2.8`, pushed 2026-06-06.
- claude-mem: 81,157 stars, latest release `v13.4.1`, pushed 2026-06-08.
- graphify: 62,294 stars, latest release `v0.8.35`, pushed 2026-06-07.
- RAGFlow: 82,150 stars, latest release `v0.25.6`, pushed 2026-06-08.
- LightRAG: 36,270 stars, latest release `v1.5.0`, pushed 2026-06-08.
- GraphRAG: 33,545 stars, latest release `v3.1.0`, pushed 2026-06-05.

Interpretation: this is not a settled market. ELF should keep watching external
implementation velocity, but the current activity signal alone does not justify
replacing ELF's evidence-bound service contract.

## Current Planning Surface

- Linear project: [ELF vNext: Evidence-to-Knowledge Memory](https://linear.app/hack-ink/project/elf-vnext-evidence-to-knowledge-memory-d7a9dd3f3e86)
- Active workstreams:
  - [XY-286](https://linear.app/hack-ink/issue/XY-286/knowledge-memory-derived-entityconceptproject-pages-with-provenance) knowledge-memory layer
  - [XY-19](https://linear.app/hack-ink/issue/XY-19/add-a-read-only-web-viewer-for-sessions-and-traces) and [XY-27](https://linear.app/hack-ink/issue/XY-27/viewer-add-retrieval-observability-panels-on-top-of-the-read-only) operator workflow
  - [XY-70](https://linear.app/hack-ink/issue/XY-70/graph-lite-dx-typed-schema-typed-query-nanograph-inspired) graph-lite DX
- Historical research/foundation issues now closed:
  - [XY-40](https://linear.app/hack-ink/issue/XY-40/vision-track-elf-as-a-high-trust-memory-system-for-singlemulti-agent)
  - [XY-51](https://linear.app/hack-ink/issue/XY-51/agent-memory-ux-mcp-surface-skills-doc-pointers-epic)
  - [XY-63](https://linear.app/hack-ink/issue/XY-63/research-openviking-as-optional-doc-backend-integration-sketch)
- Current June 2026 research runs:
  - `docs/research/2026-06-08-agent-memory-selection.json`
  - `docs/research/2026-06-09-xy-841-external-memory-benchmark-dimensions.json`
  - `docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json`

## Notes

- This inventory tracks research state, not implementation commitment.
- Any architecture change must still pass code-level feasibility and regression validation in ELF.
