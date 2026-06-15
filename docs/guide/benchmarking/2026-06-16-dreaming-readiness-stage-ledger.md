# Dreaming-Readiness Stage Ledger - June 16, 2026

Goal: Define the Decodex benchmark gate for Dreaming-inspired ELF memory-system
optimization stages.
Read this when: You are starting or finishing a staged memory improvement lane and
need the baseline command matrix, typed evidence status, and report shape required
before claiming the stage improved.
Inputs: `docs/research/2026-06-16-dreaming-readiness-stage-ledger.json`, the June 11
competitor-strength, temporal-history, and iteration-direction reports, the
consolidation proposal spec, and the checked-in real-world fixture suites.
Outputs: A stage-by-stage ledger that downstream issues can update with
`improved`, `regressed`, `unchanged`, `blocked`, or `not_tested` judgments.

## Executive Judgment

This ledger does not claim a new product win. It creates the gate later product lanes
must pass before they can claim a Dreaming or competitor-inspired stage is done.

Current baseline:

- `improved`: none.
- `regressed`: none.
- `unchanged`: current-vs-historical correctness, preference evolution,
  deletion/TTL/tombstone behavior, and the final competitor retest baseline.
- `blocked`: scheduled-memory-task readiness.
- `not_tested`: reviewable consolidation beyond fixtures, memory-summary/top-of-mind
  live behavior, and proactive brief readiness.

The important known loss is preserved: live `memory_evolution` is not solved until
XY-905 changes behavior and reruns the live gate. The current ELF live adapter passes
only the delete/TTL tombstone job and keeps five current-vs-historical jobs as
`wrong_result`.

## Ledger Rules

- Every downstream Dreaming or competitor-improvement stage must write a post-stage
  JSON report and Markdown summary before claiming phase completion.
- The report must compare against the baseline counts in
  `docs/research/2026-06-16-dreaming-readiness-stage-ledger.json`.
- The comparison judgment must be one of `improved`, `regressed`, `unchanged`,
  `blocked`, or `not_tested`.
- Typed non-pass labels stay typed. Do not collapse `wrong_result`, `blocked`,
  `not_tested`, `not_encoded`, `incomplete`, `lifecycle_fail`, `unsupported`, or
  `non_goal` into a single pass/fail label.
- Fixture-backed evidence proves benchmark shape only. It does not prove live product
  behavior.
- Private-corpus and provider-backed gates remain typed blocked unless an operator
  supplies explicit inputs; those boundaries are tied to XY-930.

## Stage Command Matrix

| Stage | Baseline command(s) | Required post-stage command(s) | Current counts | Judgment | Next optimization direction |
| --- | --- | --- | --- | --- | --- |
| Current-vs-historical correctness | `cargo make real-world-memory-evolution`; `cargo make real-world-memory-live-adapters` | Same commands; publish post-stage JSON and Markdown evidence | `pass=1`, `wrong_result=5`, `blocked=0`, `not_tested=0` | `unchanged` | XY-905 must make live answers cite current, historical, rationale, and tombstone evidence instead of only retrieving snippets. |
| Preference evolution and correction history | `cargo make real-world-memory-evolution`; `cargo make real-world-memory-live-adapters`; `cargo make openmemory-ui-export-readback` | Same commands; include mem0/OpenMemory boundary evidence | `pass=0`, `wrong_result=1`, `blocked=0`, `not_tested=0` | `unchanged` | Preserve current and superseded preferences with rationale evidence; do not claim ELF beats mem0/OpenMemory history until measured. |
| Deletion, TTL, and tombstone behavior | `cargo make real-world-memory`; `cargo make real-world-memory-live-adapters` | Same commands | `pass=1`, `wrong_result=0`, `blocked=0`, `not_tested=0` | `unchanged` | Preserve the current tombstone pass while repairing adjacent temporal-history wrong results. |
| Reviewable consolidation | `cargo make real-world-memory-consolidation` | `cargo make real-world-memory-consolidation`; `cargo make real-world-memory-live-adapters` | `pass=4`, `wrong_result=0`, `blocked=0`, `not_tested=1` | `not_tested` | Keep Dreaming output derived and reviewable with lineage, confidence, unsupported-claim flags, apply/defer/discard audit, and no source mutation. |
| Memory summary and top-of-mind behavior | `cargo make real-world-memory-knowledge`; `cargo make real-world-memory-core-archival` | Same commands plus `cargo make real-world-memory-live-adapters` | `pass=8`, `wrong_result=0`, `blocked=0`, `not_tested=1` | `not_tested` | Build summaries as cited, rebuildable derived pages or core blocks; do not turn hidden summaries into authoritative memory. |
| Proactive brief readiness | `cargo make real-world-first-generation-oss`; `cargo make real-world-job-operator-ux` | Same commands plus `cargo make real-world-memory-live-adapters` | `pass=0`, `wrong_result=0`, `blocked=0`, `not_tested=1` | `not_tested` | Add direct proactive-brief fixtures before any pass claim; briefs must be source-linked and repairable. |
| Scheduled memory task readiness | `cargo make real-world-memory-consolidation` | `cargo make real-world-memory-consolidation`; `cargo make real-world-memory-live-adapters` | `pass=0`, `wrong_result=0`, `blocked=1`, `not_tested=0` | `blocked` | Scheduled runs are future work; start with queued derived proposal runs and keep operator review mandatory. |
| Final competitor retest status | `cargo make real-world-memory-live-adapters`; `cargo make real-world-first-generation-oss`; `cargo make real-world-memory-graph-rag`; `cargo make openmemory-ui-export-readback`; `cargo make baseline-production-private-addendum` when operator input exists | Same commands; private/provider commands may remain typed blocked under XY-930 | `pass=22`, `wrong_result=5`, `blocked=2`, `not_tested=11` | `unchanged` | Rerun the relevant competitor matrix after each optimization and update improved/regressed/unchanged/blocked/not-tested buckets. |

## Evidence Anchors

| Stage | Evidence file(s) |
| --- | --- |
| Current-vs-historical correctness | `docs/guide/benchmarking/2026-06-11-temporal-history-competitor-gap-report.md`; `docs/research/2026-06-11-temporal-history-competitor-gap-report.json`; `docs/guide/benchmarking/2026-06-11-competitor-strength-adoption-report.md` |
| Preference evolution and correction history | `docs/guide/benchmarking/2026-06-11-temporal-history-competitor-gap-report.md`; `docs/guide/benchmarking/2026-06-11-mem0-openmemory-history-ui-export-report.md`; `docs/research/2026-06-11-temporal-history-competitor-gap-report.json` |
| Deletion, TTL, and tombstone behavior | `docs/guide/benchmarking/2026-06-11-temporal-history-competitor-gap-report.md`; `docs/guide/benchmarking/2026-06-11-measurement-coverage-audit.md` |
| Reviewable consolidation | `docs/spec/system_consolidation_proposals_v1.md`; `apps/elf-eval/fixtures/real_world_memory/consolidation/`; `docs/guide/benchmarking/2026-06-11-competitor-strength-adoption-report.md` |
| Memory summary and top-of-mind behavior | `apps/elf-eval/fixtures/real_world_memory/knowledge/`; `apps/elf-eval/fixtures/real_world_memory/core_archival_memory/`; `docs/guide/benchmarking/2026-06-11-competitor-strength-adoption-report.md` |
| Proactive brief readiness | `docs/research/2026-06-08-agent-memory-selection.json`; `docs/guide/benchmarking/2026-06-11-first-generation-oss-continuity-source-store-report.md` |
| Scheduled memory task readiness | `docs/spec/system_consolidation_proposals_v1.md`; `docs/research/2026-06-08-agent-memory-selection.json` |
| Final competitor retest status | `docs/guide/benchmarking/2026-06-11-competitor-strength-adoption-report.md`; `docs/research/2026-06-11-competitor-strength-adoption-report.json`; `docs/guide/benchmarking/2026-06-11-graph-rag-scored-smoke-adapter-report.md`; `docs/guide/benchmarking/2026-06-11-first-generation-oss-continuity-source-store-report.md` |

## Report Shape For Downstream Issues

Downstream stage reports should use the same fields as the JSON ledger:

- `stage_id`
- `baseline_commands`
- `post_stage_commands`
- `evidence_files`
- `baseline_counts`
- `post_stage_counts`
- `comparison_judgment`
- `regression_rule`
- `improvement_rule`
- `next_optimization_direction`

If a stage cannot run because credentials, private corpus, provider setup, or a
product surface is absent, record `blocked` or `not_tested` with the concrete blocker.
Do not silently drop the stage from the report.

## Claim Boundaries

Allowed:

- The Dreaming-readiness gate exists and names required stage commands and evidence
  files.
- The current baseline preserves typed non-pass states and the known live
  memory-evolution loss.
- Fixture-backed consolidation, knowledge, and core/archival jobs can be used as
  regression guards for report shape.

Not allowed:

- Do not claim this ledger fixes temporal reconciliation, preference history,
  consolidation, proactive briefs, scheduled tasks, or competitor adapters.
- Do not claim ELF has full-suite live real-world pass evidence.
- Do not claim private-corpus or provider-backed production quality without the
  operator-owned inputs required by XY-930.
- Do not claim fixture-only or smoke-only evidence proves broad competitor
  superiority.
