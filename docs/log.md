# Documentation Maintenance Log

Purpose: Record material OKF and LLM Wiki navigation, promotion, naming, and
maintenance changes.
Read this when: You need to understand why documentation structure changed.
Not this document: Detailed subsystem history, raw research state, or plan execution
logs.

## 2026-06-18

- Adopted the Decodex Markdown-only OKF and LLM Wiki profile for `docs/`.
- Added `docs/policy.md` as the canonical documentation-shape owner.
- Added required lane indexes for `decisions`, `evidence`, `reference`, `research`,
  and `runbook`.
- Moved raw JSON research and evaluation artifacts out of `docs/` so docs can remain
  Markdown-only while preserving machine-readable evidence.
- Promoted settled legacy research JSON into decision, spec, runbook, and evidence
  owners; moved test-required machine reports to app fixtures after Markdown reports
  became the docs owners.
- Carried unresolved but valuable points forward as explicit research contracts under
  `docs/research/`.
- Moved the external-memory pattern radar cursor to
  `apps/elf-eval/fixtures/external_memory_pattern_radar/cursor.json` because it is
  active tool state rather than a research conclusion.
- Moved the latest external-memory pattern radar summary to
  `docs/evidence/external_memory_pattern_radar_latest.md` because it is evidence, not
  latent research.
- Added a docs self-check drift audit under `docs/evidence/`.
- Removed the legacy guide top-level lane. Procedural documents now live under
  `docs/runbook/`; checked reports and external comparison inputs live under
  `docs/evidence/`.
- Moved retained plan artifacts from the legacy plans top-level lane to
  `docs/reference/plans/` so the
  top-level docs directories match the Decodex docs lane set.

## 2026-06-19

- Added the OpenViking trajectory materialization evidence report and snapshot for
  XY-983, preserving staged retrieval, hierarchy selection, and recursive/context
  expansion as typed blockers until comparable staged artifacts exist.
- Added `cargo make real-world-memory-context-trajectory` as the reproducible
  context-trajectory benchmark entrypoint and linked the new report from the
  benchmarking evidence index and README.
- Added the Letta core/archive export-readback materialization report and snapshot
  for XY-984, plus `cargo make smoke-letta-core-archive-export-readback`, preserving
  all six Letta comparison scenarios as typed blockers until exported core block JSON,
  archival readback/search JSON, and fixture source ids exist.
- Added the service-native Dreaming readback report and snapshots for XY-986, plus
  `cargo make real-world-memory-service-native-dreaming`, proving public/local
  memory summary, proactive brief, and scheduled-memory artifacts can be materialized
  through `ElfService` readback while preserving XY-930 private/provider blockers.
- Added the OpenMemory UI/export product readback recheck report and snapshot for
  XY-987, preserving the product UI/export scenario as blocked while keeping mem0 SDK
  `get_all` evidence separate from OpenMemory product evidence.
- Added the graph/RAG citation/navigation promotion report and snapshot for XY-985,
  preserving representative graph/RAG outcomes as typed non-pass while recording
  graphify evidence-linked output and the remaining adapter-specific blockers.
- Added the operator-approved public-proxy production-private addendum report and
  snapshot for XY-930, recording `baseline-production-private-addendum` as 8/8 pass
  on the simulated/public-proxy corpus while preserving real private-corpus and
  provider-backed production quality as unproven.

## 2026-06-20

- Added the live knowledge-page rebuild/lint report for XY-935, plus
  `cargo make real-world-memory-live-knowledge`, proving the checked-in knowledge
  fixture pack can be materialized through `ElfService` rebuild, lint, and page
  search before scoring while keeping external wiki/graph/RAG product comparisons
  separate.
- Added the Knowledge Workspace version-diff report for XY-1019. Knowledge page
  rebuild metadata now exposes `elf.knowledge_page.version_diff/v1`, live benchmark
  artifacts expose `page_version_diff`, and the Docker-contained live knowledge
  report now publishes `version_diff_coverage`.
- Added the Graph Topic-Map report for XY-1020. ELF now exposes
  `elf.graph_report/v1` through service, HTTP, and MCP readback, using existing
  Postgres graph-lite facts with sourced, inferred, ambiguous, stale, and superseded
  markers while keeping `valid_from`/`valid_to` as the internal temporal vocabulary.

## 2026-06-22

- Added `docs/spec/agent_memory_knowledge_system_v1.md` for XY-1059, codifying the
  Agent Memory + Knowledge System product boundary, P0-P5 roadmap, Decodex
  phase-gate rule, competitor absorption boundaries, validation expectations, and
  phase closeout checklist.
- Linked the new product contract from the docs root index and spec index.
- Added the P1 memory-authority closeout report for XY-1063, plus
  `cargo make real-world-memory-p1-closeout`, preserving the Source Library ->
  Memory Candidate -> approved memory -> recall/debug -> correction/rollback
  authority chain and keeping P2 queueing conditional on main-thread acceptance.
- Added the P2 Knowledge Workspace PageIndex/OpenKB closeout report for XY-1066,
  plus `cargo make real-world-memory-p2-knowledge-closeout` and a changed-source
  watch/rebuild fixture, preserving PageIndex/OpenKB as reference-only `not_tested`
  rows until contained adapters emit comparable artifacts.
- Added the mem0/OpenMemory and Letta P3 adapter report for XY-1069, plus
  `cargo make real-world-memory-mem0-openmemory-letta` and same-corpus fixtures that
  map mem0 SDK history/export outputs to source ids while preserving OpenMemory
  UI/export and Letta core/archive as typed blockers.
- Added the Knowledge Workspace changed-source watch/rebuild contract for XY-1065,
  plus a drift audit covering the new admin rebuild endpoint, changed/unchanged/
  stale/blocked section output, stale-section/changed-claim/missing-citation/conflict
  classifications, and reviewable memory-candidate proposal routing.

## 2026-06-23

- Added the P3 competitor-strength absorption closeout report for XY-1072, plus a
  checked-in snapshot and guard test that keeps qmd, PageIndex/OpenKB,
  mem0/OpenMemory, Letta, Graphiti/Zep, OpenViking, RAGFlow, GraphRAG, and LightRAG
  claim boundaries explicit while leaving P4 queue labels unapplied pending
  main-thread acceptance.
- Added the P4 quality hardening and productization readiness closeout report for
  XY-1075, plus `cargo make real-world-memory-p4-quality-hardening-closeout` and a
  checked-in snapshot that reruns adversarial, source-library, knowledge, and
  production-readiness slices while preserving private/provider blockers and keeping
  P5 queue labels unapplied pending main-thread acceptance.
- Added the one-command local agent loop for XY-1076, plus agent setup recipes for
  Codex, Claude/Cursor-style MCP clients, generic MCP clients, and CLI workflows.
  The new drift audit anchors the deterministic source import -> proposal approval
  -> recall/debug -> correction/rollback route to current HTTP, MCP, config, and
  task surfaces.
- Added the privacy, delete, export, and retention boundary runbook for XY-1078,
  plus a drift audit and spec updates for Source Library span suppression, Knowledge
  Workspace source visibility, graph evidence readback, and relation-context
  evidence filtering.

## 2026-06-27

- Added `docs/spec/system_work_journal_v1.md` for XY-1117, defining
  source-adjacent Work Journal capture and readback, canonical entry families,
  redaction, source refs, promotion-boundary metadata, and the non-authoritative
  memory boundary.
- Linked the Work Journal contract from the spec index, version registry, and ELF v2
  service/MCP endpoint map.
- Added the Work Journal drift audit tying the new service, storage, HTTP, MCP, and
  test surfaces to the source-adjacent promotion-boundary contract.
- Tightened the Work Journal promotion-boundary contract so accepted references must
  resolve to storage-backed Memory Authority or Dreaming Review evidence instead of
  granting authority from JSON shape alone.
- Added the XY-1118 Work Continuity benchmark slice, documenting the
  `work_continuity` real-world suite, `cargo make real-world-memory-work-continuity`,
  Work Journal oracle fields, report rates, and hard-fail counters for redaction,
  rejected-option, inferred-step, journal-authority, and janitor false-promotion
  boundaries.
- Added the XY-1119 authority recovery drill production-ops slice, defining
  `elf.authority_recovery_drill/v1` report artifacts, validating topology, degraded
  reads, RPO/RTO, authority record counts, idempotent outbox replay, Qdrant rebuild,
  migration repair, and dead-letter handling, and linking the drift audit.
