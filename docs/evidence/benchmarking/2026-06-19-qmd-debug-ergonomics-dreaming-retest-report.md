---
type: Evidence
title: "qmd Debug-Ergonomics Dreaming Retest Report - June 19, 2026"
description: "Checked-in benchmark evidence record: qmd Debug-Ergonomics Dreaming Retest Report - June 19, 2026."
resource: docs/evidence/benchmarking/2026-06-19-qmd-debug-ergonomics-dreaming-retest-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-19
tags:
  - docs
  - evidence
  - benchmarking
---
# qmd Debug-Ergonomics Dreaming Retest Report - June 19, 2026

Goal: Close XY-982 by retesting the qmd debug-ergonomics follow-up after the
Dreaming-readiness stages and the XY-955 competitor-strength closeout.
Read this when: You need to know whether Dreaming-stage improvements erased,
improved, or regressed the qmd local-debug artifact edge.
Inputs:
`apps/elf-eval/fixtures/report_snapshots/2026-06-19-qmd-debug-ergonomics-dreaming-retest-report.json`,
`docs/evidence/benchmarking/2026-06-11-elf-qmd-trace-replay-diagnostics-report.md`,
`docs/evidence/benchmarking/2026-06-17-dreaming-competitor-strength-retest-report.md`,
and the fresh `tmp/real-world-job/operator-ux-live-adapters/summary.json` output.
Outputs: Scenario-level improved/regressed/unchanged/not-tested/non-goal judgments
for qmd debug ergonomics, with claim boundaries and next optimization direction.

## Executive Judgment

The qmd debug-ergonomics outcome is unchanged after the Dreaming stages.

The fresh live operator-debug retest confirms ELF's narrow trace/stage visibility
advantage:

- `cargo make real-world-job-operator-ux-live-adapters` passed on June 19, 2026.
- ELF scored 6 operator-debug jobs, with 6 pass, 0 wrong_result, trace visibility on
  all 6 jobs, replay commands on all 6 jobs, and no raw SQL requirement.
- qmd scored the same 6 operator-debug jobs, with 0 pass and 6 wrong_result because
  local replay output is available but service trace hydration and intermediate
  candidate-drop stages are not exposed in this live slice.

This does not erase qmd's measured debug edge from the June 11 diagnostics. qmd still
preserves the default top-10 candidate JSON and short local CLI replay advantage.
ELF has useful service trace/admin surfaces, but the default stress/report artifacts
still do not emit a directly comparable qmd-style candidate artifact with expansion,
fusion, rerank, and dropped-candidate stage details.

No retested debug-ergonomics scenario regressed. No broad ELF-over-qmd superiority
claim is supported.

## Command Evidence

| Command | Status | Artifact | Result |
| --- | --- | --- | --- |
| `cargo make real-world-job-operator-ux-live-adapters` | `pass` | `tmp/real-world-job/operator-ux-live-adapters/summary.json` | ELF 6 pass/0 wrong_result; qmd 0 pass/6 wrong_result. |

## Fresh Live Retest

| Adapter | Jobs | Pass | Wrong result | Trace available | Replay available | Candidate-drop visibility | Raw SQL needed |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ELF operator-debug live | 6 | 6 | 0 | 6 | 6 | stage visibility present across all jobs | 0 |
| qmd operator-debug live | 6 | 0 | 6 | 0 | 6 | top-k replay output only; no intermediate candidate-drop stages | 0 |

The qmd rows are typed non-pass for this live operator-debug slice, not a regression
of qmd's default local replay surface. qmd remains useful for direct local top-k
inspection.

## Scenario Retest Matrix

| Scenario | June 11 baseline | June 19 retest | Judgment | Boundary |
| --- | --- | --- | --- | --- |
| qmd default top-10 candidate artifact | ELF `loss` | ELF `loss` | `unchanged` | qmd still exposes direct top-10 rows; ELF has trace ids and admin surfaces but no default qmd-like candidate artifact in the stress report. |
| qmd short CLI replay | ELF `loss` | ELF `loss` | `unchanged` | qmd replay remains a short local CLI path; ELF replay still depends on service config, headers, traces, and bundle hydration. |
| ELF operator-debug trace hydration | ELF `win` | ELF `win` | `unchanged` | ELF has trace visibility on 6/6 jobs; qmd has replay commands but no service trace hydration in this slice. |
| Operator-debug replay command availability | `tie` | `tie` | `unchanged` | Both adapters emit replay commands on 6/6 jobs; this does not score equivalent UI quality. |
| Operator-debug candidate-drop visibility | ELF `win` | ELF `win` | `unchanged` | ELF exposes stage visibility; qmd exposes top-k output but not intermediate drops. |
| Operator-debug selected-but-not-narrated visibility | ELF `win` | ELF `win` | `unchanged` | ELF exposes final results and narration-stage details for the selected-but-not-narrated case; qmd does not expose an equivalent service trace surface. |
| Query expansion attribution | `not_tested` | `not_tested` | `not_tested` | No comparable expansion-variant artifact exists for both systems. |
| Dense/sparse channel attribution | `not_tested` | `not_tested` | `not_tested` | Current artifacts still do not expose comparable dense-only and sparse-only contribution data. |
| Fusion attribution | `not_tested` | `not_tested` | `not_tested` | Current artifacts still do not expose comparable fusion inputs, rank deltas, or dropped candidates. |
| Rerank attribution | `non_goal` | `non_goal` | `non_goal` | The qmd materializer path remains a `--no-rerank` path for this evidence line. |

## Improvement and Regression Readback

| Bucket | Count | Meaning |
| --- | --- | --- |
| `improved` | 0 | The retest did not add a new comparable default artifact that beats qmd's local debug surface. |
| `regressed` | 0 | No checked scenario moved backward from the June 11 or June 17 evidence. |
| `unchanged` | 6 | qmd keeps the default top-k/replay edge; ELF keeps the operator-debug trace/stage visibility wins. |
| `not_tested` | 3 | Expansion, dense/sparse contribution, and fusion are still missing comparable artifacts. |
| `non_goal` | 1 | Rerank scoring remains out of scope for the qmd `--no-rerank` materializer path. |

## Claim Boundaries

Allowed:

- qmd's default local-debug edge remains: top-10 candidate rows plus short CLI replay.
- ELF still wins the narrow live operator-debug trace hydration, candidate-drop
  visibility, and selected-but-not-narrated visibility slice.
- Both systems still expose replay commands for the operator-debug fixtures.
- The Dreaming-stage retest did not find a debug-ergonomics regression.

Not allowed:

- Do not claim ELF broadly beats qmd from this retest.
- Do not treat qmd's 0 pass/6 wrong_result live operator-debug slice as proof that
  qmd's default top-k/replay edge is gone.
- Do not claim expansion, fusion, dense/sparse contribution, or rerank parity until
  directly comparable artifacts are emitted.
- Do not collapse `not_tested`, `non_goal`, or `wrong_result` into pass evidence.

## Next Optimization Direction

The next useful improvement is not another broad leaderboard rerun. It is a comparable
candidate-replay artifact for both ELF and qmd that emits:

1. Immediate top-k rows with source id, file or note id, score, snippet, and rank.
2. Expansion variants and whether the original query was retained.
3. Dense-only and sparse-only candidate sets.
4. Fusion rank deltas and score contributions.
5. Rerank score, or an explicit rerank-disabled marker.
6. Dropped or demoted expected evidence.
7. One-command replay lines for both systems.

Until that exists, the correct conclusion is unchanged: qmd keeps the default local
debug artifact edge, while ELF keeps the service-backed operator-debug trace/stage
visibility wins.
