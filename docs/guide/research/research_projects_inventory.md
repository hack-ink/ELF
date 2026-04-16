# External Project Research Inventory

Goal: Maintain a single, auditable inventory of external memory/context projects reviewed for ELF architecture decisions.
Read this when: You need to know which external projects have already been reviewed or still need a deep dive.
Inputs: Existing research notes, open architecture questions, and tracked adoption threads.
Depends on: `docs/guide/research/comparison_external_projects.md`.
Outputs: A current inventory of reviewed and pending external projects.

Last updated: April 16, 2026.

## Legend

- `D2`: Mechanism-level deep dive (docs + code pointers + operational trade-offs).
- `D1`: Docs-level deep dive (architecture/features/scope compared, limited code inspection).
- `D0`: Mention-level only in discussions; not yet deeply reviewed.

## Inventory

| Project | Research depth | Current status | Why it matters to ELF | Primary reference |
| ------- | -------------- | -------------- | --------------------- | ----------------- |
| [mem0](https://github.com/mem0ai/mem0) | D2 | Reviewed | Graph memory as additive context, memory history and async mode trade-offs | `docs/guide/research/comparison_external_projects.md` |
| [memsearch](https://github.com/zilliztech/memsearch) | D2 | Reviewed | Markdown-first SoT + rebuildable index pattern | `docs/guide/research/comparison_external_projects.md` |
| [qmd](https://github.com/tobi/qmd) | D2 | Reviewed | Retrieval routing, weighted fusion, and local-first explainability | `docs/guide/research/comparison_external_projects.md` |
| [claude-mem](https://github.com/thedotmack/claude-mem) | D2 | Reviewed | Progressive disclosure and strong operator workflow | `docs/guide/research/comparison_external_projects.md` |
| [OpenViking](https://github.com/volcengine/OpenViking) | D2 | Reviewed | Filesystem context paradigm, hierarchical retrieval, trajectory observability | `docs/guide/research/comparison_external_projects.md` |
| [llm-wiki](https://github.com/nvk/llm-wiki) | D1 | Reviewed | LLM-maintained wiki pattern, topic-scoped knowledge bases, query-save and lint workflows | `docs/guide/research/comparison_external_projects.md` |
| [gbrain](https://github.com/garrytan/gbrain) | D1 | Reviewed | Operational knowledge brain, `compiled_truth` + timeline pages, enrichment and maintenance loops | `docs/guide/research/comparison_external_projects.md` |
| [Letta](https://github.com/letta-ai/letta) | D1 | Reviewed | Core vs archival memory split, shared blocks | `docs/guide/research/comparison_external_projects.md` |
| [LangGraph](https://docs.langchain.com/oss/python/langgraph/persistence) | D1 | Reviewed | Checkpoint/replay mindset for quality regression workflows | `docs/guide/research/comparison_external_projects.md` |
| [Graphiti / Zep](https://help.getzep.com/graphiti/core-concepts/temporal-awareness) | D1 | Reviewed | Temporal fact validity model for graph-like memory evolution | `docs/guide/research/comparison_external_projects.md` |
| [nanograph](https://github.com/aaltshuler/nanograph) | D1 | Reviewed | Typed schema + typed query ergonomics for graph-lite developer experience | `docs/guide/research/comparison_external_projects.md` |
| [RAGFlow](https://github.com/infiniflow/ragflow) | D0 | Pending deep dive | Potential framework integration discussion; not yet audited to adoption level | Discussion history only |
| [LightRAG](https://github.com/HKUDS/LightRAG) | D0 | Pending deep dive | Graph-augmented RAG strategy relevance; not yet audited to adoption level | Discussion history only |
| [GraphRAG](https://www.microsoft.com/en-us/research/project/graphrag/) | D0 | Pending deep dive | Graph-based retrieval concepts; not yet audited to implementation decision level | Discussion history only |

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

## Notes

- This inventory tracks research state, not implementation commitment.
- Any architecture change must still pass code-level feasibility and regression validation in ELF.
