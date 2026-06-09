# Benchmarking Guide Index

Goal: Route agents to live benchmark runbooks, report publication steps, and checked-in
benchmark evidence.
Read this when: You need to run, publish, interpret, or extend ELF benchmark evidence
against external memory systems.
Inputs: The benchmark question, selected corpus profile, and whether you need a runbook
or a saved evidence snapshot.
Depends on: `docs/index.md`, `docs/guide/index.md`, and `docs/governance.md`.
Outputs: The smallest benchmarking guide or report needed to continue.

## Use This Index When

- You need to run the live Docker-only benchmark matrix.
- You need to publish a Markdown report from a generated benchmark JSON report.
- You need the checked-in benchmark evidence behind README claims.
- You need to extend the benchmark matrix with new projects, profiles, or lifecycle
  checks.

Do not use benchmark commands as the production operating procedure. For single-user
Docker Compose production start, stop, backup, restore, Qdrant rebuild, rollback, and
cleanup, use `docs/guide/single_user_production.md`.

## Guides And Reports

- `live_baseline_benchmark.md`: run, clean up, publish, and interpret the live
  Docker-only benchmark matrix, including generated public and production-corpus
  profiles.
- `2026-06-09-live-baseline-report.md`: checked-in evidence snapshot for the June 9,
  2026 ELF production-provider stress run and all-project smoke comparison.
- `2026-06-09-production-corpus-report.md`: checked-in synthetic production-corpus
  ELF adoption benchmark report with task queries and evidence IDs.
- `2026-06-09-production-adoption-gate-report.md`: XY-836 production adoption
  decision report with fresh provider-backed synthetic, stress, backfill, restore, and
  external adapter evidence.
- `2026-06-09-operator-debugging-ux-report.md`: checked-in real-world job
  operator-debugging UX report with trace/viewer links, raw-SQL avoidance, root-cause
  step counts, dropped-candidate visibility, and repair-action clarity.
- `real_world_agent_memory_benchmark.md`: operator overview for the v1 real-world
  agent memory benchmark contract, including suite taxonomy and typed report states.

## Update Rules

- Add a dated report when a new run changes README-level claims.
- Keep generated raw JSON under `tmp/live-baseline/`; commit only reviewed Markdown
  summaries and durable scripts.
- Keep generated real-world job smoke JSON and Markdown under `tmp/real-world-job/`;
  commit fixture schemas, smoke fixtures, runner code, and durable docs only.
- Keep generated real-world memory trust/personalization JSON and Markdown under
  `tmp/real-world-memory/`; commit fixtures, runner code, and durable docs only.
- Link the newest decision-relevant report from README and this index.
- When benchmark semantics change, update `live_baseline_benchmark.md` and the
  relevant spec before publishing a new result.
- Real-world job benchmark changes are governed by
  `docs/spec/real_world_agent_memory_benchmark_v1.md`; keep this guide as routing and
  do not duplicate the normative schema here.
