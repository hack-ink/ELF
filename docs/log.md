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
