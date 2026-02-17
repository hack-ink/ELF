# External Project Research Inventory

Purpose: Maintain a single, auditable inventory of external memory/context projects reviewed for ELF architecture decisions.

Last updated: February 17, 2026.

## Legend

- `D2`: Mechanism-level deep dive (docs + code pointers + operational trade-offs).
- `D1`: Docs-level deep dive (architecture/features/scope compared, limited code inspection).
- `D0`: Mention-level only in discussions; not yet deeply reviewed.

## Inventory

| Project | Research depth | Current status | Why it matters to ELF | Primary reference |
| ------- | -------------- | -------------- | --------------------- | ----------------- |
| [mem0](https://github.com/mem0ai/mem0) | D2 | Reviewed | Graph memory as additive context, memory history and async mode trade-offs | `docs/research/comparison_external_projects.md` |
| [memsearch](https://github.com/zilliztech/memsearch) | D2 | Reviewed | Markdown-first SoT + rebuildable index pattern | `docs/research/comparison_external_projects.md` |
| [qmd](https://github.com/tobi/qmd) | D2 | Reviewed | Retrieval routing, weighted fusion, and local-first explainability | `docs/research/comparison_external_projects.md` |
| [claude-mem](https://github.com/thedotmack/claude-mem) | D2 | Reviewed | Progressive disclosure and strong operator workflow | `docs/research/comparison_external_projects.md` |
| [OpenViking](https://github.com/volcengine/OpenViking) | D2 | Reviewed | Filesystem context paradigm, hierarchical retrieval, trajectory observability | `docs/research/comparison_external_projects.md` |
| [Letta](https://github.com/letta-ai/letta) | D1 | Reviewed | Core vs archival memory split, shared blocks | `docs/research/comparison_external_projects.md` |
| [LangGraph](https://docs.langchain.com/oss/python/langgraph/persistence) | D1 | Reviewed | Checkpoint/replay mindset for quality regression workflows | `docs/research/comparison_external_projects.md` |
| [Graphiti / Zep](https://help.getzep.com/graphiti/core-concepts/temporal-awareness) | D1 | Reviewed | Temporal fact validity model for graph-like memory evolution | `docs/research/comparison_external_projects.md` |
| [RAGFlow](https://github.com/infiniflow/ragflow) | D0 | Pending deep dive | Potential framework integration discussion; not yet audited to adoption level | Discussion history only |
| [LightRAG](https://github.com/HKUDS/LightRAG) | D0 | Pending deep dive | Graph-augmented RAG strategy relevance; not yet audited to adoption level | Discussion history only |
| [GraphRAG](https://www.microsoft.com/en-us/research/project/graphrag/) | D0 | Pending deep dive | Graph-based retrieval concepts; not yet audited to implementation decision level | Discussion history only |

## Adoption Tracks Linked To Research

- OpenViking-inspired track: https://github.com/hack-ink/ELF/issues/57
- Search modes: https://github.com/hack-ink/ELF/issues/58
- Retrieval trajectory explain: https://github.com/hack-ink/ELF/issues/59
- Progressive payload levels: https://github.com/hack-ink/ELF/issues/60
- Scoped recursive retrieval: https://github.com/hack-ink/ELF/issues/61

## Notes

- This inventory tracks research state, not implementation commitment.
- Any architecture change must still pass code-level feasibility and regression validation in ELF.
