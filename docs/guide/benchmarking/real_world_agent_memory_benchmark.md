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
- `project_decisions`: accepted durable decisions, superseded/reversed decisions,
  old-versus-current validation gates, tradeoff rationale, and bounded caveat or
  uncertainty handling.
- `retrieval`: alternate phrasing, distractor-heavy retrieval, multi-hop routing,
  current-versus-obsolete selection, and minimal sufficient context.
- `memory_evolution`: TTL/delete suppression plus current-versus-historical preference,
  issue status, deployment method, benchmark conclusion, and temporal relation cases.
- `operator_debugging_ux`: trace-backed stage attribution that identifies where
  expected evidence was filtered, demoted, or selected against.
- `capture_integration`: write-policy audit behavior for redaction/private exclusion
  and fixture-backed capture/integration boundary classification.
- `production_ops`: interrupted generated backfill resume, backup/restore plus
  cold-start readback, resource-envelope interpretation, pinned OpenViking local
  embedding runtime/wrong-result classification, missing private manifest `blocked`
  classification, and provider credential boundary `blocked` classification.
- `personalization`: scoped stable preference correction without temporary or
  cross-project preference leakage.

The generated report includes evidence coverage, source-ref coverage, quote coverage,
unsupported-claim count, stale retrieval count, stale-answer count, conflict detection
count, update rationale availability, temporal validity encoding count, scope
correctness, redaction leak count, capture/integration behavior classes, Qdrant
rebuild case/pass counts, expected evidence recall, irrelevant context ratio,
latency/cost, answer-type plus caveat/refusal/uncertainty flags, and trace
explainability counters, production-ops blocked/wrong-result job states, and
private-corpus redaction policy. The fixtures include negative traps for stale
blockers, unsupported prior claims, stale deleted facts, stale historical facts,
cross-project preference leakage, private/redacted text leakage, obsolete retrieval
context, project-decision stale reuse, missing rationale, uncited current policy
claims, overconfident unsupported decision answers, distractor context,
index-only restore claims, private-corpus pass claims without a manifest, and
checked-in credential leakage.

Current checked-in project-decisions increment:

```sh
cargo make real-world-memory-project-decisions
```

This parses `apps/elf-eval/fixtures/real_world_memory/project_decisions/`, writes
`tmp/real-world-memory/project-decisions/report.json`, and renders
`tmp/real-world-memory/project-decisions/report.md`. The fixture set covers:

- accepted decision recovery with required rationale;
- superseded decision recovery where historical evidence must not become the current
  answer;
- old-versus-current validation gate recovery;
- fixture-backed-first tradeoff rationale with an external-adapter parity caveat;
- missing private-manifest uncertainty where the correct answer is a bounded caveat.

The report exposes `answer_type`, `requires_caveat`, `requires_refusal`, and
`can_answer_unknown` per job, and the memory-evolution table shows current evidence,
historical evidence, conflict detections, and update-rationale availability. These jobs
are fixture-backed only; they do not claim external adapter parity or private-corpus
validation.

The report also loads the checked-in external adapter coverage manifest by default:

```text
apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json
```

That manifest records the first memory-project set plus expanded RAG and graph-memory
research gates. Its `external_adapters` report section distinguishes:

- `fixture_backed`: checked-in real-world fixture scoring, such as the ELF fixture
  response path.
- `live_baseline_only`: Docker live-baseline retrieval/lifecycle evidence that is not
  a real-world suite win.
- `live_real_world`: external adapters that actually execute `real_world_job`
  prompts and scoring.
- `research_gate`: checked-in source/setup/runtime/resource/retry metadata for a
  future adapter path, not fixture-backed or live execution evidence.

Current state: the `elf_live_real_world` and `qmd_live_real_world` adapters run a full
encoded-suite sweep through `cargo make real-world-memory-live-adapters`. Each adapter
materializes generated runtime answers for 38 jobs across 11 suites before scoring.
The original targeted `work_resume`, `retrieval`, and `project_decisions` slice still
passes, but the full sweep is not a full-suite pass: memory_evolution is
`wrong_result`, production_ops remains typed `incomplete`/`blocked`/`not_encoded`, and
consolidation, knowledge_compilation, operator_debugging_ux, and capture_integration
remain `not_encoded` for this live adapter path. qmd still also keeps its separate
`live_baseline_only` same-corpus record for update/delete/cold-start checks; that
record is not a real-world suite win. agentmemory is blocked on durable upstream
storage for lifecycle proof. mem0/OpenMemory, memsearch, and claude-mem currently
retain wrong-result or incomplete live-baseline states for the checked-in adapter
evidence. OpenViking now reaches its pinned Docker local embedding setup but remains a
same-corpus `wrong_result` until it returns evidence-bearing retrieval output. The
expanded RAG and graph-memory records for RAGFlow, LightRAG, GraphRAG,
Graphiti/Zep, Letta, LangGraph, nanograph, llm-wiki, gbrain, graphify, and deeper
qmd/OpenViking profiles are `research_gate` records until their Docker-isolated
adapter runs are implemented. These typed states describe benchmark coverage; do not
convert setup weight, missing research, or unencoded suites into broad project quality
rankings.

To run the full live adapter sweep for ELF and qmd:

```sh
cargo make real-world-memory-live-adapters
```

Artifacts:

```text
tmp/real-world-memory/live-adapters/elf-materialization.json
tmp/real-world-memory/live-adapters/elf-report.json
tmp/real-world-memory/live-adapters/elf-report.md
tmp/real-world-memory/live-adapters/qmd-materialization.json
tmp/real-world-memory/live-adapters/qmd-report.json
tmp/real-world-memory/live-adapters/qmd-report.md
tmp/real-world-memory/live-adapters/summary.json
```

To run the fixture report without the manifest during local debugging:

```sh
cargo run -p elf-eval --bin real_world_job_benchmark -- \
  run \
  --fixtures apps/elf-eval/fixtures/real_world_memory \
  --skip-external-adapter-manifest
```

To test an adapter-pack manifest before committing it:

```sh
cargo run -p elf-eval --bin real_world_job_benchmark -- \
  run \
  --fixtures apps/elf-eval/fixtures/real_world_memory \
  --external-adapter-manifest path/to/manifest.json \
  --out tmp/real-world-memory/adapter-contract-report.json
```

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
The relation temporal-validity fixture is encoded and scores current owner,
historical owner, update rationale, and stale-owner trap behavior.

Current checked-in retrieval-quality increment:

```sh
cargo make real-world-memory-retrieval
```

This parses `apps/elf-eval/fixtures/real_world_memory/retrieval/`, writes
`tmp/real-world-memory/retrieval-report.json`, and renders
`tmp/real-world-memory/retrieval-report.md`. The fixture set covers alternate
phrasing, distractor-heavy retrieval, multi-hop routing, current-versus-obsolete
selection, minimal sufficient context, and trace-backed stage attribution for
operator debugging. Reports include expected evidence recall, irrelevant context ratio,
latency/cost, and optional trace explainability metadata. The qmd and OpenViking
references in these fixtures are design references only; no parity claim is allowed
unless an external adapter run actually provides evidence.

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

These fixtures use the same reviewable proposal shape as the runtime manual/fixture
consolidation service. They remain offline fixture responses and do not claim scheduled
provider-backed proposal generation.

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

Current checked-in production-ops increment:

```sh
cargo make real-world-memory-production-ops
```

Artifacts:

```text
tmp/real-world-memory/production-ops-report.json
tmp/real-world-memory/production-ops-report.md
```

The production-ops fixtures live under
`apps/elf-eval/fixtures/real_world_memory/production_ops/`. They encode user-job
readback over existing public benchmark and restore evidence: interrupted backfill
resume from checkpoint, clean-run comparison, backup/restore readback, Qdrant rebuild
from Postgres-held vectors, cold-start search recovery, and resource-envelope
interpretation.

The same slice deliberately keeps non-pass boundaries typed. A missing private
production manifest is `blocked`, unavailable provider credentials are `blocked`, and
the OpenViking cold-start dependency fixture now records a pinned Docker-local
embedding path that reaches `OpenViking.add_resource` and `OpenViking.find` but returns
`wrong_result` evidence for the smoke queries. If the pinned wheel cannot install or
import on a Docker platform, that setup boundary remains `incomplete`. These states
are evidence for operator caveats, not proof of private-corpus, provider-backed
production, or external-adapter quality success.

This suite does not run private corpus data, does not require or publish credentials,
does not perform live Docker restore/backfill work, and does not reinterpret older
live-baseline reports as real-world production-ops wins. For personal production
adoption, cite both the relevant live-baseline or restore proof and this real-world
fixture report; rerun `baseline-production-private` with an operator-owned manifest
before claiming private-corpus retrieval quality.

Do not treat the full live adapter sweep as a private-corpus or production-ops
adoption verdict. It is a full-suite sweep with typed non-pass states, not a
full-suite pass.
