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
  profiles, private addendum publication, opt-in 10k/100k backfill, and soak
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
- `2026-06-10-real-world-comparison-report.md`: checked-in post-P1 real-world
  comparison report with aggregate fixture evidence, external-adapter evidence classes,
  remaining typed gaps, and adoption implications.
- `2026-06-10-live-real-world-sweep-report.md`: XY-880 full-suite live real-world
  sweep report for ELF and qmd, showing per-suite live pass and typed non-pass states
  without claiming full-suite live parity.
- `2026-06-10-production-adoption-refresh.md`: XY-884 post-adapter production
  adoption refresh that keeps the decision at adopt with bounded caveats and separates
  fixture, live adapter, private corpus, credentialed, blocked, and research-gate
  evidence.
- `2026-06-11-competitor-strength-evidence-matrix.md`: XY-897 competitor-strength
  matrix contract that maps every tracked memory/RAG/graph project to its strongest
  scenario, current evidence class, typed blockers, next measurement gate, and ELF
  borrow-if-stronger direction.
- `2026-06-11-elf-iteration-direction-from-competitor-benchmarks.md`: current
  optimization-direction report that translates measured benchmark data and competitor
  strengths into prioritized ELF iteration themes and explicit non-claims.
- `2026-06-11-measurement-coverage-audit.md`: fresh coverage audit that separates
  current measured ELF/qmd data, fixture evidence, external adapter ledger coverage,
  scenario non-claims, and the next measurement reports needed before stronger
  competitor claims.
- `2026-06-11-elf-qmd-retrieval-debug-profile.md`: fresh ELF/qmd retrieval-debug
  profile with real-world retrieval-suite evidence, 480-document stress baseline
  evidence, qmd top-10 artifact inspection, and explicit rerank/fusion non-claims.
- `2026-06-11-elf-qmd-memory-evolution-diagnostic.md`: fresh ELF/qmd
  memory-evolution diagnostic showing fixture pass, live ELF/qmd current-vs-historical
  wrong-result patterns, qmd tombstone evidence miss, and temporal-reconciliation
  iteration directions.
- `2026-06-11-temporal-history-competitor-gap-report.md`: fresh report-only
  temporal/history competitor-gap report that updates the mem0 basic lifecycle result,
  records Graphiti/Zep and Letta claim boundaries, and turns qmd, mem0/OpenMemory,
  Graphiti/Zep, Letta, and adjacent project strengths into benchmark-gated ELF
  optimization directions.
- `2026-06-11-qmd-openviking-strength-profile-report.md`: XY-899 strength-profile
  report that separates qmd retrieval quality from debug/replay ergonomics, records
  qmd wrong-result diagnosis classes, and preserves OpenViking context-trajectory
  surfaces as `not_tested` until staged/hierarchical evidence is encoded.
- `2026-06-11-elf-qmd-trace-replay-diagnostics-report.md`: XY-923 trace-level
  replay and wrong-result diagnostics report that scores qmd top-10/replay artifact
  ergonomics against ELF trace/admin surfaces while keeping retrieval correctness,
  rerank, fusion, candidate-drop, and typed non-pass boundaries separate.
- `2026-06-11-first-generation-oss-adapter-promotion-report.md`: XY-898
  first-generation OSS adapter promotion report that updates agentmemory,
  mem0/OpenMemory, memsearch, and claude-mem with fresh scenario-level baseline
  evidence and ELF win/tie/loss/untested positions without converting baseline-only
  evidence into real-world suite wins.
- `2026-06-11-graph-rag-scored-smoke-adapter-report.md`: XY-900 graph/RAG
  scored-smoke adapter report that promotes RAGFlow, LightRAG, GraphRAG,
  Graphiti/Zep, and graphify smoke contracts into scored or typed non-pass
  `real_world_job` adapter reports without converting smoke evidence into quality
  claims.
- `2026-06-11-competitor-strength-adoption-report.md`: XY-901 final
  competitor-strength adoption report with the bounded personal-production decision,
  scenario-level win/tie/loss/not-tested matrix, claim boundaries, and optimization
  issue queue.
- `2026-06-11-mem0-openmemory-history-ui-export-report.md`: XY-924
  mem0/OpenMemory local OSS history, preference-correction, deletion-audit,
  personalization, and export-readback comparison with normalized
  win/tie/loss/not-tested/blocked/non-goal outcomes and explicit hosted/UI/graph
  non-claims.
- `real_world_agent_memory_benchmark.md`: operator overview for the v1 real-world
  agent memory benchmark contract, including suite taxonomy, typed report states,
  knowledge-compilation fixture tasks, and the production-ops fixture target.
- `real_world_memory_evolution.md`: run and interpret the checked-in memory evolution
  jobs for current facts, historical facts, stale traps, conflicts, update rationales,
  and temporal graph limitations.

## Update Rules

- Add a dated report when a new run changes README-level claims.
- Keep generated raw JSON under `tmp/live-baseline/`; commit only reviewed Markdown
  summaries and durable scripts.
- Keep generated real-world job smoke JSON and Markdown under `tmp/real-world-job/`;
  commit fixture schemas, smoke fixtures, runner code, and durable docs only.
- Keep generated real-world memory trust/personalization/knowledge/production-ops JSON
  and Markdown under `tmp/real-world-memory/`; commit fixtures, runner code, and
  durable docs only.
- Link the newest decision-relevant report from README and this index.
- When benchmark semantics change, update `live_baseline_benchmark.md` and the
  relevant spec before publishing a new result.
- Real-world job benchmark changes are governed by
  `docs/spec/real_world_agent_memory_benchmark_v1.md`; keep this guide as routing and
  do not duplicate the normative schema here.
