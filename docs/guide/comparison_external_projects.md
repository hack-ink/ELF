# External Memory Project Comparison

Purpose: Provide a detailed, evidence-backed comparison between ELF and adjacent memory projects.

Scope note: This document is intentionally detailed and source-heavy. Keep `README.md` concise and link here for full analysis.

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
| First-class graph memory mode                | —   | —         | —   | —          | ✅ (optional) |
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
- mem0 graph memory is optional and requires an OpenAI-compatible LLM setup ([source](https://docs.mem0.ai/platform/features/graph-memory)).
- mem0 search docs describe optional reranking, query optimization, and keyword-search toggles ([source](https://docs.mem0.ai/platform/features/search-filters)).
- mem0 lifecycle docs describe `expiration_date` and automatic exclusion of expired memories from retrieval ([source](https://docs.mem0.ai/cookbooks/essentials/memory-expiration-short-and-long-term)).
- claude-mem supports `<private>` tags to exclude selected content from storage ([source](https://github.com/thedotmack/claude-mem?tab=readme-ov-file#memory-privacy-controls)).

## Project Strengths And Trade-offs

- [memsearch](https://github.com/zilliztech/memsearch): Strong Markdown-first transparency, smart dedup, and live file-watch sync. Trade-off: integration is centered on plugin/CLI workflows rather than a general MCP + HTTP service surface.
- [qmd](https://github.com/tobi/qmd): Strong local-first retrieval quality (BM25 + vector + rerank + query expansion) with practical CLI and MCP tooling. Trade-off: focused on document retrieval workflows more than memory-specific safety/lifecycle semantics.
- [claude-mem](https://github.com/thedotmack/claude-mem): Strong automatic capture and progressive disclosure UX, plus a practical local web viewer for inspection. Trade-off: optimized for Claude session continuity, with fewer explicit deterministic ingestion boundaries.
- [mem0](https://github.com/mem0ai/mem0): Strong ecosystem reach (SDK + hosted + OpenMemory), multi-entity scoping, and lifecycle controls like `expiration_date`. Trade-off: ingestion and retrieval behavior depends heavily on configurable LLM-assisted flows, which can be less deterministic by default.

## Mechanism-Level Deep Dive (Beyond README)

Snapshot date for this subsection: February 17, 2026.

| Project | Ingestion and update semantics | Retrieval internals | Consistency and reliability model | Operational profile |
| ------- | ------------------------------ | ------------------- | --------------------------------- | ------------------- |
| [mem0](https://github.com/mem0ai/mem0) | `add()` can run LLM-guided `ADD/UPDATE/DELETE/NONE`; history events are persisted; optional graph extraction runs alongside vector memory | Dense retrieval is core; rerank/filter are optional; graph mode adds relation retrieval as an extra context channel | OSS sync mode waits for processing completion; Platform API is async-by-default with event queue semantics | Rich hosted + OSS surface; stronger built-in feedback/events, but more tuning knobs and potential latency/cost variance |
| [memsearch](https://github.com/zilliztech/memsearch) | Markdown is canonical; reindex is incremental/content-addressed; stale chunks are removed by hash-based reconciliation | Milvus hybrid search (dense + BM25 sparse) with RRF fusion | Plugin hook workflow favors practical continuity; failures are mostly handled operationally rather than through strict policy contracts | Very pragmatic local workflow; Milvus Lite/Server/Cloud flexibility, but capability envelope depends on Milvus mode |
| [qmd](https://github.com/tobi/qmd) | Content-addressed SQLite model; `qmd update` reactivates/upserts and deactivates missing documents | Typed query expansion (`lex/vec/hyde`), hybrid routing, weighted RRF, then rerank blend by rank bands | Strong deterministic local index behavior with schema self-healing for vector tables | Excellent local-first control and explainability; less focused on multi-tenant memory governance semantics |
| [claude-mem](https://github.com/thedotmack/claude-mem) | Hook-driven capture tied to Claude Code lifecycle; queue-backed worker persists pending tasks | Progressive-disclosure retrieval is explicit (`search -> timeline -> get_observations`); hybrid local stack (SQLite + Chroma) | Deliberate fail-open handler behavior reduces workflow interruption but may accept occasional capture gaps | Best-in-class local operator ergonomics (viewer/SSE/logs), centered on Claude-centric usage patterns |

Key takeaways for ELF from this deeper pass:

- mem0 demonstrates that graph context can be additive instead of replacing vector retrieval.
- qmd shows retrieval quality gains from explicit routing heuristics and transparent score fusion.
- memsearch validates a strong pattern: canonical primary store + rebuildable derived index.
- claude-mem demonstrates how much adoption improves when operator inspection is first-class.

## Where ELF Is Currently Weaker (Objective Gaps)

- No built-in web UI viewer yet (claude-mem and OpenMemory provide this today).
- No hosted/cloud product option (mem0 provides managed deployment).
- No first-class graph memory in released schema yet (mem0 provides optional graph mode now).
- Less turnkey for zero-config local plugin workflows than memsearch/claude-mem defaults.

## Extended Deep-Dive Comparison (Reference Only)

Snapshot date for this subsection: February 17, 2026.

| Project | Distinct memory model | High-value mechanism | Known trade-off | Optional takeaway for ELF |
| ------- | --------------------- | -------------------- | --------------- | -------------------------- |
| [mem0](https://github.com/mem0ai/mem0) | Entity-scoped memories (`user_id`/`agent_id`/`app_id`/`run_id`) with optional graph augmentation | Async ingestion + webhooks, explicit memory history events, optional graph relations context | Async default introduces read-after-write complexity; graph path adds cost and provider coupling | Add first-class memory update events and stronger entity-scoped query semantics; keep graph context additive first |
| [Letta](https://github.com/letta-ai/letta) | Explicit split between core memory blocks and archival memory | Attachable/detachable blocks with `read_only` sharing for multi-agent coordination | Requires clear policy boundaries between always-loaded context and retrieval-only context | Add `core` vs `archival` memory layers in ELF without replacing note storage |
| [LangGraph](https://docs.langchain.com/oss/python/langgraph/persistence) | Threaded checkpoints + replay/fork over persisted state | Deterministic replay model (`thread_id` + checkpoint lineage) for debugging and regression analysis | Replay safety requires idempotent side-effect boundaries | Elevate trace replay and ranking compare to hard regression gates in CI |
| [Graphiti / Zep](https://help.getzep.com/graphiti/core-concepts/temporal-awareness) | Temporal knowledge graph (entities/relations/facts) with explicit validity windows | Invalidate-and-append fact updates (`valid_at`/`invalid_at`) instead of destructive overwrite | Full graph backends add operational complexity and traversal cost | Implement Postgres-first graph-lite with temporal fact validity before introducing graph infra |
| [qmd](https://github.com/tobi/qmd) + [claude-mem](https://github.com/thedotmack/claude-mem) | Retrieval UX and operator workflow focus | Progressive-disclosure search + local inspection/debug loops | Less emphasis on strict deterministic ingestion contracts | Productize ELF debug loop (viewer, status, explain-first inspection) |

## Extended Source Map

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
- qmd / claude-mem:
  - https://github.com/tobi/qmd
  - https://github.com/tobi/qmd/blob/main/src/store.ts
  - https://github.com/tobi/qmd/blob/main/src/llm.ts
  - https://github.com/tobi/qmd/blob/main/src/mcp.ts
  - https://docs.claude-mem.ai/user-guide/progressive-disclosure-search
  - https://docs.claude-mem.ai/user-guide/view-memory
  - https://github.com/thedotmack/claude-mem/blob/main/src/servers/mcp-server.ts
  - https://github.com/thedotmack/claude-mem/blob/main/src/services/worker/http/routes/ViewerRoutes.ts

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

