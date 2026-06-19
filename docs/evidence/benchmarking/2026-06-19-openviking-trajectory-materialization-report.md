---
type: Evidence
title: "OpenViking Trajectory Materialization Report - June 19, 2026"
description: "Checked-in benchmark evidence record: OpenViking Trajectory Materialization Report - June 19, 2026."
resource: docs/evidence/benchmarking/2026-06-19-openviking-trajectory-materialization-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-19
tags:
  - docs
  - evidence
  - benchmarking
---
# OpenViking Trajectory Materialization Report - June 19, 2026

Goal: Close XY-983 by materializing the OpenViking context-trajectory follow-up
into reproducible benchmark evidence without turning blocked fixture rows into
ELF win, tie, or loss claims.
Read this when: You need to know whether OpenViking staged retrieval trajectory,
hierarchy selection, or recursive/context expansion blockers were removed after the
Dreaming competitor-strength retest.
Inputs:
`apps/elf-eval/fixtures/report_snapshots/2026-06-19-openviking-trajectory-materialization-report.json`,
`apps/elf-eval/fixtures/real_world_memory/context_trajectory/`,
`docs/evidence/benchmarking/2026-06-11-qmd-openviking-strength-profile-report.md`,
and `docs/evidence/benchmarking/2026-06-17-dreaming-competitor-strength-retest-report.md`.
Outputs: Scenario-level materialization readback for OpenViking staged retrieval,
hierarchy selection, and recursive/context expansion gates.

## Executive Judgment

The OpenViking trajectory follow-up is now materialized as a dedicated benchmark
slice, but the competitive status is unchanged.

`cargo make real-world-memory-context-trajectory` runs the three checked-in
`context_trajectory` fixtures and publishes:

- 3 jobs.
- 0 pass.
- 0 wrong_result.
- 3 blocked.
- 9/9 expected evidence matched.
- 9/9 source refs covered.
- 9/9 required quotes covered.

This improves auditability because the OpenViking trajectory blocker now has a
small, named repo task and a checked-in report snapshot. It does not remove the
blocker. ELF still has no scored win, tie, or loss against OpenViking staged
retrieval trajectory, hierarchy selection, or recursive/context expansion.

## Command Evidence

| Command | Status | Artifact | Result |
| --- | --- | --- | --- |
| `cargo make real-world-memory-context-trajectory` | `pass` | `tmp/real-world-memory/context-trajectory/report.json` and `tmp/real-world-memory/context-trajectory/report.md` | 3 encoded jobs, 0 pass, 3 blocked, 9/9 evidence coverage. |

No dedicated live OpenViking trajectory adapter was run in this lane. The command
uses the checked-in fixture contract to preserve the exact blocker and the artifact
shape required before a live comparison can be scored.

## Scenario Materialization

| Scenario | Previous status | Current status | Judgment | Required next artifact |
| --- | --- | --- | --- | --- |
| OpenViking staged retrieval trajectory | `blocked` | `blocked` | `unchanged` | Same-corpus expected/matched/missing evidence ids plus stage-level trajectory output for the same prompt. |
| OpenViking hierarchy selection | `blocked` | `blocked` | `unchanged` | Selected parent context, selected child context, final resource evidence ids, and rejected sibling or decoy context. |
| OpenViking recursive/context expansion | `blocked` | `blocked` | `unchanged` | Seed context, expanded child contexts, final evidence ids, and pruned branches. |

## Improvement and Regression Readback

| Bucket | Count | Meaning |
| --- | --- | --- |
| `improved` | 0 | No OpenViking context-trajectory strength moved from blocked to pass, wrong_result, or incomplete. |
| `regressed` | 0 | No checked scenario moved backward. |
| `unchanged` | 3 | All three trajectory/hierarchy/recursive scenarios remain typed blocked. |
| `blocked` | 3 | Every encoded OpenViking context-trajectory job still waits on materialized staged output. |

The useful improvement is operational, not competitive: future agents can now run a
single repo task to reproduce the exact blocked slice instead of rediscovering the
fixture directory and long runner command.

## Claim Boundaries

Allowed:

- The OpenViking context-trajectory slice is reproducible through
  `cargo make real-world-memory-context-trajectory`.
- The three OpenViking trajectory fixtures preserve typed blocked states with
  full evidence, source-ref, and quote coverage.
- The current result is unchanged versus the June 11 and June 17 reports.

Not allowed:

- Do not claim ELF beats OpenViking on staged retrieval trajectory.
- Do not claim ELF ties or beats OpenViking hierarchy selection.
- Do not claim ELF ties or beats OpenViking recursive/context expansion.
- Do not convert OpenViking same-corpus wrong_result evidence into a
  context-trajectory comparison win.

## Next Optimization Direction

The next useful lane is a real OpenViking live adapter materializer. It needs to emit:

1. `expected_evidence_ids`.
2. `matched_evidence_ids`.
3. `missing_evidence_ids`.
4. Stage outputs.
5. Selected parent and child contexts.
6. Final resource evidence ids.
7. Rejected or pruned contexts.
8. Expansion paths.

The success condition is not "ELF wins." The success condition is that at least one
OpenViking context-trajectory job can move from `blocked` to `pass`,
`wrong_result`, or `incomplete` based on comparable staged, hierarchy, or expansion
artifacts. Until then, the correct conclusion remains: OpenViking-style context
trajectory is an unresolved evidence gap for ELF.
