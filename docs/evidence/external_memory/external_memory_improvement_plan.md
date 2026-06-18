---
type: Evidence
title: "External Memory Improvement Plan - June 9, 2026"
description: "Convert the June 2026 live benchmark, external memory-system research, and Dexter radar operating pattern into an issue-ready ELF improvement plan."
resource: docs/evidence/external_memory/external_memory_improvement_plan.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-18
tags:
  - docs
  - evidence
  - external_memory
---
# External Memory Improvement Plan - June 9, 2026

Goal: Convert the June 2026 live benchmark, external memory-system research, and Dexter radar operating pattern into an issue-ready ELF improvement plan.
Read this when: Deciding what to implement next before using ELF as a personal production memory system.
Inputs: `README.md`, `docs/evidence/benchmarking/2026-06-09-live-baseline-report.md`, `docs/evidence/external_memory/comparison_external_projects.md`, `docs/evidence/external_memory/research_projects_inventory.md`, current Linear readback, and the local Dexter Pattern Radar automation pattern.
Depends on: `docs/policy.md`, `docs/spec/system_elf_memory_service_v2.md`, and the checked-in live baseline runner.
Outputs: Prioritized gaps, issue queue, parallelization plan, acceptance criteria, and follow-up radar model.

## Summary Judgment

ELF is currently a credible personal-production candidate for an evidence-bound agent memory service, but it should not be treated as fully proven until the P0 items below land.

The objective position is:

- Better than the tested alternatives on evidence-bound writes, deterministic ingestion boundaries, source-of-truth discipline, rebuildable indexing, multi-tenant service shape, and the current encoded Docker benchmark.
- Comparable to the best tested alternative, qmd, on local retrieval quality under the smoke scenario, but ELF has a stronger service/provenance model while qmd has stronger local retrieval-debug ergonomics.
- Behind agentmemory, claude-mem/OpenMemory-style tools, and some managed-memory products on operator UX, visible memory inspection, and turn-by-turn operational comfort.
- Behind Graphiti/Zep, Letta, and mem0-style systems on some broader memory semantics: temporal graph workflows beyond graph-lite relation context, explicit memory history, core-vs-archival blocks, and reviewable memory evolution.
- Not yet proven on large private personal corpus migration, repeated batch backfill, cold-start persistence across every adapter, or long-running unattended production operation.

So the answer is not "ELF is universally better." The current evidence supports "ELF is the better foundation for this repo's desired high-trust, evidence-linked memory system, and it can become the better personal-production choice if the P0 work lands and is benchmarked."

## Evidence Base

### Live Benchmark Evidence

Checked-in report: `docs/evidence/benchmarking/2026-06-09-live-baseline-report.md`.

Current encoded result:

- ELF provider stress run: `live-baseline-20260609010854`, `Qwen3-Embedding-8B`, 4096-dimensional provider embeddings, 480 documents, 16 queries, 8 of 8 encoded checks passing, elapsed 1163 seconds.
- All-project smoke run: `live-baseline-20260609022837`.
- ELF and qmd passed every encoded smoke check.
- agentmemory passed same-corpus retrieval but failed or could not complete lifecycle checks.
- mem0, memsearch, and claude-mem returned wrong same-corpus retrieval results in the encoded smoke.
- OpenViking was incomplete in the June 9 run because its local embedding dependency
  could not complete inside the Docker runner. XY-881 later pinned the Docker path to
  a CPU `llama-cpp-python` wheel and moved the current OpenViking state to
  `wrong_result` when `add_resource`/`find` misses expected evidence terms.

What this proves:

- ELF's current service path can run real provider embeddings through Docker-isolated benchmark scripts.
- ELF's strict provenance/service model does not prevent it from passing the encoded retrieval checks.
- 4096-dimensional provider embeddings are operationally usable for the tested scale.

What this does not prove:

- It does not prove ELF beats every project on all retrieval workloads.
- It does not prove long-running personal production safety.
- It does not prove private-corpus migration quality.
- It does not prove viewer/operator ergonomics are competitive.
- It does not prove every adapter's lifecycle behavior is correctly represented.

### External Project Activity Snapshot

Captured from GitHub API on June 9, 2026. Activity is only a refresh signal, not a quality ranking.

| Project | Stars | Last push | Latest release | Why keep tracking |
| --- | ---: | --- | --- | --- |
| rohitg00/agentmemory | 21969 | 2026-06-08 | v0.9.27 | Coding-agent continuity, packaging, viewer, benchmark claims |
| mem0ai/mem0 | 58095 | 2026-06-09 | cli-node-v0.2.8 | Memory lifecycle, hosted/OpenMemory ecosystem, graph option |
| zilliztech/memsearch | 1948 | 2026-06-01 | v0.4.6 | Markdown-first store and hybrid retrieval ergonomics |
| tobi/qmd | 26294 | 2026-06-08 | v2.5.3 | Strong local retrieval pipeline and transparent debug workflow |
| thedotmack/claude-mem | 81336 | 2026-06-08 | v13.4.1 | Progressive disclosure, auto-capture loop, local viewer |
| volcengine/OpenViking | 25368 | 2026-06-09 | v0.3.24 | Hierarchical context model and staged retrieval trajectory |
| nvk/llm-wiki | 547 | 2026-05-23 | v0.10.2 | Evidence-to-knowledge page compilation |
| garrytan/gbrain | 21723 | 2026-06-08 | none | Human-operable knowledge memory shape |
| GoogleCloudPlatform/generative-ai | 17001 | 2026-06-09 | none | Managed memory/dreaming reference patterns |
| safishamsi/graphify | 63545 | 2026-06-08 | v0.8.36 | Graph-compressed navigation and graph reports |
| nanograph/nanograph | 149 | 2026-05-17 | v1.3.0 | Typed graph ergonomics |
| letta-ai/letta | 23219 | 2026-05-14 | 0.16.8 | Core memory blocks vs archival memory |
| langchain-ai/langgraph | 34219 | 2026-06-07 | 1.2.4 | Replay-first state and regression workflow |
| getzep/graphiti | 27194 | 2026-06-09 | v0.29.2 | Temporal graph memory semantics |
| infiniflow/ragflow | 82243 | 2026-06-09 | v0.25.6 | Full RAG app benchmark reference |
| HKUDS/LightRAG | 36316 | 2026-06-09 | v1.5.0 | Lightweight graph/RAG architecture |
| microsoft/graphrag | 33574 | 2026-06-05 | v3.1.0 | GraphRAG indexing and community reports |
| virattt/dexter | 26927 | 2026-06-03 | v2026.6.3 | Radar operating model and research-worker patterns |

### Failure Semantics

Use these terms in future benchmark reports and Linear issues:

| Term | Meaning | Example |
| --- | --- | --- |
| `pass` | Encoded check completed and returned expected result. | ELF same-corpus retrieval and lifecycle checks pass. |
| `wrong_result` | The system completed but returned an incorrect memory or missed the expected evidence. | mem0/memsearch/claude-mem smoke retrieval mismatch. |
| `lifecycle_fail` | Retrieval may work, but update/delete/cold-start/persistence behavior is wrong or incomplete. | agentmemory adapter passing retrieval but not lifecycle. |
| `incomplete` | The benchmark could not reach the behavioral check due to install/runtime/dependency failure. | A pinned local embedding wheel/import failure before OpenViking `add_resource`/`find`. |
| `not_encoded` | Capability is not currently covered by the benchmark, so no pass/fail claim is allowed. | Viewer quality and batch backfill UX. |
| `blocked` | A safe test cannot run without external credentials, manual setup, or a dependency outside the issue scope. | Private corpus evaluation before sanitized corpus exists. |

## Priority Program

### P0 - Personal Production Readiness

These items decide whether ELF is safe and comfortable enough for single-user production use.

#### P0.1 Batch Ingest and Backfill Throughput

Problem:
The current provider stress result is acceptable for 480 documents, but production adoption needs predictable bulk loading and recovery behavior for a larger personal memory corpus.

Adopt from:

- qmd and memsearch: practical local indexing ergonomics.
- LangGraph-style replay discipline: rerunnable import paths with explicit progress.
- ELF's own outbox/worker architecture.

Implementation shape:

- Add a bulk ingest/backfill command or HTTP job surface that accepts generated or file-backed note batches.
- Use micro-batched embedding requests.
- Add bounded concurrent embedding workers.
- Use durable job rows with checkpointed offsets and retry state.
- Use batch Qdrant upserts.
- Preserve Postgres as source of truth; Qdrant remains rebuildable.
- Expose batch progress and per-stage timing in report artifacts.

Acceptance:

- Docker-only benchmark profile for 480, 2k, and 10k document backfills.
- Backfill can be interrupted and resumed without duplicate source notes.
- Search quality after resume equals a clean run for the same manifest.
- Provider credentials stay in `.env`; no host-global install path is required.

Linear mapping:

- New issue required: `[ELF prod P0] Add resumable batch ingest and backfill benchmark`.
- Parallelizable with P0.2 and P0.4.

#### P0.2 Private Production Corpus Benchmark

Problem:
The generated benchmark is useful but not enough to decide personal production adoption. A sanitized real corpus is needed.

Adopt from:

- agentmemory: coding-agent continuity scenarios.
- qmd: local query/debug workflow.
- LangGraph: replayable regression cases.

Implementation shape:

- Build a private/sanitized corpus manifest for real project memory: issues, PRs, worktrees, runbooks, decisions, and stalled-lane recovery notes.
- Define task-oriented queries: "resume lane", "find prior decision", "explain stale blocker", "recover exact command", "compare project status".
- Include cold-start, update, delete/expiry, and contradictory-memory cases.
- Keep the actual private corpus out of public docs if needed, but commit the manifest schema and synthetic fixtures.

Acceptance:

- Benchmark reports separate public generated corpus from private production corpus.
- Every query has expected evidence ids and allowed alternates.
- Results record precision, wrong-result count, latency, provider, dimensions, and cost proxy.
- Any claim that ELF is production-ready must cite this report.

Linear mapping:

- New issue required: `[ELF prod P0] Add private-corpus production adoption benchmark`.
- Blocks a final "use as personal production memory" decision.

#### P0.3 Single-User Production Runbook and Recovery Contract

Problem:
Docker compose and strict config now exist, but production use needs backup, restore, upgrade, and disaster-recovery instructions.

Adopt from:

- memsearch: simple local store expectations.
- Docker-first deployment discipline from the new live baseline.
- ELF governance: explicit config and source-of-truth boundaries.

Implementation shape:

- Document a single-user production profile using Docker Compose for Postgres, Qdrant, API, worker, and MCP if needed.
- Add backup/restore commands for Postgres.
- Add Qdrant rebuild instructions from Postgres.
- Add health checks, migration checks, and rollback notes.
- Document provider `.env` expectations and what must not be committed.

Acceptance:

- Fresh machine restore proves notes/search work after Postgres restore and Qdrant rebuild.
- Runbook includes exact commands and fail-closed warnings.
- No host-global service install is required.

Linear mapping:

- New issue required: `[ELF prod P0] Add single-user production runbook with backup and restore`.
- Parallelizable with P0.1 after config paths are stable.

#### P0.4 Retrieval Observability and Viewer Follow-Through

Problem:
For daily use, API-only debugging is too slow. ELF now has a base read-only viewer path, but retrieval tuning still needs first-class panels.

Adopt from:

- claude-mem/OpenMemory-style viewer ergonomics.
- qmd transparent expansion/fusion/rerank controls.
- OpenViking staged retrieval trajectory.

Implementation shape:

- Extend the viewer with search session timelines, candidate lists, dense/BM25/fusion/rerank scores, relation context, latency, and provider metadata.
- Add a `GET /v2/searches/{id}` or equivalent trace readback if not already exposed for every panel.
- Keep the viewer read-only for P0.
- Add direct links from benchmark failures to trace ids where possible.

Acceptance:

- A benchmark wrong-result can be debugged from viewer panels without raw database queries.
- The viewer shows which stage dropped or reranked the expected memory.
- Read-only authorization and no-mutation behavior are tested.

Linear mapping:

- Existing: XY-19 base read-only viewer is done.
- Existing follow-up: XY-27 should be prioritized from Backlog to active after P0.1/P0.2 are queued.

#### P0.5 Durable External Adapter and Lifecycle Benchmark Coverage

Problem:
The current all-project smoke found adapter-level ambiguity. It is not enough to say "agentmemory failed" if the adapter uses an in-memory or incomplete lifecycle path.

Adopt from:

- agentmemory: actual durable package behavior and benchmark claims.
- ELF benchmark runner: Docker-isolated reproducibility.

Implementation shape:

- Replace mock/in-memory external adapters with durable local modes where feasible.
- For every external adapter, mark which behaviors are real, mocked, unsupported, or blocked.
- For expanded RAG and graph-memory systems, use `research_gate` records until D1/D2
  research, resource sizing, and Docker runtime boundaries are proven.
- Add lifecycle checks: update, delete/expire, cold-start reload, and same-corpus retrieval.
- Keep failures typed with the terms in this document.
- Use `apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`
  as the real-world adapter coverage contract so fixture-only, live-baseline-only, and
  future live-real-world evidence stay separate.

Acceptance:

- agentmemory adapter either passes durable lifecycle checks or is explicitly marked blocked with evidence.
- OpenViking records a pinned Docker local embedding retry path; install/import
  failure remains `incomplete`, while evidence misses after `add_resource`/`find`
  are `wrong_result`.
- qmd smoke pass remains covered and gains scale/stress profiles.
- Real-world reports include adapter coverage counters before any external adapter is
  allowed to claim a real-world suite pass.

Linear mapping:

- Existing: XY-801 created the initial agentmemory import/baseline boundary and is done.
- New issue required: `[ELF benchmark P0] Make external adapters lifecycle-durable and fail-typed`.

### P1 - Memory Quality and Product Differentiation

These items make ELF not merely usable, but materially better than adjacent memory products for high-trust agent work.

#### P1.1 Reviewable Consolidation Worker

Problem:
ELF has the right evidence-bound source model, but long-term memory quality needs consolidation without hidden mutation.

Adopt from:

- Gemini/managed memory "dreaming" direction, but with explicit review.
- Always-On Memory Agent: background consolidation loop.
- Dexter: proposal-only memo/readback artifacts.

Implementation shape:

- Implement consolidation jobs over immutable notes/events/traces.
- Write derived proposals, not source-note rewrites.
- Include source ids, confidence, unsupported-claim flags, conflicts, and review state.
- Add apply/discard/defer transitions.

Acceptance:

- Every proposed derived memory is traceable to source evidence.
- No derived proposal can silently replace source truth.
- Consolidation output appears in viewer/readback.

Linear mapping:

- Existing foundation: XY-800 is done.
- New follow-up required: `[ELF vNext P1] Implement reviewable consolidation worker and proposal review flow`.

#### P1.2 Knowledge Memory Pages

Problem:
Many compact memories remain hard to navigate unless compiled into stable, provenance-linked entity/project/concept pages.

Adopt from:

- llm-wiki and gbrain: maintained knowledge pages.
- ELF provenance model: every page section cites notes/events.

Implementation shape:

- Build derived pages for entities, concepts, projects, issues, and decisions.
- Add backlinks, source coverage, stale/unsupported-claim lint, and rebuild commands.
- Keep pages derived and rebuildable, not authoritative source truth.

Acceptance:

- A project page can be rebuilt from notes and preserves citations.
- Lint catches unsupported claims and stale source references.
- Viewer/search can surface page snippets with provenance.

Linear mapping:

- Existing: XY-286 is the right epic and should be expanded with smaller implementation issues.

#### P1.3 Temporal Graph-Lite Validity

Problem:
ELF already persists structured relations, but production memory needs time-aware facts: what was true when, what superseded it, and why.

Adopt from:

- Graphiti/Zep: temporal graph memory semantics.
- nanograph: typed graph/query ergonomics, without replacing Postgres.

Implementation shape:

- Use `valid_from` and `valid_to` semantics for relation facts.
- Keep append-only relation history and supersession evidence.
- Expose current versus historical temporal status in graph query and search relation context.
- Keep broader typed graph query ergonomics scoped to XY-70.

Acceptance:

- Contradictory facts do not overwrite silently.
- Search relation context labels current and historical facts.
- Tests cover invalidation, current readback, and old-state replay.

Linear mapping:

- Existing related: XY-70 covers graph-lite typed schema/query.
- Focused implementation issue: XY-863 `[ELF graph P1] Add temporal validity to graph-lite relation context`.

#### P1.4 Memory History and Evolution API

Problem:
Users and agents need to inspect how a memory changed over time, especially when an LLM proposed an update.

Adopt from:

- mem0: lifecycle/event history.
- ELF ingest decision table: existing audit direction.

Implementation shape:

- Add memory event history for add, update, ignore, reject, expire, derived, applied, and invalidated transitions.
- Expose history readbacks via HTTP/MCP.
- Link ingest decisions to note/relation versions.

Acceptance:

- A user can explain why a memory currently exists and what earlier evidence changed it.
- History survives restart and migration.
- Benchmark lifecycle checks include history expectations.

Linear mapping:

- New issue required: `[ELF memory P1] Add memory history and evolution readback API`.

#### P1.5 Core Memory Blocks vs Archival Memory

Problem:
Some memories should be intentionally small, always-attached operating context; most memory should remain retrievable archival context.

Adopt from:

- Letta: core memory blocks vs archival memory.
- ELF scope controls: explicit attachment and sharing.

Implementation shape:

- Add scoped, read-only memory blocks for stable agent/project instructions.
- Keep block attachment explicit per tenant/project/agent.
- Do not let blocks bypass evidence or policy boundaries.
- Keep blocks inspectable in viewer and MCP readback.

Acceptance:

- Agents can request their attached core blocks separately from search.
- Blocks have source/provenance metadata and audit history.
- Archival search remains independent.

Linear mapping:

- New issue required: `[ELF memory P1] Add scoped core memory blocks with archival separation`.

#### P1.6 Search Trajectory and Query Planning

Problem:
ELF already has expansion, hybrid retrieval, and reranking, but external tools expose the route more clearly.

Adopt from:

- qmd: weighted fusion and local debug knobs.
- OpenViking: staged retrieval trajectory and recursive retrieval.
- graphify: graph-compressed navigation hints.

Implementation shape:

- Add stable trace schema for query expansion, dense retrieval, BM25 retrieval, fusion, rerank, graph context, and final selection.
- Add optional recursive or staged retrieval profiles.
- Expose search-plan hints without making them hidden authority.

Acceptance:

- Every search result can explain its path.
- Tuning can be done through config/profile changes and benchmark replay.
- Wrong-result reports show stage-level cause.

Linear mapping:

- Existing related: XY-27 retrieval observability.
- New issue may be needed after XY-27: `[ELF retrieval P1] Add staged search trajectory profiles`.

### P2 - Ongoing Intelligence and Ecosystem Parity

These items keep ELF improving after the first production cut.

#### P2.1 ELF External Memory Pattern Radar

Problem:
External memory projects are moving quickly. Manual one-off reviews will go stale.

Adopt from:

- Local Dexter Pattern Radar automation.
- Decodex radar evidence discipline.

Implementation shape:

- Create a weekly Codex automation for ELF memory-system radar.
- Track upstream deltas for agentmemory, mem0, qmd, claude-mem, OpenViking, Graphiti, Letta, LightRAG, GraphRAG, and related projects.
- Maintain a structured cursor file plus prose memory.
- For every candidate pattern, produce an architecture-fit matrix:
  - upstream change
  - reusable pattern
  - ELF verdict: covered, reject, or gap
  - product value
  - duplicate/coverage evidence
  - safety boundary
  - issue decision
  - acceptance evidence
- Search Linear before creating issues.
- Create issues only when repo evidence shows a real gap.

Acceptance:

- A no-issue run records why ELF is already covered or why a pattern is rejected.
- A new issue includes source links, repo evidence, non-goals, and validation criteria.
- The radar never treats external runtime adoption as the default.

Linear mapping:

- New issue required: `[ELF ops P2] Add weekly external memory pattern radar automation`.

#### P2.2 Broaden Benchmark Adapter Coverage

Problem:
The current smoke covers the first project set, but broader claims need RAGFlow, LightRAG, GraphRAG, and deeper qmd/OpenViking profiles.

Adopt from:

- RAGFlow, LightRAG, GraphRAG: graph/RAG baselines.
- Current Docker live benchmark.

Implementation shape:

- Add D1/D2 research runs before implementation for large RAG systems.
- Add adapters only when Docker isolation is practical.
- Track install time, resource needs, and failure mode separately from retrieval quality.

Acceptance:

- Reports separate unsupported, blocked, incomplete, and wrong-result states.
- No external project is marked worse solely because setup is heavier.
- Claims remain scoped to encoded checks.

Linear mapping:

- New issue required: `[ELF benchmark P2] Add expanded RAG and graph-memory baseline adapters`.

#### P2.3 CLI and SDK Ergonomics

Problem:
ELF is service-first. External projects often feel easier for a local developer because their CLI path is direct.

Adopt from:

- qmd, memsearch, agentmemory: local CLI ergonomics.

Implementation shape:

- Add CLI wrappers for add/search/status/backfill/report if they are still missing or scattered.
- Keep commands thin over HTTP/MCP contracts.
- Link commands to benchmark and runbook workflows.

Acceptance:

- A local user can add notes, search, view status, run backfill, and generate benchmark report from documented commands.
- CLI output includes trace ids and source ids.

Linear mapping:

- New issue required after P0 runbook: `[ELF dx P2] Add local CLI wrappers for production memory workflows`.

## Issue Queue

| Order | Priority | Issue | Existing mapping | Parallelizable | Blocks |
| ---: | --- | --- | --- | --- | --- |
| 1 | P0 | Add resumable batch ingest and backfill benchmark | New | yes | production corpus migration |
| 2 | P0 | Add private-corpus production adoption benchmark | New | yes | final adoption claim |
| 3 | P0 | Add single-user production runbook with backup and restore | New | yes | unattended use |
| 4 | P0 | Prioritize retrieval observability panels | XY-27, after XY-19 | yes | efficient tuning |
| 5 | P0 | Make external adapters lifecycle-durable and fail-typed | New, follows XY-801 | yes | fair external comparison |
| 6 | P1 | Implement reviewable consolidation worker and proposal review flow | follows XY-800 | partly | knowledge pages |
| 7 | P1 | Split XY-286 into derived page storage, rebuild, lint, and viewer/search integration | XY-286 | partly | durable knowledge layer |
| 8 | P1 | Add temporal validity to graph-lite relation context | XY-863, follows/relates XY-70 | yes | time-aware relation context |
| 9 | P1 | Add memory history and evolution readback API | New | yes | lifecycle auditability |
| 10 | P1 | Add scoped core memory blocks with archival separation | New | yes | agent operating context |
| 11 | P1 | Add staged search trajectory profiles | New or XY-27 follow-up | after XY-27 | advanced retrieval tuning |
| 12 | P2 | Add weekly external memory pattern radar automation | New | yes | ongoing parity |
| 13 | P2 | Add expanded RAG and graph-memory baseline adapters | New | yes | broader public comparison |
| 14 | P2 | Add local CLI wrappers for production memory workflows | New | after P0.3 | local ergonomics |

## Parallel Development Plan

Safe concurrent lanes:

- Lane A: P0.1 batch ingest/backfill.
- Lane B: P0.2 private-corpus benchmark and manifest schema.
- Lane C: P0.3 production runbook and backup/restore proof.
- Lane D: P0.5 adapter lifecycle benchmark hardening.
- Lane E: XY-27 retrieval observability panels.
- Lane F: P2.1 radar automation, because it is mostly automation/config/docs and should not touch runtime code.

Avoid running concurrently without coordination:

- P1.1 consolidation worker and P1.2 knowledge pages, because knowledge pages should build on the reviewed derived proposal model.
- P1.3 temporal graph validity and XY-70 typed graph work, unless ownership is split cleanly between storage semantics and query ergonomics.
- P1.6 staged search trajectory and XY-27 viewer panels, unless the trace schema is agreed first.

Recommended Decodex queue order:

1. Queue P0.2 and P0.3 first because they define adoption evidence and recovery expectations.
2. Queue P0.1 and P0.5 in parallel because they exercise different implementation surfaces.
3. Promote XY-27 after the trace data needed by P0.5 is clear.
4. Start P1.1 only after P0.2 has enough corpus scenarios to evaluate consolidation quality.
5. Split XY-286 after P1.1 defines derived proposal semantics.

## Non-Goals

- Do not replace ELF core storage with any external memory runtime.
- Do not make Qdrant authoritative.
- Do not treat graph memory as a separate hidden source of truth.
- Do not allow background consolidation to mutate source notes silently.
- Do not benchmark with host-global installs when Docker isolation is feasible.
- Do not claim overall superiority from a benchmark dimension that is not encoded.
- Do not create new Linear issues from radar output without duplicate search and repo evidence.

## Production Adoption Gate

For personal production use, the minimum acceptable gate is:

- P0.1 batch ingest/backfill passes generated scale checks and resume checks.
- P0.2 private corpus benchmark has a passing or explicitly bounded result.
- P0.3 backup/restore runbook is tested on Docker Compose.
- P0.4/XY-27 gives enough viewer traceability to debug bad retrieval without raw SQL.
- P0.5 benchmark reports use typed failure states for external comparisons.

After that gate, ELF can reasonably be used as the personal production memory system with known limitations. Before that gate, ELF is a strong foundation with promising benchmark evidence, but the adoption risk is still too high to call it production-proven.
