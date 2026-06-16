# Dreaming-Readiness Stage Ledger - June 16, 2026

Goal: Define the Decodex benchmark gate for Dreaming-inspired ELF memory-system
optimization stages.
Read this when: You are starting or finishing a staged memory improvement lane and
need the baseline command matrix, typed evidence status, post-stage outcome, and
report shape required before claiming the stage improved.
Inputs: `docs/research/2026-06-16-dreaming-readiness-stage-ledger.json`, the June 11
competitor-strength, temporal-history, and iteration-direction reports, the XY-905
June 16 live temporal reconciliation report, the consolidation proposal spec, the
memory summary spec, the XY-953 proactive brief scoring report, and the checked-in
real-world fixture suites.
Outputs: A stage-by-stage ledger that downstream issues can update with
`improved`, `regressed`, `unchanged`, `blocked`, or `not_tested` judgments.

## Executive Judgment

This ledger does not claim a broad product win. It records the gate later product
lanes must pass before they can claim a Dreaming or competitor-inspired stage is done,
and now includes the XY-905 post-stage result for live temporal reconciliation.

Current stage status:

- `improved`: current-vs-historical correctness, preference evolution, reviewable
  consolidation, memory-summary/top-of-mind fixture readback, and proactive brief
  fixture scoring.
- `regressed`: none.
- `unchanged`: deletion/TTL/tombstone behavior and the final competitor retest
  baseline.
- `blocked`: scheduled-memory-task readiness.
- `not_tested`: none.

The known live `memory_evolution` loss is now repaired for the encoded ELF live
adapter slice: the XY-905 run passes all six memory-evolution jobs and reports
current, historical, rationale, tombstone, invalidation, selected, dropped, and
non-narrated evidence fields. This is not a private-corpus, hosted memory, or broad
competitor-superiority claim.

Reviewable consolidation is also improved for the narrow ELF self-check: XY-934 adds
service-backed proposal materialization, source lineage, confidence/usefulness,
unsupported-claim flags, apply/defer/discard audit transitions, and zero source
mutations. Direct competitor runners remain untested or product-reference only.

Memory summary and top-of-mind behavior is improved only at the fixture-backed
contract level: XY-952 adds a reviewable `elf.memory_summary/v1` source-trace fixture
that distinguishes current top-of-mind, background, stale, superseded, tombstoned, and
derived project-profile entries. It does not prove live top-of-mind product behavior or
parity with managed memory products.

Proactive brief readiness is improved only at the fixture-backed benchmark level:
XY-953 adds a direct `proactive_brief` suite with daily project brief, resume-work
brief, stale decision audit, stale plan/preference warning, and private-corpus refresh
blocker scenarios. It does not prove OpenAI Pulse parity, hosted managed-memory
parity, background scheduling, or private-corpus production quality.

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

| Stage | Baseline command(s) | Required post-stage command(s) | Baseline counts | Post-stage counts | Judgment | Next optimization direction |
| --- | --- | --- | --- | --- | --- | --- |
| Current-vs-historical correctness | `cargo make real-world-memory-evolution`; `cargo make real-world-memory-live-adapters` | Same commands; publish post-stage JSON and Markdown evidence | `pass=1`, `wrong_result=5`, `blocked=0`, `not_tested=0`, `not_encoded=0` | `pass=6`, `wrong_result=0`, `blocked=0`, `not_tested=0`, `not_encoded=0` | `improved` | Move from benchmark materialization into service-native temporal reconciliation APIs and compare against mem0/OpenMemory history and Graphiti/Zep temporal graph evidence without broad superiority claims. |
| Preference evolution and correction history | `cargo make real-world-memory-evolution`; `cargo make real-world-memory-live-adapters`; `cargo make openmemory-ui-export-readback` | Same commands; include mem0/OpenMemory boundary evidence | `pass=0`, `wrong_result=1`, `blocked=0`, `not_tested=0`, `not_encoded=0` | `pass=1`, `wrong_result=0`, `blocked=0`, `not_tested=0`, `not_encoded=0` | `improved` | Measure preference correction against mem0/OpenMemory history and UI/export surfaces before making any broader history-quality claim. |
| Deletion, TTL, and tombstone behavior | `cargo make real-world-memory`; `cargo make real-world-memory-live-adapters` | Same commands | `pass=1`, `wrong_result=0`, `blocked=0`, `not_tested=0`, `not_encoded=0` | `pass=1`, `wrong_result=0`, `blocked=0`, `not_tested=0`, `not_encoded=0` | `unchanged` | Extend tombstone and TTL readback beyond the single encoded job into update/delete/recreate history cases. |
| Reviewable consolidation | `cargo make real-world-memory-consolidation` | `cargo make real-world-memory-consolidation`; `cargo make real-world-memory-live-consolidation`; `cargo make real-world-memory-live-adapters` | `pass=4`, `wrong_result=0`, `blocked=0`, `not_tested=1`, `not_encoded=1` | `pass=4`, `wrong_result=0`, `blocked=0`, `not_tested=0`, `not_encoded=0` | `improved` | Keep Dreaming output derived and reviewable, and add direct competitor/reference runners only when they emit comparable source ids, confidence, unsupported-claim flags, and review audit artifacts. |
| Memory summary and top-of-mind behavior | `cargo make real-world-memory-knowledge`; `cargo make real-world-memory-core-archival` | `cargo make real-world-memory-summary`; `cargo make real-world-memory-knowledge`; `cargo make real-world-memory-core-archival`; `cargo make real-world-memory-live-adapters` | `pass=8`, `wrong_result=0`, `blocked=0`, `not_tested=1`, `not_encoded=1` | `pass=9`, `wrong_result=0`, `blocked=0`, `not_tested=0`, `not_encoded=0` | `improved` | Move from fixture-backed summary/source-trace readback into service-native admin readback and later live top-of-mind behavior; do not turn hidden summaries into authoritative memory. |
| Proactive brief readiness | `cargo make real-world-first-generation-oss`; `cargo make real-world-job-operator-ux` | `cargo make real-world-memory-proactive-brief`; `cargo make real-world-memory`; `cargo test -p elf-eval --test real_world_job_benchmark -- --test-threads=1` | `pass=0`, `wrong_result=0`, `blocked=0`, `not_tested=1`, `not_encoded=1` | `pass=4`, `wrong_result=0`, `blocked=1`, `not_tested=0`, `not_encoded=0`; evidence-ref/freshness/rationale coverage `1.000`; invalid-current and tombstone violations `0` | `improved` | Move from fixture-backed proactive brief scoring into service-native generated brief readback and later live adapter materialization; keep scheduling and private-corpus refresh behind owned lanes and operator inputs. |
| Scheduled memory task readiness | `cargo make real-world-memory-consolidation` | `cargo make real-world-memory-consolidation`; `cargo make real-world-memory-live-adapters` | `pass=0`, `wrong_result=0`, `blocked=1`, `not_tested=0`, `not_encoded=0` | not run by XY-905 | `blocked` | Scheduled runs are future work; start with queued derived proposal runs and keep operator review mandatory. |
| Final competitor retest status | `cargo make real-world-memory-live-adapters`; `cargo make real-world-first-generation-oss`; `cargo make real-world-memory-graph-rag`; `cargo make openmemory-ui-export-readback`; `cargo make baseline-production-private-addendum` when operator input exists | Same commands; private/provider commands may remain typed blocked under XY-930 | `pass=22`, `wrong_result=5`, `blocked=2`, `not_tested=11`, `not_encoded=11` | partial XY-905 evidence: ELF live adapter `pass=40`, `wrong_result=0`, `blocked=5`, `not_encoded=10` | `unchanged` | Rerun the broader competitor matrix after each optimization; the XY-905 live adapter improvement does not replace private/provider or external competitor gates. |

## Evidence Anchors

| Stage | Evidence file(s) |
| --- | --- |
| Current-vs-historical correctness | `docs/guide/benchmarking/2026-06-16-live-temporal-reconciliation-report.md`; `docs/research/2026-06-16-live-temporal-reconciliation-report.json`; `docs/guide/benchmarking/2026-06-11-temporal-history-competitor-gap-report.md`; `docs/research/2026-06-11-temporal-history-competitor-gap-report.json`; `docs/guide/benchmarking/2026-06-11-competitor-strength-adoption-report.md` |
| Preference evolution and correction history | `docs/guide/benchmarking/2026-06-16-live-temporal-reconciliation-report.md`; `docs/research/2026-06-16-live-temporal-reconciliation-report.json`; `docs/guide/benchmarking/2026-06-11-temporal-history-competitor-gap-report.md`; `docs/guide/benchmarking/2026-06-11-mem0-openmemory-history-ui-export-report.md`; `docs/research/2026-06-11-temporal-history-competitor-gap-report.json` |
| Deletion, TTL, and tombstone behavior | `docs/guide/benchmarking/2026-06-16-live-temporal-reconciliation-report.md`; `docs/research/2026-06-16-live-temporal-reconciliation-report.json`; `docs/guide/benchmarking/2026-06-11-temporal-history-competitor-gap-report.md`; `docs/guide/benchmarking/2026-06-11-measurement-coverage-audit.md` |
| Reviewable consolidation | `docs/spec/system_consolidation_proposals_v1.md`; `apps/elf-eval/fixtures/real_world_memory/consolidation/`; `docs/guide/benchmarking/2026-06-16-live-consolidation-proposal-scoring-report.md`; `docs/research/2026-06-16-live-consolidation-proposal-scoring-report.json` |
| Memory summary and top-of-mind behavior | `docs/spec/system_memory_summary_v1.md`; `apps/elf-eval/fixtures/real_world_memory/memory_summary/`; `apps/elf-eval/fixtures/real_world_memory/knowledge/`; `apps/elf-eval/fixtures/real_world_memory/core_archival_memory/`; `docs/guide/benchmarking/2026-06-11-competitor-strength-adoption-report.md` |
| Proactive brief readiness | `docs/guide/benchmarking/2026-06-16-proactive-brief-scoring-report.md`; `docs/research/2026-06-16-proactive-brief-scoring-report.json`; `apps/elf-eval/fixtures/real_world_memory/proactive_brief/`; `docs/research/2026-06-08-agent-memory-selection.json`; `docs/guide/benchmarking/2026-06-11-first-generation-oss-continuity-source-store-report.md` |
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
- The current ledger preserves typed non-pass states and records the XY-905 live
  memory-evolution improvement.
- The current ledger records the XY-952 fixture-backed memory-summary/source-trace
  contract improvement.
- The current ledger records the XY-953 fixture-backed proactive brief scoring
  improvement with source refs, freshness/currentness markers, reject/defer rationale,
  and typed private-corpus blocking.
- Fixture-backed knowledge and core/archival jobs can be used as regression guards for
  report shape.
- Reviewable consolidation now has ELF live service-backed proposal scoring evidence,
  with direct competitor runners still untested.

Not allowed:

- Do not claim this ledger proves preference history against mem0/OpenMemory,
  live top-of-mind behavior, live proactive brief behavior, scheduled tasks,
  private-corpus gates, hosted memory, broad consolidation superiority, or competitor
  adapters.
- Do not claim fixture-backed proactive brief scoring proves OpenAI Pulse parity or
  hosted managed-memory parity.
- Do not claim ELF has full-suite live real-world pass evidence.
- Do not claim private-corpus or provider-backed production quality without the
  operator-owned inputs required by XY-930.
- Do not claim fixture-only or smoke-only evidence proves broad competitor
  superiority.
