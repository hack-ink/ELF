---
type: Evidence
title: "Live Baseline Benchmark Report"
description: "Checked-in benchmark evidence record: Live Baseline Benchmark Report."
resource: docs/evidence/benchmarking/2026-06-09-production-corpus-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-18
tags:
  - docs
  - evidence
  - benchmarking
---
# Live Baseline Benchmark Report

Goal: Publish a Markdown summary for one generated live baseline aggregate report.
Read this when: You need a durable, reviewable summary of a live baseline JSON report.
Inputs: `tmp/live-baseline/live-baseline-report.json`.
Depends on: `scripts/live-baseline-benchmark.sh` and `docs/runbook/benchmarking/live_baseline_benchmark.md`.
Verification: Compare this Markdown summary with the source JSON before committing.

## Summary

- Run ID: `live-baseline-20260609045306`
- Generated at: `2026-06-09T04:53:18Z`
- Verdict: `pass`
- Project filter: `ELF`
- Corpus profile: `production-synthetic`
- Corpus track: `synthetic_production`
- Corpus manifest: `synthetic-coding-agent-prod-corpus-2026-06-09`
- Documents: `8`
- Queries: `6`
- Wrong-result count: `0`
- Query latency mean: `7.137632833333334 ms`
- Project summary: `1 pass`, `0 fail`, `0 incomplete`
- Same-corpus summary: `1 pass`, `0 fail`, `0 incomplete`
- Full check summary: `7/7 pass`

This report is production-corpus benchmark evidence only. Use
`docs/runbook/single_user_production.md` for the single-user Docker Compose production
runbook, including backup, restore, Qdrant rebuild, rollback, provider config
handling, and cleanup commands.

## Projects

| Project | Status | Retrieval | Checks | Elapsed | Reason |
| --- | --- | --- | --- | --- | --- |
| ELF | `pass` | `retrieval_pass` | `7/7` | `12s` | ELF added the corpus, rebuilt Qdrant, and returned expected evidence for every query |

## Embedding

| Project | Mode | Provider | Model | Dimensions | Timeout | API Base | Path |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ELF | `local` | `local` | `local-hash` | `256` | `1000ms` | `http://127.0.0.1` | `/embeddings` |

## Query Evidence

| Project | Query | Task | Expected Evidence | Allowed Alternates | Top Evidence | Matched | Latency |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ELF | `q-resume-lane` | `resume_lane` | `issue-xy812-resume` | `` | `issue-xy812-resume` | `true` | `9.213627 ms` |
| ELF | `q-recover-exact-command` | `recover_exact_command` | `worktree-xy791-repair` | `runbook-live-baseline` | `worktree-xy791-repair` | `true` | `6.424872 ms` |
| ELF | `q-explain-stale-blocker` | `explain_stale_blocker` | `blocker-stale-qwen-key` | `` | `blocker-stale-qwen-key` | `true` | `7.749393 ms` |
| ELF | `q-find-prior-decision` | `find_prior_decision` | `decision-qdrant-derived` | `` | `decision-qdrant-derived` | `true` | `6.66385 ms` |
| ELF | `q-compare-project-status` | `compare_project_status` | `pr-110-review` | `recovery-xy640-ledger` | `recovery-xy640-ledger` | `true` | `6.344976 ms` |
| ELF | `q-detect-contradiction-update` | `detect_contradiction_update` | `decision-xy818-supersedes` | `` | `decision-xy818-supersedes` | `true` | `6.429079 ms` |

## Result Semantics

- `pass`: every encoded check for the selected project and profile passed.
- `fail`: clone, install, import, build, retrieval, lifecycle, recovery, concurrency, soak, resource-envelope, or another declared check failed.
- `incomplete`: the encoded check could not complete without extra provider keys, host integration, native dependency support, durable runtime wiring, or more adapter work.

`incomplete` is not a pass; treat it as benchmark wiring debt.
