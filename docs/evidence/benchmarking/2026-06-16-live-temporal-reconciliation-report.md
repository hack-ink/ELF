---
type: Evidence
title: "Live Temporal Reconciliation Report - June 16, 2026"
description: "Checked-in benchmark evidence record: Live Temporal Reconciliation Report - June 16, 2026."
resource: docs/evidence/benchmarking/2026-06-16-live-temporal-reconciliation-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-18
tags:
  - docs
  - evidence
  - benchmarking
---
# Live Temporal Reconciliation Report - June 16, 2026

Goal: Record the XY-905 live memory-evolution before/after result and trace contract.
Read this when: You need the current evidence for ELF live current-vs-historical,
supersession, rationale, tombstone, and invalidation behavior.
Inputs: `cargo make real-world-memory-evolution`, `cargo make
real-world-memory-live-adapters`, and
`docs/evidence/benchmarking/2026-06-16-live-temporal-reconciliation-report.md`.
Outputs: A scoped benchmark result for ELF live `memory_evolution` only.

## Executive Judgment

XY-905 improves the encoded ELF live `memory_evolution` slice. The fresh Docker live
adapter sweep shows ELF passing all six memory-evolution jobs with current,
historical, rationale, tombstone, invalidation, selected, dropped, and non-narrated
evidence fields exposed.

This is not a broad competitor-superiority claim. It does not prove ELF beats
Graphiti/Zep, mem0/OpenMemory, Letta, qmd broadly, hosted memory products, private
corpus gates, or provider-backed production quality.

## Commands

| Command | Result | Main artifact |
| --- | --- | --- |
| `cargo test -p elf-eval --test real_world_job_benchmark -- --test-threads=1` | pass | stdout |
| `cargo make real-world-memory-evolution` | pass | `tmp/real-world-memory/evolution-report.json` |
| `cargo make real-world-memory-live-adapters` | pass | `tmp/real-world-memory/live-adapters/summary.json` |

The live adapter run completed in 187.57 seconds. It emitted the pre-existing Qdrant
client/server compatibility warning, but the command completed and wrote ELF and qmd
reports.

## Before And After

| Adapter | Stage | Jobs | Status counts | Score mean | Expected evidence recall | Judgment |
| --- | --- | ---: | --- | ---: | ---: | --- |
| ELF live service adapter | June 11 baseline | 6 | `pass=1`, `wrong_result=5` | `0.492` | `1.000` | baseline loss |
| ELF live service adapter | XY-905 post-stage | 6 | `pass=6`, `wrong_result=0` | `1.000` | `1.000` | improved |
| qmd live CLI adapter | June 11 baseline | 6 | `pass=0`, `wrong_result=6` | `0.325` | `0.769` | baseline non-pass |
| qmd live CLI adapter | XY-905 post-stage | 6 | `pass=0`, `wrong_result=6` | `0.325` | `0.769` | unchanged non-pass |

ELF full live adapter summary after XY-905: 55 jobs, 40 pass, 0 wrong_result, 5
blocked, 10 not_encoded, mean score 0.727, expected evidence recall 0.655.

## ELF Memory Evolution Result

| Job | Status | Selected lifecycle evidence |
| --- | --- | --- |
| `memory-evolution-benchmark-verdict-001` | pass | current verdict, historical not-ready verdict, update rationale |
| `memory-evolution-deploy-method-001` | pass | current production runbook, historical quickstart, supersession rationale |
| `memory-evolution-issue-state-001` | pass | current done state, historical blocked state, resolution rationale |
| `memory-evolution-preference-001` | pass | current preference, historical preference, rationale |
| `memory-evolution-relation-temporal-001` | pass | current owner, historical owner, temporal rationale |
| `memory-evolution-delete-ttl-001` | pass | current plan, tombstone, invalidation evidence |

The suite reports conflict detection count `5`, update rationale availability count
`6`, temporal-validity not-encoded count `0`, and history-readback encoded count `1`.

## Trace Contract

The report JSON now exposes selected lifecycle evidence fields:

- `selected_current_evidence`
- `selected_historical_evidence`
- `selected_rationale_evidence`
- `selected_tombstone_evidence`
- `selected_invalidation_evidence`
- `conflict_candidate_evidence`
- `retrieved_but_dropped_evidence`
- `selected_but_not_narrated_evidence`

The ELF materialization artifact also records:

- current winner evidence
- historical loser evidence
- supersession rationale evidence
- tombstone and invalidation evidence
- retrieved, selected, absent, retrieved-but-dropped, selected-but-not-narrated, and
  lifecycle-demoted evidence ids

The scorer still fails selected-but-not-narrated conflicts as `wrong_result`; the
targeted integration test mutates a passing preference fixture to select the
historical evidence without attaching it to the current-preference conflict claim and
confirms the job remains `wrong_result`.

## Ledger Update

The XY-951 ledger now records:

- `current_vs_historical_correctness`: improved from `pass=1`, `wrong_result=5` to
  `pass=6`, `wrong_result=0`.
- `preference_evolution`: improved from `pass=0`, `wrong_result=1` to `pass=1`,
  `wrong_result=0`.
- `deletion_ttl_tombstone_behavior`: unchanged at `pass=1`, `wrong_result=0`, with
  tombstone and invalidation evidence now explicit in report fields.

## Claim Boundaries

Allowed:

- ELF live `memory_evolution` now passes all six encoded jobs in the XY-905 run.
- The trace/readback contract distinguishes current, historical, rationale,
  tombstone, invalidation, selected, dropped, non-narrated, and lifecycle-demoted
  evidence.
- qmd remains `wrong_result` on this memory-evolution slice in the same run.

Not allowed:

- Do not claim ELF broadly beats qmd as a memory system.
- Do not claim ELF beats Graphiti/Zep, mem0/OpenMemory, or Letta.
- Do not claim private-corpus, hosted memory, OpenMemory UI/export, or provider-backed
  production quality from this issue.

## Next Direction

Move this reconciliation contract from benchmark materialization toward service-native
temporal answer/readback APIs. Then compare against mem0/OpenMemory history and
Graphiti/Zep temporal graph gates before making broader history or temporal-memory
claims.
