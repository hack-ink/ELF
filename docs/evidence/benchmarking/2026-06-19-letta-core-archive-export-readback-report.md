---
type: Evidence
title: "Letta Core/Archive Export-Readback Report - June 19, 2026"
description: "Checked-in benchmark evidence record: Letta Core/Archive Export-Readback Report - June 19, 2026."
resource: docs/evidence/benchmarking/2026-06-19-letta-core-archive-export-readback-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-19
tags:
  - docs
  - evidence
  - benchmarking
---
# Letta Core/Archive Export-Readback Report - June 19, 2026

Goal: Close the XY-984 materialization gap by adding a Docker-contained Letta
core/archive export-readback benchmark surface without changing ELF product
behavior or claiming ELF-over-Letta superiority.
Read this when: You need to know whether the Letta core-vs-archival comparison
blocker from the Dreaming competitor-strength retest was removed.
Inputs:
`apps/elf-eval/fixtures/report_snapshots/2026-06-19-letta-core-archive-export-readback-report.json`,
`apps/elf-eval/fixtures/real_world_memory/core_archival_memory/`,
`docs/evidence/benchmarking/2026-06-17-dreaming-competitor-strength-retest-report.md`,
and `docs/evidence/benchmarking/2026-06-16-scheduled-memory-task-scoring-report.md`.
Outputs: A Docker-contained materialization command, a generated Letta export/readback
artifact contract, and a scored typed-blocked report over the six core/archive jobs.

## Executive Judgment

The Letta follow-up is now reproducible as a benchmark materialization command, but
the competitive status is unchanged.

`cargo make smoke-letta-core-archive-export-readback` runs inside the baseline
Docker runner and publishes:

- 6 core/archive jobs.
- 0 pass.
- 0 wrong_result.
- 6 typed blocked.
- 14/14 evidence coverage.
- 14/14 source-ref coverage.
- 14/14 quote coverage.

This improves the audit trail relative to XY-955 because the Letta comparison now
has an executable materialization/report path. It does not remove the Letta blocker:
the default run intentionally does not start a live Letta server or use provider
credentials, so it records `letta_live_run_disabled` and preserves the comparison as
blocked until exported Letta core block JSON, archival readback/search JSON, and
fixture source ids exist.

## What Changed

- Added `cargo make smoke-letta-core-archive-export-readback`.
- Added `scripts/letta-core-archive-export-readback-smoke.py`.
- Added an optional `letta` Docker Compose profile.
- Updated the external adapter manifest so Letta is no longer recorded as lacking a
  materializer; it is recorded as materialized but still blocked by missing live
  export/readback source-id evidence.
- Checked in the JSON companion at
  `apps/elf-eval/fixtures/report_snapshots/2026-06-19-letta-core-archive-export-readback-report.json`.

## Scenario Status

| Scenario | Current Status | Judgment |
| --- | --- | --- |
| Core block attachment readback | `blocked` | Materialized typed blocker; no Letta pass/tie/loss claim. |
| Core block scope readback | `blocked` | Materialized typed blocker; no visibility claim without export metadata. |
| Core block provenance readback | `blocked` | Materialized typed blocker; no provenance claim without source-id export. |
| Stale core detection | `blocked` | Still blocked until core export joins archival supersession evidence. |
| Archival fallback readback | `blocked` | Still blocked until archival search/readback maps fallback source ids. |
| Core/archive project-decision recovery | `blocked` | Still blocked until core routing plus archival rationale source ids are exported. |

## Improvement/Regression Readback

- Improved: reproducibility and auditability. The comparison now has a Docker-owned
  command and durable JSON snapshot instead of only a research-gate note.
- Unchanged: competitive status. Letta remains blocked on live exported core/archive
  evidence, so there is no ELF win, tie, or loss.
- No regression: the existing ELF `core_archival_memory` fixture pass remains
  separate from Letta comparison scoring.

## Claim Boundaries

Allowed:

- The Letta comparison has a reproducible Docker-contained materialization/report
  command.
- The current default run preserves typed blockers when live Letta/provider setup
  cannot produce export/readback evidence.

Not allowed:

- Do not claim ELF beats Letta on core-vs-archival memory from fixture-only ELF
  evidence.
- Do not score Letta pass, win, tie, or loss unless exported core block JSON,
  archival readback/search JSON, and fixture source ids are present.

## Next Optimization Direction

The next live attempt should run:

`ELF_LETTA_SMOKE_START=1 ELF_LETTA_SMOKE_RUN=1 cargo make smoke-letta-core-archive-export-readback`

Required fields before scoring can move beyond blocked:

- exported core block JSON with fixture source ids,
- archival passage list/readback JSON with fixture source ids,
- archival search JSON for required evidence ids,
- model and embedding configuration,
- Docker-local agent/storage boundary,
- audit-equivalent metadata for source-id provenance.
