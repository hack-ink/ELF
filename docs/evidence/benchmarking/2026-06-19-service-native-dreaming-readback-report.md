---
type: Evidence
title: "Service-Native Dreaming Readback Report - June 19, 2026"
description: "Checked-in benchmark evidence record: Service-Native Dreaming Readback Report - June 19, 2026."
resource: docs/evidence/benchmarking/2026-06-19-service-native-dreaming-readback-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-19
tags:
  - docs
  - evidence
  - benchmarking
---
# Service-Native Dreaming Readback Report - June 19, 2026

Goal: Close XY-986 by moving the public/local Dreaming summary, proactive brief,
and scheduled-memory readback slice from fixture-only artifacts into a reproducible
ELF service-native materialization path.
Read this when: You need to know whether ELF now materializes Dreaming-style
derived outputs through `ElfService` before benchmark scoring.
Inputs:
`apps/elf-eval/fixtures/report_snapshots/2026-06-19-service-native-dreaming-readback-report.json`,
`apps/elf-eval/fixtures/report_snapshots/2026-06-19-service-native-dreaming-readback-materialization.json`,
`apps/elf-eval/fixtures/real_world_memory/memory_summary/`,
`apps/elf-eval/fixtures/real_world_memory/proactive_brief/`, and
`apps/elf-eval/fixtures/real_world_memory/scheduled_memory/`.
Outputs: A Docker-contained service-native Dreaming benchmark command, a scored
report snapshot, and a materialization snapshot proving readback through
`ElfService::add_note -> ElfService::list -> derived readback artifact`.

## Executive Judgment

The service-native Dreaming readback follow-up improves ELF's local public
Dreaming evidence authority, but it does not prove broad managed-memory product
superiority.

`cargo make real-world-memory-service-native-dreaming` runs inside the baseline
Docker runner and publishes:

- 11 jobs.
- 9 pass.
- 0 wrong_result.
- 0 lifecycle_fail.
- 0 incomplete.
- 2 typed blocked.
- 22/22 expected evidence coverage.
- 22/22 source-ref coverage.
- 22/22 quote coverage.

The two blocked jobs are the existing XY-930 private/provider gates:
`proactive-private-corpus-refresh-blocked-001` and
`scheduled-private-provider-scheduler-blocked-001`. They remain blocked because no
operator-owned private production corpus manifest, provider credentials, or hosted
scheduler configuration is present.

## What Changed

- Added `cargo make real-world-memory-service-native-dreaming`.
- Added `scripts/real-world-dreaming-service-native.sh`.
- Added the `memory-service-native-dreaming` Docker runner profile.
- Extended the ELF live adapter so `memory_summary`, `proactive_brief`, and
  `scheduled_memory` jobs can materialize derived output artifacts from service
  readback instead of fixture-only answer payloads.
- Separated full artifact source-ref audit from scored evidence ids. The
  materialization snapshot keeps stale, superseded, tombstoned, and dropped refs
  visible for review, while the scored answer only exposes required non-trap refs.

## Command Evidence

| Command | Status | Artifact | Result |
| --- | --- | --- | --- |
| `cargo make real-world-memory-service-native-dreaming` | `pass` | `tmp/real-world-memory/service-native-dreaming/report.json`; `tmp/real-world-memory/service-native-dreaming/elf-materialization.json` | 11 jobs, 9 pass, 0 wrong_result, 2 blocked, 22/22 evidence/source-ref/quote coverage. |

## Service Readback Evidence

Every passing public/local Dreaming job records:

- `runtime_path`: `ElfService::add_note -> ElfService::list -> derived readback artifact`.
- `missing_source_refs`: `[]`.
- `source_mutation_count`: `0`.
- `no_source_mutation_checked`: `true`.

The audit snapshot intentionally preserves stale and trap refs inside
`dreaming_readback.selected_source_refs` when they appear in `source_trace`; the
scored `evidence_ids` and benchmark `produced_evidence` exclude those trap refs so
they are not treated as used evidence.

## Improvement/Regression Readback

| Bucket | Count | Meaning |
| --- | --- | --- |
| `improved` | 9 | Public/local Dreaming jobs now pass after service-native readback materialization. |
| `regressed` | 0 | No checked public/local Dreaming job moved backward. |
| `blocked` | 2 | Private corpus and provider/hosted scheduler gates remain blocked under XY-930. |

Compared with the earlier fixture-backed Dreaming readiness evidence, this lane
improves runtime authority and auditability: the benchmark now proves ELF can
materialize reviewable summary, proactive brief, and scheduled-memory artifacts
through its own service list/readback path. It does not add provider-backed private
corpus coverage or hosted scheduler parity.

## Claim Boundaries

Allowed:

- ELF has a reproducible service-native Dreaming readback benchmark for the checked
  public/local `memory_summary`, `proactive_brief`, and `scheduled_memory` fixtures.
- The current service-native slice scores 9 pass, 0 wrong_result, and 2 typed
  blockers with full evidence/source-ref/quote coverage.
- Passing jobs preserve source-readback audit metadata and record zero source
  mutations.

Not allowed:

- Do not claim ELF broadly beats OpenAI Pulse, ChatGPT Tasks, Claude Dreams, or
  hosted managed-memory products from this local service-native slice.
- Do not claim private-corpus or provider-backed Dreaming readiness until XY-930
  operator-owned inputs exist.
- Do not treat stale/trap refs preserved in materialization audit metadata as used
  benchmark evidence.

## Next Optimization Direction

The next useful lane is XY-930: run private-corpus, provider-backed, and hosted
scheduler gates only when operator-owned inputs exist. Until then, optimization
should focus on surfacing these derived artifacts in operator UI/review workflows
without converting private/provider blockers into claimed wins.
