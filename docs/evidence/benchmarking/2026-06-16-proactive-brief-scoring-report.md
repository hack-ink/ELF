---
type: Evidence
title: "Proactive Brief Scoring Report - June 16, 2026"
description: "Checked-in benchmark evidence record: Proactive Brief Scoring Report - June 16, 2026."
resource: docs/evidence/benchmarking/2026-06-16-proactive-brief-scoring-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-18
tags:
  - docs
  - evidence
  - benchmarking
---
# Proactive Brief Scoring Report - June 16, 2026

Purpose: Publish the XY-953 fixture-backed proactive project brief scoring result.
Status: benchmark report
Read this when: You need the current proactive-brief fixture evidence, stage-ledger
delta, and claim boundaries.
Not this document: A scheduler design, morning-dashboard UI, private-corpus run, or
hosted managed-memory comparison.
Report owner: `docs/evidence/benchmarking/2026-06-16-proactive-brief-scoring-report.md`.

## Summary

`cargo make real-world-memory-proactive-brief` now scores a direct
`proactive_brief` fixture suite. The suite has 5 jobs: 4 pass, 1 blocked, 0
wrong_result, and 0 unsupported-claim results.

The four runnable jobs produce 5 suggestions across daily project brief,
resume-work brief, stale decision audit, and stale plan/preference warning scenarios.
Suggestion evidence-ref coverage is `5/5`; freshness/currentness coverage is `1.000`;
action-rationale coverage is `1.000`. The suite records 2 recommendations, 2 defers,
and 1 rejection, with 0 invalid-current suggestions and 0 tombstone violations.

The private-corpus refresh scenario remains a typed blocker tied to XY-930 because no
operator-owned private production corpus manifest is available. This is intentional:
the benchmark must not require private corpus access and must not turn missing private
inputs into a fixture pass.

## Fixture Results

| Job | Status | Suggestion kind | Decision | Evidence and freshness outcome |
| --- | --- | --- | --- | --- |
| `proactive-daily-project-brief-001` | `pass` | `daily_project_brief` | `recommend` | Current source refs selected; stale Pulse-parity trap dropped. |
| `proactive-resume-work-brief-001` | `pass` | `resume_work` | `recommend` | Current handoff and validation refs selected; stale branch trap dropped. |
| `proactive-stale-decision-audit-001` | `pass` | `stale_decision_audit` | `defer` | Superseded decision is surfaced as stale, not current. |
| `proactive-stale-plan-preference-warning-001` | `pass` | `stale_plan_preference_warning` | `defer`, `reject` | Expired, superseded, and tombstoned sources are warning inputs, not current recommendations. |
| `proactive-private-corpus-refresh-blocked-001` | `blocked` | `private_corpus_refresh` | blocked | Private-corpus refresh stays blocked until XY-930 operator inputs exist. |

## Aggregate Delta

The root fixture aggregate after XY-953 is:

| Metric | Value |
| --- | ---: |
| Jobs | `55` |
| Encoded suites | `15` |
| Pass | `49` |
| Blocked | `6` |
| Wrong result | `0` |
| Incomplete | `0` |
| Not encoded | `0` |
| Unsupported claim count | `0` |
| Evidence coverage | `123/123` |
| Source-ref coverage | `123/123` |
| Quote coverage | `123/123` |
| Expected evidence recall | `1.000` |
| Mean score | `0.891` |

XY-951 stage-ledger delta for `proactive_brief_readiness`:

| Baseline | After XY-953 | Judgment |
| --- | --- | --- |
| `pass=0`, `wrong_result=0`, `blocked=0`, `not_tested=1`, `not_encoded=1` | `pass=4`, `wrong_result=0`, `blocked=1`, `not_tested=0`, `not_encoded=0` | `improved` |

## Regression Guards

The proactive scorer fails or downgrades output when a suggestion:

- lacks evidence refs,
- lacks freshness/currentness markers,
- lacks a reject/defer/recommend rationale,
- presents stale, superseded, expired, or tombstoned evidence as current,
- ignores TTL invalidations or tombstones,
- carries unsupported current-suggestion flags,
- or claims private-corpus, Pulse, or hosted managed-memory parity from fixture-only
  output.

## Claim Boundaries

Allowed:

- ELF now has fixture-backed proactive brief scoring for project briefs and stale
  context warnings.
- Passing proactive suggestions include evidence refs, freshness/currentness markers,
  and action rationale.
- The private-corpus refresh case is encoded as a typed blocker tied to XY-930.

Not allowed:

- Do not claim OpenAI Pulse parity.
- Do not claim hosted managed-memory parity.
- Do not claim scheduler, morning-dashboard, or background execution behavior.
- Do not claim private-corpus refresh quality without operator-owned inputs.
- Do not treat proactive suggestions as authoritative notes; they are derived,
  source-linked output that must remain reviewable.

## Next Direction

Move from fixture-backed proactive brief scoring into service-native generated brief
readback and later live adapter materialization. Scheduling and private-corpus refresh
remain owned by their separate lanes and operator-input gates.
