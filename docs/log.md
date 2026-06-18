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
