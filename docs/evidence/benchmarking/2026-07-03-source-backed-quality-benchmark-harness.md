---
type: Evidence
title: "Source-Backed Memory Quality Benchmark Harness"
description: "Evidence report for the XY-1155 source-backed memory quality benchmark harness, gate metrics, and review result."
resource: docs/evidence/benchmarking/2026-07-03-source-backed-quality-benchmark-harness.md
status: active
authority: evidence
owner: benchmarking
last_verified: 2026-07-03
tags:
  - docs
  - evidence
  - benchmarking
  - source-backed-memory
source_refs: []
code_refs:
  - Makefile.toml
  - apps/elf-eval/src/bin/real_world_job_benchmark/main.rs
  - apps/elf-eval/src/bin/real_world_job_benchmark/source_backed_quality.rs
  - apps/elf-eval/src/bin/real_world_job_benchmark/markdown/source_backed_quality.rs
  - apps/elf-eval/src/bin/real_world_job_benchmark/source_backed_quality_reports.rs
  - docs/spec/agent_memory_knowledge_system_v1.md
  - docs/spec/real_world_agent_memory_benchmark_v1.md
related:
  - docs/evidence/benchmarking/2026-07-03-final-source-backed-project-memory-closeout-report.md
  - docs/spec/agent_memory_knowledge_system_v1.md
drift_watch:
  - Makefile.toml
  - apps/elf-eval/src/bin/real_world_job_benchmark/main.rs
  - apps/elf-eval/src/bin/real_world_job_benchmark/source_backed_quality.rs
  - apps/elf-eval/src/bin/real_world_job_benchmark/markdown/source_backed_quality.rs
  - apps/elf-eval/src/bin/real_world_job_benchmark/source_backed_quality_reports.rs
  - docs/spec/agent_memory_knowledge_system_v1.md
---
# Source-Backed Memory Quality Benchmark Harness - July 3, 2026

## Purpose

This report records the XY-1155 executable benchmark harness update for ELF's
source-backed project memory quality gate.

## Command

Run:

```sh
cargo make source-backed-memory-quality
```

The task generates JSON, validates the source-backed quality gate with
`validate-source-backed-quality`, and then emits:

- `tmp/source-backed-memory-quality/report.json`
- `tmp/source-backed-memory-quality/report.md`

## Added Report Surface

The real-world job benchmark now emits
`elf.source_backed_memory_quality_benchmark/v1` at
`/source_backed_quality`.

The surface aggregates the checked-in real-world memory fixtures into the
XY-1155 product-quality metrics:

- expected evidence recall
- precision@5
- irrelevant context ratio
- source-ref coverage
- stale suppression rate
- correction persistence rate
- delete/tombstone suppression rate
- unsupported claim rate
- cross-scope leak count
- journal-only authority claim count
- Context Pack activation precision/recall
- activation trace coverage
- latency and cost

The two hard-fail counters are:

- `cross_scope_leak_count`
- `journal_only_authority_claim_count`

Both must be zero for the source-backed quality gate to pass.

The gate also fails when any required scenario is non-pass, any required job
uses a hard-fail trap, or the Context Pack routing decisions are missing or
incorrect.

## Scenario Coverage

The report also emits required scenario coverage for:

- source-backed recall
- source-backed memory promotion
- stale suppression
- correction persistence
- supersession
- delete/tombstone suppression
- cross-scope/private leak traps
- read-profile downgrade traps
- pinned Context Pack traps
- journal-only current-fact traps
- work-resume where-stopped readback
- Dreaming no-silent-mutation boundaries
- Context Pack activation/suppression/disabled/stale routing
- stale derived knowledge after source changes
- authoritative revalidation
- Recall Debug reason codes and privacy

## Current Local Run

The local XY-1155 run produced a passing
`source_backed_quality.result_state = "pass"` with:

- expected evidence recall: `1.0`
- precision@5: `0.414`
- irrelevant context ratio: `0.0`
- source-ref coverage: `1.0`
- stale suppression rate: `1.0`
- correction persistence rate: `1.0`
- delete/tombstone suppression rate: `1.0`
- unsupported claim rate: `0.0`
- cross-scope leak count: `0`
- journal-only authority claim count: `0`
- Context Pack activation precision: `1.0`
- Context Pack activation recall: `1.0`
- activation trace coverage: `1.0`
- Context Pack routing decisions: `7/7` traced, `0` incorrect, covering enabled,
  suppressed, disabled, stale-suppressed, blocked, and pinned-ineligible states
- mean latency: `2.864ms`

The aggregate benchmark still preserves typed non-pass rows elsewhere in the
report. Those rows remain blocker evidence and do not support an unqualified
leaderboard or product-runtime superiority claim.

## Review Result

A read-only skeptic review first blocked the change because the benchmark task
generated reports without gating failures, hard-fail handling did not reject all
required non-pass scenarios, and Context Pack activation metrics were too
tag-derived.

The follow-up implementation added a `validate-source-backed-quality` command,
wired it into `cargo make source-backed-memory-quality`, hard-fails all required
non-pass scenarios and required job trap usage, counts `private_scope_leak` in
scope leak metrics, and derives Context Pack metrics from structured routing
decisions. A second read-only skeptic review returned `pass` with no P0/P1
findings.
