---
type: Evidence
title: "Operator-Approved Public-Proxy Production-Private Addendum - June 19, 2026"
description: "Checked-in benchmark evidence record for the XY-930 operator-approved public-proxy run through the production-private addendum path."
resource: docs/evidence/benchmarking/2026-06-19-operator-approved-public-proxy-production-private-addendum.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-19
tags:
  - docs
  - evidence
  - benchmarking
---
# Operator-Approved Public-Proxy Production-Private Addendum - June 19, 2026

Goal: Close the current XY-930 blocker with an operator-approved simulated/public-proxy
production corpus while preserving the private-corpus and provider-backed evidence
boundaries.
Read this when: You need to know whether the fail-closed
`ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST` path can run without a real private corpus,
and which claims remain disallowed.
Inputs:
`apps/elf-eval/fixtures/report_snapshots/2026-06-19-operator-approved-public-proxy-production-private-addendum.json`,
`tmp/live-baseline/live-baseline-report.json`, and
`tmp/live-baseline/operator-approved-public-proxy-addendum.md`.
Outputs: A public-safe report snapshot, a production-private addendum run, and explicit
claim boundaries for simulated/public-proxy versus real private/provider evidence.

## Executive Judgment

The XY-930 proxy run is complete: the production-private addendum entrypoint passed on
an operator-approved simulated/public-proxy corpus.

The command exercised the fail-closed production-private manifest path and published:

- 12 documents.
- 8 queries.
- 8/8 full checks passing.
- 8/8 same-corpus query matches.
- 0 wrong_result.
- 0 lifecycle_fail.
- 0 blocked.
- 0 incomplete.
- 0 not_encoded.

This improves the lane from "blocked by missing manifest" to "proxy corpus pass." It
does not prove real private-corpus production quality, provider-backed embedding
quality, or broad competitor superiority.

## Command Evidence

| Command | Status | Run ID | Artifacts |
| --- | --- | --- | --- |
| `ELF_BASELINE_PROJECTS=ELF ELF_BASELINE_PRODUCTION_CORPUS_MANIFEST=/workspace/tmp/<operator-approved-public-proxy-manifest>.json ELF_BASELINE_PRIVATE_ADDENDUM=tmp/live-baseline/operator-approved-public-proxy-addendum.md cargo make baseline-production-private-addendum` | `pass` | `live-baseline-20260619143959` | `tmp/live-baseline/live-baseline-report.json`; `tmp/live-baseline/operator-approved-public-proxy-addendum.md` |

The runner reports corpus profile `production-private` and track
`private_production` because the production-private entrypoint was used. The manifest
itself is `operator-approved-public-proxy-prod-corpus-2026-06-19`, so the track label
must not be read as real private data authority.

## Run Summary

| Field | Value |
| --- | --- |
| Project | `ELF` |
| Commit | `56c68e6518ed7c255d6c21b867315277670fc995` |
| Corpus profile | `production-private` |
| Corpus track | `private_production` |
| Corpus manifest | `operator-approved-public-proxy-prod-corpus-2026-06-19` |
| Manifest kind | `operator_approved_public_proxy` |
| Embedding mode | `local` |
| Embedding model | `local-hash` |
| Query mean latency | `10.842727625 ms` |
| Query P50/P95/P99 | `8.186716 ms` / `30.443385 ms` / `30.443385 ms` |
| Resource envelope | `1.313984156s`, `37656` RSS KB |
| Cost proxy | `386` estimated input tokens; no configured cost rate |

## Query Evidence

| Query | Task | Expected Evidence | Top Evidence | Trace ID | Latency |
| --- | --- | --- | --- | --- | --- |
| `q-resume-xy930-policy` | `resume_lane` | `issue-xy930-policy` | `issue-xy930-policy` | `882fc41f-7ea0-42c1-a04e-a62713b8e7d0` | `9.300164 ms` |
| `q-recover-private-command` | `recover_exact_command` | `runbook-private-command` | `runbook-private-command` | `929516c3-03d9-4d9f-aa7d-cc5a5c76e9d3` | `30.443385 ms` |
| `q-explain-provider-blocker` | `explain_stale_blocker` | `blocker-provider-missing` | `blocker-provider-missing` | `66e32fc2-71b1-40bf-b1d3-7e60427a2573` | `8.186716 ms` |
| `q-find-proxy-boundary` | `find_prior_decision` | `decision-proxy-boundary` | `decision-proxy-boundary` | `93651b26-6584-4883-ae30-ff9928cace59` | `7.743761 ms` |
| `q-compare-dreaming-graphrag` | `compare_project_status` | `issue-xy986-dreaming` | `issue-xy986-dreaming` | `b4a71e95-1571-4b7d-9fa6-e6e8be1b62a1` | `7.350473 ms` |
| `q-detect-sdk-ui-export` | `detect_contradiction_update` | `issue-xy987-openmemory` | `issue-xy987-openmemory` | `6790eab4-561c-4c9e-abc4-728580f359c5` | `7.606096 ms` |
| `q-recover-addendum-safety` | `recover_exact_command` | `runbook-addendum-safety` | `runbook-addendum-safety` | `11fa7d80-7a95-4b6f-861f-ae43acf469e0` | `7.805386 ms` |
| `q-resume-cleanup` | `resume_lane` | `worktree-cleanup` | `worktree-cleanup` | `7e44260b-330d-4168-ab98-7fae99e5318f` | `8.30584 ms` |

## Backfill And Lifecycle Evidence

- Backfill source count: `12`.
- Completed count: `12`.
- Batch size: `32`.
- Worker concurrency: `1`.
- Resume probe: interrupted after `6/12`, then resumed to `12/12`.
- Skipped completed on resume: `6`.
- Duplicate source notes: `0`.
- Encoded checks passed: resumable backfill, same-corpus retrieval, async worker
  indexing, update replacement, delete suppression, cold-start recovery, concurrent
  write/search, and resource envelope.

## Improvement/Regression Readback

Improved:

- XY-930 no longer depends on a human-supplied real private manifest for this proxy
  stage.
- The production-private addendum path moved from missing-manifest blocked to 8/8
  pass on the approved public-proxy corpus.
- Resume, lifecycle, cold-start, concurrent write/search, and resource checks stayed
  green.

Unchanged:

- Real private-corpus production quality is still not proven.
- Provider-backed embedding quality is still not proven because this run used
  `local-hash`.
- Broad competitor superiority is unchanged; this run only covers the ELF
  private-entrypoint proxy signal.

Regressed: none.

## Claim Boundaries

Allowed:

- The production-private addendum entrypoint passed on the operator-approved
  public-proxy corpus.
- This stage produced 8/8 query passes, 0 wrong_result, 0 lifecycle_fail, 0 blocked,
  0 incomplete, and 0 not_encoded.
- The result is a useful proxy signal for XY-930 planning and benchmark continuity.

Not allowed:

- Do not call this real private-corpus production proof.
- Do not claim provider-backed production quality; embedding mode was local.
- Do not treat the runner track `private_production` as a private data authority
  claim.
- Do not use this single ELF proxy run as broad competitor-superiority evidence.

## Next Optimization Direction

Immediate:

- Keep this report as the XY-930 public-proxy closure evidence.
- Reuse the same addendum path for future public/downloaded corpora before any real
  private corpus is introduced.

When operator-owned inputs exist:

- Run the same profile with a real private production corpus manifest.
- Run provider-backed embeddings with `ELF_BASELINE_ELF_EMBEDDING_MODE=provider`.
- Compare proxy, real-private, and provider-backed results before claiming production
  quality.
