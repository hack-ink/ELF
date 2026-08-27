---
type: Reference
title: "Source-Backed Memory Knowledge Roadmap"
description: "Retained research-backed plan for ELF's source-backed project memory, knowledge workspace, work journal, dreaming, model ladder, competitor absorption, and benchmark roadmap."
resource: docs/reference/plans/2026-07-03-source-backed-memory-knowledge-roadmap.md
status: active
authority: current_state
owner: reference
last_verified: 2026-07-04
tags:
  - docs
  - reference
  - plans
  - agent-memory
  - project-memory
  - knowledge
source_refs:
  - https://developers.openai.com/api/docs/guides/model-selection
  - https://developers.openai.com/api/docs/guides/reasoning-best-practices
  - https://langchain-ai.github.io/langmem/
  - https://docs.langchain.com/oss/python/concepts/memory
  - https://www.letta.com/blog/sleep-time-compute/
  - https://docs.mem0.ai/cookbooks/companions/local-companion-ollama
  - https://help.getzep.com/graphiti/configuration/llm-configuration
  - https://github.com/tobi/qmd
  - https://github.com/VectifyAI/PageIndex
  - https://github.com/VectifyAI/OpenKB
  - https://github.com/plastic-labs/honcho
  - https://github.com/infiniflow/ragflow
  - https://microsoft.github.io/graphrag/
  - https://github.com/HKUDS/LightRAG
  - https://docs.docker.com/dhi/core-concepts/digests/
related:
  - docs/spec/agent_memory_knowledge_system_v1.md
  - docs/spec/system_context_pack_v1.md
  - docs/spec/system_work_journal_v1.md
  - docs/spec/system_memory_summary_v1.md
  - docs/spec/system_knowledge_pages_v1.md
  - docs/spec/system_recall_debug_panel_v1.md
  - docs/evidence/benchmarking/2026-07-03-final-source-backed-project-memory-closeout-report.md
  - docs/evidence/benchmarking/2026-06-27-public-quantitative-competitor-scoreboard-report.md
---
# Source-Backed Memory Knowledge Roadmap

Purpose: Retain the current research-backed execution plan for ELF after competitor
absorption, small-model review, diary/work-journal review, dreaming review, and
skeptic challenge.

Status: retained planning artifact. Promote accepted invariants into `docs/spec/`
before treating them as normative.

## Final Product Decision

ELF should continue as open-source, source-backed project memory for AI agents. The
knowledge system is required, but it is a supporting workspace for memory quality,
not a separate generic Knowledge OS.

The user-facing promise is:

> Stop re-explaining your project to AI. ELF turns your project sources into
> traceable knowledge, promotes reliable knowledge into memory, and shows why each
> memory was recalled, updated, or rejected.

The internal product contract is:

> Source Library -> Knowledge Workspace, Work Journal, and Dreaming Review ->
> Memory Authority -> scoped Context Packs -> Recall Engine and Recall Debug.

The differentiation is not raw retrieval alone. The strongest defensible claim is
source-backed memory authority: provenance, lifecycle, correction, rollback,
review, scoped recall, and measurable quality.

## Current Baseline

The July 2026 checked-in baseline already supports the product direction:

- `docs/spec/agent_memory_knowledge_system_v1.md` defines the source-backed project
  memory boundary, supporting capabilities, non-goals, competitor absorption rules,
  metrics, and phase gates.
- `docs/evidence/benchmarking/2026-07-03-final-source-backed-project-memory-closeout-report.md`
  records the current source-backed quality gate with expected evidence recall
  `1.0`, precision@5 `0.414`, source-ref coverage `1.0`, stale suppression `1.0`,
  correction persistence `1.0`, delete/tombstone suppression `1.0`, unsupported
  claim rate `0.0`, Context Pack activation precision/recall `1.0`, and mean
  latency `2.864ms`.
- `docs/evidence/benchmarking/2026-06-27-public-quantitative-competitor-scoreboard-report.md`
  tracks 20 products but explicitly records no comparable product-runtime pass row.
  This prevents broad "ELF beats everyone" claims.

The current gap is not lack of direction. The gap is turning optimization inputs
into executable Decodex slices with same-corpus, source-id-mapped, Docker-contained
benchmark evidence.

The July 4 research pass sharpened three gaps that must remain explicit:

- local model use is technically possible through schema-constrained local runtimes,
  but ELF currently disables local extraction in the default Docker config and has
  no benchmark evidence that local models are safe for authority-changing work;
- Docker benchmark provenance must distinguish immutable image digests from local
  image IDs and must not turn missing held-out or leakage-audit evidence into a
  leaderboard claim;
- Work Journal and Dreaming evidence is strongest when it proves the
  journal-to-proposal-to-promotion boundary, not when it treats diary text as
  current memory.

## Small-Model And Strong-Model Policy

Small or local models are a targeted design goal for bounded work after benchmark
gates pass. The current repository evidence does not yet prove that a small model
can safely run every background organizer job. Treat small-model use as gated by
schema validity, citation coverage, unsupported-claim checks, escalation behavior,
latency, and cost evidence.

The first small-model implementation target should be read-only and proposal-only:
run a local extractor adapter smoke against `ExtractorOutput`-like JSON Schema,
materialize reviewable proposals, and score JSON validity, source quote coverage,
unsupported-claim flags, zero source mutation, review-queue visibility, and
escalation behavior. Do not persist or auto-apply local-model memory changes until
that evidence passes.

Use this ladder:

| Tier | Engine | Jobs | Required gates |
| --- | --- | --- | --- |
| L0 | Deterministic code | source-id mapping, hashing, schema validation, tombstone checks, scope checks, lifecycle filters, diffing | hard fail on schema/scope/lifecycle violations |
| L1 | Sparse/vector retrieval | BM25, embeddings, candidate generation, source neighborhood selection | recall, precision, latency, cost, and stale-candidate metrics |
| L2 | Small/local model | classification, routing, tagging, structured extraction drafts, diary compaction, summary drafts, query expansion, rerank explanation drafts | JSON-schema validity, citation coverage, unsupported-claim lint, confidence thresholds, replay tests |
| L3 | Strong reasoning model | conflict adjudication, multi-source synthesis, policy-sensitive promotion proposals, contradiction resolution, benchmark judging, roadmap synthesis | independent review, source coverage, disagreement tracking, no silent authority mutation |
| L4 | Human/operator or explicit policy approval | irreversible deletion policy, public claims, private/provider-backed corpus claims, product-scope decisions | audit log and explicit acceptance |

This means background organization should be designed so cheap models can handle
repeatable low-risk drafting after they pass evals. Strong models are reserved for
hard synthesis, ambiguous evidence, and final review gates. The right product shape
is not "use a small model everywhere"; it is "small model drafts, strong model or
policy gates promote."

## Latency And Cost Strategy

Hot-path recall must stay fast:

- Retrieve with L0/L1 first.
- Use precomputed Knowledge Workspace pages, Work Journal summaries, graph-lite
  facts, and Context Pack manifests.
- Run L2 model routing only when deterministic routing confidence is low.
- Keep L3 dreaming/review work asynchronous and batchable.

The expected user experience is conversational: the user asks the agent normally,
and ELF internally routes memory and knowledge scopes. The user should not manually
choose packs. `Context Pack` is an internal, traceable transport artifact, not a
manual user workflow.

## Diary And Work Journal Position

Diary-like memory is valuable, but it should be implemented as Work Journal plus
Dreaming Review rather than as authoritative memory.

The diary layer should capture:

- session summaries;
- decisions made and decisions rejected;
- next steps;
- blockers;
- files/issues/sources touched;
- agent uncertainty and assumptions;
- handoff notes;
- cleanup/janitor observations.

The diary layer must not directly become current memory. It becomes evidence that
can support promotion proposals. This absorbs the "agent should write a diary"
idea without weakening source-backed authority.

## Dreaming Design

Dreaming is asynchronous, reviewable background organization.

Triggers:

- conversation or task end;
- source import;
- source update or deletion;
- stale-memory detection;
- repeated recall misses;
- scheduled daily/weekly maintenance;
- benchmark failure;
- explicit "organize this project" command.

Pipeline:

1. L0 validates source refs, lifecycle state, scopes, and schema.
2. L1 retrieves related memory, sources, pages, journal rows, and traces.
3. L2 drafts proposals: tags, page deltas, summary updates, candidate memories,
   duplicate clusters, stale-memory warnings, route hints, and benchmark fixtures.
4. L0 validators reject malformed, uncited, cross-scope, stale, or unsupported
   proposals.
5. L3 reviews only proposals that need synthesis, conflict handling, or authority
   promotion.
6. Accepted low-risk derived organization may update derived pages or indexes.
7. Memory Authority changes require explicit policy acceptance and audit history.

This should make non-frontier models useful because the harness constrains the job,
but it still keeps strong-model review available where model mistakes would damage
memory quality.

## Scopes And Automatic Activation

Both knowledge and memory need scope management. Scopes should be automatically
activated by the router rather than manually selected by users.

Required scope dimensions:

- tenant and project;
- repository, package, module, or directory;
- issue, PR, task, incident, milestone, or decision;
- source collection such as docs, chats, Twitter/X captures, design notes, code
  references, tickets, or papers;
- entity such as person, team, product, component, customer, competitor, model, or
  dependency;
- time window and lifecycle state;
- trust tier and privacy boundary;
- authority layer: source, derived page, journal, proposal, memory, graph, trace.

Automatic Context Routing should produce:

- selected scopes;
- suppressed scopes;
- confidence and rationale;
- budget allocation;
- source/memory/page/journal/proposal counts;
- stale and blocked candidates;
- privacy-safe recall debug output.

Manual overrides should exist for debugging and pinning, but the normal agent path
must remain automatic.

## Knowledge Workspace Role

The Knowledge Workspace is the bridge between raw source capture and reliable
memory. It should support:

- source-linked project, entity, concept, issue, decision, author, and timeline
  pages;
- tree and outline views for long documents;
- cross-links between concepts and source refs;
- stale-source lint;
- watch/rebuild workflows;
- version diffs;
- candidate memory extraction from changed pages;
- readback through MCP/HTTP for agents.

The workspace must preserve the derived-only boundary. A page can explain and
organize; it cannot silently become current memory.

## Competitor Absorption Matrix

| Product/reference | Strength to absorb | ELF implementation target | Benchmark gate | Claim boundary |
| --- | --- | --- | --- | --- |
| qmd | Local-first hybrid search, BM25 + vector + local rerank, compact top-k/replay ergonomics | one-command recall replay, top-k candidate JSON, dense/sparse/fusion/rerank attribution, dropped-candidate explanations | same-corpus qmd candidate replay with source-id mapping, held-out/leakage evidence, digest, product commit | qmd keeps debug/replay edge until comparable ELF artifacts exist |
| PageIndex | Long-document tree navigation and traceable reasoning over document hierarchy | Source Library tree artifacts, cited node paths, traversal traces, long-doc route hints | PageIndex same-corpus adapter emits tree artifacts mapped to ELF source ids | no win/tie/loss until product-runtime tree output is comparable |
| OpenKB | Compiled wiki, concept/entity pages, cross-links, watch/recompile | Knowledge Workspace pages, stale lint, version diffs, source-linked wiki exports | OpenKB same-corpus adapter emits wiki/index/lint/watch artifacts | derived wiki pages are not Memory Authority |
| mem0/OpenMemory | Drop-in agent memory, entity-scoped history, ADD/UPDATE/DELETE history, local and hosted ecosystem | memory history API, entity-scoped recall, import/export, local SDK recipes, delete audit | local SDK same-corpus history/export and separate OpenMemory UI/export product rows | hosted/UI/export claims require product evidence |
| Letta | Core/archive split, stateful agents, archival search, sleep-time compute | core project memory blocks, archival source search, sleep-time Dreaming jobs, export/readback mapping | contained Letta export/readback with source ids, core/archive fixtures | no core/archive parity until export/readback maps to ELF ids |
| Graphiti/Zep | Temporal graph validity, invalidating old facts, vector/full-text/graph retrieval | bitemporal graph-lite facts, valid-from/valid-to, supersession, graph recall labels | temporal-validity adapter with contradiction/staleness cases and source ids | graph-lite evidence is not broad graph/RAG parity |
| RAGFlow | Deep document understanding, complex PDFs/layout/tables, citation-backed RAG workflows | document parsing adapters, citation navigation, layout/table provenance, ingestion quality gates | Docker adapter with document ids, chunk ids, metadata, stale-source outputs | no document-understanding parity without product-runtime outputs |
| GraphRAG | Entity/relationship extraction, community summaries, global/local graph retrieval | graph reports, community/topic summaries, local/global scope routing | GraphRAG adapter with documents, text units, communities, reports, entities, relations | no broad graph superiority from ELF-native graph-lite only |
| LightRAG | Lightweight graph + vector retrieval, local/global/hybrid/mix modes, API/UI | efficient graph/vector hybrid recall, mode-specific traces, KG export comparison | LightRAG adapter emits context, file paths, snippets, source refs, answer checks | incomplete/not-encoded rows stay non-pass |
| Honcho | Stateful agents over people/agents/groups/projects/ideas, MCP integration, peer modeling | project/person/team/entity memory views, relationship-aware recall, Codex/MCP recipes | Honcho same-corpus adapter for project/person/task continuity and memory evolution | currently source provenance only; no product-runtime score |
| OpenViking | Filesystem-like context URIs, hierarchy selection, staged trajectory, recursive expansion | staged Context Pack trajectory, hierarchy-aware routing, recursive expansion trace, rejected-sibling evidence | same-corpus context-trajectory adapter with staged artifacts and hierarchy output | no trajectory parity until staged artifacts exist |
| agentmemory | Capture hooks, local viewer, coding-agent continuity | source capture hooks, local operator readback, work-resume trace, capture audit | lifecycle/capture/work-resume benchmark rows | capture convenience cannot bypass write policy |
| claude-mem | Durable local repository memory and progressive disclosure | portable memory summary export, progressive context disclosure, repo-local handoff readback | local continuity fixtures only where not Claude-specific | Claude-only runtime adapters should be removed from Codex/ELF candidates |
| memsearch | Markdown canonical store, incremental reindex, local hybrid retrieval | exportable Markdown memory snapshots, deterministic reindex, local hybrid recall diagnostics | same-corpus retrieval/evolution/trust rows | ELF source of truth remains Postgres with typed source refs |
| LangGraph | persistence and work-resume graph patterns | durable agent-state recipes, checkpoint-linked context handoff | persistence/work-resume benchmark rows | not encoded as product-runtime parity yet |
| graphify | Docker graph-report generation | graph report export and source-linked visualization evidence | product-runtime graph report rows with digest evidence | current tracked row has wrong-result/blocker evidence |
| llm-wiki/gbrain/nanograph | knowledge compilation, query-save/lint loops, lightweight graph memory ideas | derived page quality, saved-query lint, graph-lite drift tests | typed research-gate rows before scoring | not encoded or blocked rows remain non-pass |
| Claude/OpenClaw memory files | Simple durable project memory files and just-in-time retrieval | exportable compact memory summaries and Work Journal handoffs | continuity benchmark: reset/resume, rejected option suppression, next-step precision | Claude-specific integrations should be dropped from candidate runtime adapters when not usable by Codex/ELF |
| LangMem/LangChain | semantic/episodic/procedural memory managers and background formation | typed memory categories, prompt/procedure proposal review, memory manager recipes | extraction/update/delete/consolidation evals by category | memory manager pattern only; not proof of quality |
| OpenAI Memory/Dreaming | background memory curation, saved memories, chat-history reference, user controls | Dreaming Review queue, memory summary, source links, delete/archive/disable controls | dreaming proposal acceptance, unsupported claim, stale update, deletion, traceability | ELF should be more auditable than opaque background memory |
| diary/work-journal style systems | session continuity, rejected options, blockers, next steps, assumptions, touched files, and handoff notes | Work Journal capture plus Dreaming proposal generation from journal evidence | journal-to-dreaming-promotion-boundary scenario with source refs, no note/index mutation before approval, queue visibility, unsupported-claim lint, and promotion provenance | diary text is continuity evidence, not current Memory Authority |

## Implementation Roadmap

### R0: Preserve accepted baseline

Keep the July 2026 source-backed project memory contract as the baseline. Do not
reopen generic Knowledge OS positioning.

Done when:

- README, docs index, specs, and evidence language use the same product position.
- Existing benchmark reports keep typed non-pass states visible.

### R1: Model ladder and background organizer harness

Add explicit model profiles and job classes for L0/L1/L2/L3/L4.

Implementation targets:

- `model_profile` config for extraction, routing, summarization, judging, and
  strong review.
- structured output validators for L2 proposals;
- escalation policy from L2 to L3;
- failure classes for schema invalidity, unsupported claim, source-ref miss,
  contradiction miss, stale miss, and cross-scope leak.

Benchmark targets:

- small-model extraction F1;
- JSON validity rate;
- citation coverage;
- unsupported claim rate;
- stale, correction, and delete/tombstone behavior;
- zero source mutation and zero silent Memory Authority mutation;
- escalation rate;
- cost and latency by tier.

### R2: Automatic scope router and Context Pack quality

Make automatic scoping the default agent path.

Implementation targets:

- scope classifier over project/repo/task/source/entity/time/trust layers;
- automatic pack assembly with no required manual pack selection;
- route traces and suppression reasons;
- debug-only pin/disable overrides.

Benchmark targets:

- activation precision;
- activation recall;
- wrong-scope activation count;
- cross-scope leak count;
- stale-source inclusion count;
- hot-path latency.

### R3: Reproducible benchmark substrate

Make competitor work reproducible before claiming adapter parity.

Implementation targets:

- Docker Compose runner for ELF and comparable adapters;
- immutable container image digest capture, with local image IDs recorded only as
  weaker local provenance when no registry digest exists;
- product commit capture;
- held-out corpus manifest;
- leakage audit;
- same-corpus source-id mapping;
- bounded public report generation.

Benchmark targets:

- product-runtime pass rows;
- competitor row-level metrics;
- typed blocker preservation;
- no unqualified leaderboard claims.

### R4: qmd-style recall replay and hybrid retrieval

Close the main ergonomics gap against qmd.

Implementation targets:

- compact top-k JSON;
- one-command replay lines;
- dense/sparse/fusion/rerank attribution;
- dropped evidence and selected-but-not-narrated evidence;
- local-first mode for deterministic recall diagnostics.

Benchmark targets:

- recall@5, precision@5, MRR, nDCG;
- replay consistency;
- qmd candidate replay comparability gate;
- latency and zero-provider local run evidence.

### R5: Knowledge Workspace tree/wiki adapters

Absorb PageIndex and OpenKB without becoming their clone.

Implementation targets:

- tree artifacts for long docs;
- source-linked wiki/page exports;
- concept/entity/decision/timeline pages;
- watch/rebuild and stale lint;
- page-delta to memory-candidate proposals.

Benchmark targets:

- source-id mapped PageIndex tree output;
- source-id mapped OpenKB wiki/index/lint/watch output;
- citation coverage;
- stale-source lint correctness;
- page rebuild diff correctness.

### R6: Memory history, core/archive, and entity views

Absorb mem0/OpenMemory, Letta, and Honcho strengths.

Implementation targets:

- entity-scoped memory history;
- ADD/UPDATE/DELETE audit readback;
- export/restore format;
- core project memory blocks;
- archival source search;
- person/team/project/agent relationship views.

Benchmark targets:

- correction persistence;
- delete/tombstone suppression;
- entity recall quality;
- core/archive export/readback mapping;
- work-continuity reset/resume accuracy.

### R7: Temporal graph and graph/RAG adapters

Add graph strength where it improves memory truth.

Implementation targets:

- valid-from/valid-to relation facts;
- superseded and stale relation labels;
- local/global graph recall modes;
- community/topic reports;
- citation navigation for graph-derived context.

Benchmark targets:

- stale graph fact suppression;
- temporal contradiction resolution;
- graph citation coverage;
- Graphiti/Zep, GraphRAG, LightRAG, and RAGFlow adapter rows with typed outcomes.

### R8: Work Journal and Dreaming automation

Turn diary and background organization into production-grade loops.

Implementation targets:

- automatic session journal capture;
- next-step, decision, rejected-option, blocker, and touched-source extraction;
- daily/weekly dreaming batches;
- proposal review queue;
- low-risk derived-page auto-organization;
- no silent Memory Authority mutation.

Benchmark targets:

- resume accuracy;
- decision-rationale recall;
- rejected-option suppression;
- journal-to-dreaming proposal boundary;
- journal-only authority claim count;
- proposal acceptance rate;
- unsupported dreaming claim rate;
- source mutation count.

### R9: Operator UI, export, and privacy controls

Build the UI as an operator console, not the primary product surface.

Implementation targets:

- source review;
- Memory Inbox and Memory Ledger;
- Knowledge Workspace page browser;
- Work Journal timeline;
- Dreaming Review queue;
- Recall Debug panel;
- export/delete/restore controls;
- privacy and scope labels.

Benchmark targets:

- export/readback completeness;
- delete/archive/restore correctness;
- UI/export product evidence for OpenMemory-style comparisons;
- privacy leak counters.

## Benchmark Metrics

Required metrics:

- expected evidence recall;
- precision@k;
- MRR;
- nDCG;
- source-ref coverage;
- citation coverage;
- stale suppression rate;
- update correctness;
- correction persistence;
- delete/tombstone suppression rate;
- unsupported claim rate;
- uncertainty handling;
- trap avoidance;
- cross-scope leak count;
- journal-only authority claim count;
- Context Pack activation precision and recall;
- route trace coverage;
- mean and p95 latency;
- cost by model tier;
- resource footprint;
- proposal acceptance rate;
- small-model escalation rate;
- product-runtime digest and commit provenance.

Product-specific metrics may add:

- qmd replay consistency;
- PageIndex tree-node citation coverage;
- OpenKB wiki rebuild diff correctness;
- mem0 history event correctness;
- Letta core/archive readback coverage;
- Graphiti/Zep temporal invalidation correctness;
- RAGFlow layout/table provenance coverage;
- GraphRAG community summary grounding;
- LightRAG local/global/hybrid mode correctness;
- Honcho project/person continuity quality.

## Decodex Execution Plan

Queue only one accepted slice at a time:

1. README/product-position drift cleanup and plan promotion.
2. R1 model ladder and background organizer benchmark.
3. R2 automatic scope router and Context Pack quality.
4. R3 reproducible benchmark substrate.
5. R4 qmd replay parity.
6. R5 PageIndex/OpenKB tree/wiki adapters.
7. R6 memory history/core/archive/entity views.
8. R7 temporal graph and graph/RAG adapters.
9. R8 Work Journal and Dreaming automation.
10. R9 operator UI/export/privacy.

Every Decodex issue must include:

- owning spec or plan reference;
- exact files/modules in scope;
- required benchmark task;
- allowed and disallowed claims;
- competitor rows touched;
- review requirements;
- no broad refactor unless it directly improves the slice's measured quality.

## Claim Rules

Allowed now:

- ELF is open-source, source-backed project memory for AI agents.
- ELF has strong checked-in evidence for source-backed memory authority, lifecycle,
  recall debug, Context Pack routing, and measured source-backed quality slices.
- Priority tracked competitor strengths are mapped to optimization lanes, while
  unencoded or blocked scoreboard rows remain explicit future evidence targets.

Not allowed now:

- ELF beats every competitor.
- ELF is the best generic Knowledge OS.
- ELF has product-runtime parity with qmd, PageIndex, OpenKB, mem0/OpenMemory,
  Letta, Graphiti/Zep, Honcho, RAGFlow, GraphRAG, or LightRAG.
- Fixture-backed, blocked, wrong-result, incomplete, or not-encoded rows are wins.

## Completion Criteria

This plan is implementation-ready only after:

- docs validation passes;
- skeptic challenge P0/P1 objections are resolved or recorded as blockers;
- README positioning drift is removed;
- any accepted changes are promoted to the owning spec before being treated as
  normative;
- Decodex issues are queued one phase at a time with benchmark and review gates.
