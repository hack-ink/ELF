---
type: Runbook
title: "Real-World Memory Evolution Benchmark"
description: "Run and interpret the checked-in memory evolution real-world job fixtures."
resource: docs/runbook/benchmarking/real_world_memory_evolution.md
status: active
authority: procedural
owner: runbook
last_verified: 2026-06-18
tags:
  - docs
  - runbook
  - benchmarking
---
# Real-World Memory Evolution Benchmark

Goal: Run and interpret the checked-in memory evolution real-world job fixtures.
Read this when: You need to test current facts, historical facts, stale facts,
conflicts, corrected memories, and temporal relation validity.
Inputs: `apps/elf-eval/fixtures/real_world_memory/evolution/`,
`apps/elf-eval/src/bin/real_world_job_benchmark.rs`, and `Makefile.toml`.
Depends on: `docs/spec/real_world_agent_memory_benchmark_v1.md`,
`docs/runbook/benchmarking/real_world_agent_memory_benchmark.md`, and
`docs/evidence/external_memory/comparison_external_projects.md`.
Outputs: `tmp/real-world-memory/evolution-report.json` and
`tmp/real-world-memory/evolution-report.md`.

## Scope

This suite is part of the real-world job benchmark family. It is not a Docker
live-baseline retrieval matrix and does not claim private production readiness.

The checked-in fixture set covers:

- User preference supersession, using mem0-style memory history and Letta-style
  current operating memory as reference patterns.
- Issue state evolution from blocked to done.
- Production deployment guidance superseding a local smoke quickstart.
- Benchmark adoption verdict reversal with a bounded private-corpus caveat.
- Relation fact current-versus-historical ownership with graph-lite temporal
  validity encoded as a normal pass/fail fixture.

The relation case borrows from Graphiti/Zep temporal validity and nanograph typed
query ergonomics while preserving ELF's Postgres source-of-truth and evidence-link
requirements.

## Run

```sh
cargo make real-world-memory-evolution
```

Generated artifacts:

```text
tmp/real-world-memory/evolution-report.json
tmp/real-world-memory/evolution-report.md
```

## Metrics

The runner reports memory evolution counters at summary, suite, and job levels:

- `stale_answer_count`: stale negative traps or stale-current forbidden claims used
  by produced answers.
- `conflict_detection_count`: current-versus-historical conflicts detected with
  both current and historical evidence.
- `update_rationale_available_count`: jobs where the produced answer cites the
  update rationale.
- `temporal_validity_not_encoded_count`: jobs that require temporal graph validity
  but are deliberately declared `not_encoded`; this should be `0` for the checked-in
  evolution fixture set.
- selected lifecycle evidence fields at job level:
  `selected_current_evidence`, `selected_historical_evidence`,
  `selected_rationale_evidence`, `selected_tombstone_evidence`, and
  `selected_invalidation_evidence`.
- `unsupported_claim_count`: existing real-world job unsupported claim counter.

Runnable jobs should have `stale_answer_count = 0`, nonzero conflict detection, and
an update rationale when the fixture provides one. The relation temporal-validity job
should report temporal validity as encoded and pass only when current and historical
relation evidence are distinguished. Delete/TTL jobs should keep tombstone or
invalidation evidence selected while suppressing the deleted fact as a current answer.
