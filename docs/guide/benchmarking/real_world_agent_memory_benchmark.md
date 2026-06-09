# Real-World Agent Memory Benchmark

Goal: Explain the v1 real-world agent memory benchmark suite and route implementation
work to the governing spec.
Read this when: You need to create jobs, extend benchmark suites, interpret reports,
or understand why retrieval-only comparisons are insufficient.
Inputs: `docs/spec/real_world_agent_memory_benchmark_v1.md`, current live baseline
reports, external project comparison docs, and the intended user-job scenario.
Depends on: `docs/spec/real_world_agent_memory_benchmark_v1.md`,
`live_baseline_benchmark.md`, and `docs/guide/research/comparison_external_projects.md`.
Outputs: Operator-facing suite overview, bias explanation, and implementation routing.

## Governing Spec

The authoritative contract is:

- `docs/spec/real_world_agent_memory_benchmark_v1.md`

Use the spec for field names, suite ids, report states, scoring rules, and claim
boundaries. This guide is only an operator map.

## Why This Suite Exists

The current live baseline proves useful behavior: ELF and qmd can pass the encoded
Docker smoke checks, and ELF can pass provider-backed synthetic, stress, backfill,
restore, and lifecycle checks. That evidence remains valid for the existing benchmark.

It is incomplete for real agent work. A memory system can retrieve the right chunk and
still fail the user's job by repeating completed work, trusting stale evidence, missing
a blocker, leaking private context, or inventing a decision that was never recorded.

The real-world suite changes the unit from a query to a `real_world_job`:

- corpus
- timeline
- prompt
- expected answer
- required evidence
- negative traps
- scoring rubric
- allowed uncertainty

This shape rewards systems that help agents resume, decide, debug, update stale memory,
compile knowledge, and state honest uncertainty.

## Suite Overview

| Suite | What It Tests | Example Job |
| --- | --- | --- |
| Trust/source-of-truth | Provenance, rebuildability, and derived-index boundaries. | Restore a note after index rebuild and cite authoritative source evidence. |
| Work resume | Resuming agent work without repeating completed steps. | Identify the next action after a retained lane failure. |
| Project decisions | Current decisions, rationale, reversals, and caveats. | Explain why a benchmark gate uses typed failures. |
| Retrieval | Task-relevant search with decoys and alternates. | Answer a task query while avoiding near-duplicate project evidence. |
| Memory evolution | Update, delete, expiry, contradiction, and history behavior. | Report what superseded an old fact and suppress deleted memory. |
| Consolidation | Reviewable derived memories without hidden mutation. | Produce a proposal with lineage and unsupported-claim flags. |
| Knowledge compilation | Evidence-linked project/entity/concept pages. | Compile current project status with timeline and stale-section lint. |
| Operator debugging UX | Ability to diagnose wrong results without raw store access. | Show which retrieval stage dropped expected evidence. |
| Capture/integration | Accuracy of hooks, imports, exclusions, and write policies. | Capture a session decision while excluding private spans. |
| Production ops | Backfill, restore, cold start, resource, and bounded-failure behavior. | Resume interrupted import without duplicate source notes. |
| Personalization | Scoped preferences without cross-tenant leakage. | Apply the user's current preference and ignore another project's note. |

## External Reference Mapping

The suite uses external strengths as references, not as winners:

- ELF: evidence-bound writes, deterministic ingestion boundaries, source-of-truth plus
  rebuildable index, production ops, and evaluation tooling.
- qmd: local retrieval quality, query expansion/routing, weighted fusion, rerank, and
  transparent debug ergonomics.
- agentmemory: cross-agent hooks, coding-agent continuity, local viewer, consolidation
  lifecycle, and observability console.
- claude-mem: progressive disclosure, automatic capture loop, local inspection, and
  operator comfort.
- OpenViking: filesystem context model, hierarchical retrieval, staged trajectory, and
  session iteration.
- mem0: multi-entity scoping, lifecycle history, optional graph context, hosted/OpenMemory
  ecosystem, and personalization references.
- memsearch: Markdown-first source-of-truth pattern, incremental indexing, and practical
  local hybrid retrieval.
- llm-wiki and gbrain: compiled knowledge pages, query-save/lint loops, current-truth
  plus timeline shape.
- Always-On Memory Agent, Claude Dreams, and Gemini CLI Auto Memory: background
  consolidation patterns, with ELF's requirement that derived outputs remain reviewable.
- Graphiti/Zep, Letta, LangGraph, graphify, and nanograph: temporal facts, core versus
  archival memory, replay mindset, graph-compressed navigation, and typed graph ergonomics.

## Report Interpretation

A real-world benchmark report must preserve typed outcomes:

- `pass`
- `wrong_result`
- `lifecycle_fail`
- `incomplete`
- `blocked`
- `not_encoded`
- `unsupported_claim`

Do not collapse those terms into one leaderboard. `unsupported_claim` is especially
important: it means the system made a substantive claim that the corpus or evidence did
not support. That is a different and higher-risk failure than simply missing a result.

## Implementation Routing

Downstream runner issues can cite the spec directly. They should choose a small suite
slice first, then report every untouched suite as `not_encoded`.

Recommended first increments:

1. Encode one `work_resume` job over the synthetic production corpus.
2. Encode one `retrieval` job with decoys and required evidence.
3. Encode one `memory_evolution` job that proves update/delete/supersession behavior.
4. Add report output for `unsupported_claim` before broadening the suite count.

Current checked-in smoke increment:

```sh
cargo make real-world-job-smoke
```

This parses `apps/elf-eval/fixtures/real_world_memory/work_resume/`, writes
`tmp/real-world-job/real-world-job-smoke-report.json`, and renders
`tmp/real-world-job/real-world-job-smoke-report.md`.

The checked-in fixture slice covers stale worktree resume, Decodex/Linear lane status,
failed command recovery, PR review blocker recovery, exact next-action extraction, and
cross-tool capture boundaries. The smoke report includes suite id, job id, expected
evidence, produced answer/evidence, unsupported-claim count, wrong-result count,
latency/cost fields when available, capture/integration behavior classes, and typed
suite/job statuses. Untouched suites remain `not_encoded`.

Current checked-in aggregate memory increment:

```sh
cargo make real-world-memory
```

This parses `apps/elf-eval/fixtures/real_world_memory/`, writes
`tmp/real-world-memory/real-world-memory-report.json`, and renders
`tmp/real-world-memory/real-world-memory-report.md`.

This command recursively parses all checked-in `real_world_memory` fixture slices,
including the retrieval-quality slice below. The suite currently encodes:

- `trust_source_of_truth`: evidence binding, source refs, and Qdrant rebuild from
  Postgres-held chunk embeddings before answering.
- `work_resume`: stale worktree resume, Decodex/Linear lane status, failed command
  recovery, PR review blocker recovery, and exact next-action extraction.
- `retrieval`: alternate phrasing, distractor-heavy retrieval, multi-hop routing,
  current-versus-obsolete selection, and minimal sufficient context.
- `memory_evolution`: TTL/delete suppression plus current-versus-historical preference,
  issue status, deployment method, benchmark conclusion, and temporal relation cases.
- `operator_debugging_ux`: deliberate wrong-result trace attribution that identifies
  the retrieval stage that demoted expected evidence.
- `capture_integration`: write-policy audit behavior for redaction/private exclusion
  and fixture-backed capture/integration boundary classification.
- `personalization`: scoped stable preference correction without temporary or
  cross-project preference leakage.

The generated report includes evidence coverage, source-ref coverage, quote coverage,
unsupported-claim count, stale retrieval count, stale-answer count, conflict detection
count, update rationale availability, temporal validity `not_encoded` count, scope
correctness, redaction leak count, capture/integration behavior classes, Qdrant
rebuild case/pass counts, expected evidence recall, irrelevant context ratio,
latency/cost, and trace explainability counters. The fixtures include negative traps
for stale blockers, unsupported prior claims, stale deleted facts, stale historical
facts, cross-project preference leakage, private/redacted text leakage, obsolete
retrieval context, and distractor context.

Narrow memory evolution increment:

```sh
cargo make real-world-memory-evolution
```

Artifacts:

```text
tmp/real-world-memory/evolution-report.json
tmp/real-world-memory/evolution-report.md
```

This parses `apps/elf-eval/fixtures/real_world_memory/evolution/` and reports only
the cases added for current-versus-historical interpretation and temporal staleness.
The relation temporal-validity fixture is deliberately `not_encoded` and declares the
graph follow-up instead of claiming a fake graph pass.

Current checked-in retrieval-quality increment:

```sh
cargo make real-world-memory-retrieval
```

This parses `apps/elf-eval/fixtures/real_world_memory/retrieval/`, writes
`tmp/real-world-memory/retrieval-report.json`, and renders
`tmp/real-world-memory/retrieval-report.md`. The fixture set covers alternate
phrasing, distractor-heavy retrieval, multi-hop routing, current-versus-obsolete
selection, minimal sufficient context, and a deliberate wrong-result trace attribution
case. Reports include expected evidence recall, irrelevant context ratio, latency/cost,
and optional trace explainability metadata. The qmd and OpenViking references in these
fixtures are design references only; no parity claim is allowed unless an external
adapter run actually provides evidence.

Operator debugging UX increment:

```sh
cargo make real-world-job-operator-ux
```

Artifacts:

```text
tmp/real-world-job/real-world-job-operator-ux-report.json
tmp/real-world-job/real-world-job-operator-ux-report.md
```

The operator UX fixtures live under
`apps/elf-eval/fixtures/real_world_job/operator_debugging_ux/`. They cover dropped
expected evidence, rerank promotion of a bad candidate, provider latency or failure,
Qdrant rebuild result changes, and misleading relation context. Reports include direct
viewer and admin trace bundle links, steps to root cause, whether raw SQL was needed,
dropped-candidate visibility, trace completeness, repair-action clarity, and any
encoded UX gaps.

Checked-in evidence snapshot:
`docs/guide/benchmarking/2026-06-09-operator-debugging-ux-report.md`.

The same `real-world-memory` target also includes the current consolidation fixtures
under the same fixture root.

Current checked-in consolidation increment:

```sh
cargo make real-world-memory-consolidation
```

This parses `apps/elf-eval/fixtures/real_world_memory/consolidation/`, writes
`tmp/real-world-memory/consolidation/report.json`, and renders
`tmp/real-world-memory/consolidation/report.md`. The consolidation report includes
proposal usefulness, lineage completeness, review action correctness, proposal
unsupported-claim count, executable gap count, and source mutation count. Source
mutation count must remain `0` for proposal-only cases.

These fixtures encode proposal expectations only. They do not claim that a live
scheduled consolidation worker generated the proposals; the report records that missing
primitive as an executable gap with a follow-up issue title.

Current checked-in knowledge-compilation increment:

```sh
cargo make real-world-memory-knowledge
```

This parses `apps/elf-eval/fixtures/real_world_memory/knowledge/`, writes
`tmp/real-world-memory/knowledge-report.json`, and renders
`tmp/real-world-memory/knowledge-report.md`. The fixtures include synthetic project,
entity, concept, and issue-timeline page artifacts. Generated pages are benchmark
artifacts only: every section must cite source evidence or timeline events, or it must
be explicitly flagged unsupported. The report publishes citation coverage, stale claim
detection, rebuild determinism, aggregate backlink counts and page coverage, page
usefulness, unsupported summary count, and untraced section count.

Do not generate large fixtures or update production-adoption verdicts while adding the
contract. The current adoption gate remains an existing benchmark decision until new
real-world job reports are implemented and published.
