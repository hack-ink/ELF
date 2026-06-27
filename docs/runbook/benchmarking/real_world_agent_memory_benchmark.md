---
type: Runbook
title: "Real-World Agent Memory Benchmark"
description: "Runbook for real-world agent memory benchmark execution and interpretation."
resource: docs/runbook/benchmarking/real_world_agent_memory_benchmark.md
status: active
authority: procedural
owner: runbook
last_verified: 2026-06-27
tags:
  - docs
  - runbook
  - benchmarking
---
# Real-World Agent Memory Benchmark

Goal: Explain the v1 real-world agent memory benchmark suite and route implementation
work to the governing spec.
Read this when: You need to create jobs, extend benchmark suites, interpret reports,
or understand why retrieval-only comparisons are insufficient.
Inputs: `docs/spec/real_world_agent_memory_benchmark_v1.md`, current live baseline
reports, external project comparison docs, and the intended user-job scenario.
Depends on: `docs/spec/real_world_agent_memory_benchmark_v1.md`,
`live_baseline_benchmark.md`, and `docs/evidence/external_memory/comparison_external_projects.md`.
Outputs: Operator-facing suite overview, bias explanation, and implementation routing.

## Governing Spec

The authoritative contract is:

- `docs/spec/real_world_agent_memory_benchmark_v1.md`

Use the spec for field names, suite ids, report states, scoring rules, and claim
boundaries. This runbook is only an operator map.

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
| Work continuity | Source-adjacent Work Journal reset/resume, rationale, rejected-option, next-step, handoff, redaction, and janitor boundaries. | Resume a session from Work Journal readback without promoting journal-only content to current memory authority. |
| Project decisions | Current decisions, rationale, reversals, and caveats. | Explain why a benchmark gate uses typed failures. |
| Retrieval | Task-relevant search with decoys and alternates. | Answer a task query while avoiding near-duplicate project evidence. |
| Memory evolution | Update, delete, expiry, contradiction, and history behavior. | Report what superseded an old fact and suppress deleted memory. |
| Consolidation | Reviewable derived memories without hidden mutation. | Produce a proposal with lineage and unsupported-claim flags. |
| Knowledge compilation | Evidence-linked project/entity/concept pages. | Compile current project status with timeline and stale-section lint. |
| Operator debugging UX | Ability to diagnose wrong results without raw store access. | Show which retrieval stage dropped expected evidence. |
| Capture/integration | Accuracy of hooks, imports, exclusions, and write policies. | Capture a session decision while excluding private spans. |
| Production ops | Backfill, restore, cold start, resource, and bounded-failure behavior. | Resume interrupted import without duplicate source notes. |
| Personalization | Scoped preferences without cross-tenant leakage. | Apply the user's current preference and ignore another project's note. |
| Core/archival memory | Always-loaded core memory behavior kept separate from archival note search. | Detect a stale core block and fall back to archival evidence. |
| Context trajectory | Staged context trajectory, hierarchy selection, rejected sibling/decoy handling, and recursive expansion. | Block OpenViking trajectory scoring until same-corpus evidence ids and comparable stage artifacts exist. |
| Adversarial quality | Quality-claim grammar under stale facts, unsupported claims, conflicting authority, private spans, and corrections. | Refuse a broad quality claim and preserve typed non-pass states instead of reporting a win. |

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
- `incomplete`
- `blocked`
- `not_tested`
- `not_encoded`
- `unsupported_claim`

The public quality scoreboard also reports evidence classes:

- `fixture_backed`
- `live_baseline`
- `live_real_world`
- `research_gate`

Internal diagnostics may keep narrower terms such as `lifecycle_fail`, but the public
scoreboard must expose typed public non-pass states instead of hiding them behind a
single win/loss column. Do not collapse `wrong_result`, `incomplete`, `blocked`,
`not_tested`, `not_encoded`, `unsupported_claim`, `fixture_backed`, `live_baseline`,
or `research_gate` rows into one leaderboard. `unsupported_claim` is especially
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
cargo make smoke-real-world-job
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
- `work_continuity`: Work Journal reset/resume readback, decision-rationale recall,
  rejected-option suppression, explicit next-step precision, inferred next-step
  labeling, handoff source-ref coverage, redaction, and janitor false-promotion
  guards while keeping journal-only content source-adjacent.
- `project_decisions`: accepted durable decisions, superseded/reversed decisions,
  old-versus-current validation gates, tradeoff rationale, and bounded caveat or
  uncertainty handling.
- `retrieval`: alternate phrasing, distractor-heavy retrieval, multi-hop routing,
  current-versus-obsolete selection, and minimal sufficient context.
- `memory_evolution`: TTL/delete suppression plus current-versus-historical preference,
  issue status, deployment method, benchmark conclusion, and temporal relation cases.
- `operator_debugging_ux`: trace-backed stage attribution that identifies where
  expected evidence was filtered, demoted, or selected against.
- `capture_integration`: write-policy audit behavior for redaction/private exclusion,
  source-id preservation, evidence binding, no secret leakage, and fixture-backed
  capture/integration boundary classification.
- `production_ops`: interrupted generated backfill resume, backup/restore plus
  cold-start readback, recoverable authority-plane drill evidence over source,
  journal, memory, knowledge, proposal, trace, and audit records,
  resource-envelope interpretation, public-proxy production-private addendum readback,
  pinned OpenViking local embedding runtime/wrong-result classification, missing
  private manifest `blocked` classification, and provider credential boundary
  `blocked` classification.
- `personalization`: scoped stable preference correction without temporary or
  cross-project preference leakage.
- `core_archival_memory`: core block attachment, scope, provenance, stale-core
  detection, archival fallback, and project-decision recovery through core routing
  plus archival rationale.
- `context_trajectory`: OpenViking staged retrieval, hierarchy selection, and
  recursive/context expansion jobs encoded as `blocked` until same-corpus expected
  evidence ids and comparable stage artifacts are available. The fixtures expose
  stage-level trace readback for the same-corpus gate, missing staged artifact,
  selected hierarchy/rejected sibling gate, and recursive expansion/pruned-branch
  gate so a blocker is reviewable instead of a prose-only limitation.
- `adversarial_quality`: stale-fact suppression, unsupported-claim refusal,
  conflicting source authority selection, private/excluded span suppression, and
  correction persistence. These fixtures gate the quality scoreboard grammar so
  unsupported, stale, blocked, incomplete, wrong-result, and not-encoded behavior
  cannot be counted as a win.
- `p1_closeout` fixture slice: four jobs across the existing `consolidation`,
  `memory_evolution`, and `work_resume` suites for Source Library -> Memory Candidate
  -> approved memory -> recall/debug -> correction/rollback, stale decision
  suppression, unsupported-claim refusal, and work-resume next action.

The generated report includes the public quality scoreboard
`elf.quality_scoreboard/v1`, encoded-job and external-adapter typed non-pass
counts/states, aggregate typed non-pass counts/states, evidence-class counts, bounded
job and aggregate summary claims, the unqualified-win guard, operational evidence
gates with `local_fixture`, `public_proxy`, `private_corpus`, and `provider_backed`
tiers, evidence coverage, source-ref coverage, quote coverage, unsupported-claim
count, stale retrieval count, stale-answer count, conflict detection count, update
rationale availability, temporal validity encoding count, scope correctness,
redaction leak count, capture/integration
behavior classes, Qdrant rebuild case/pass counts, expected evidence recall,
irrelevant context ratio, latency/cost, answer-type plus
caveat/refusal/uncertainty flags, trace explainability counters, production-ops
blocked/wrong-result job states, Work Continuity rates for reset/resume,
decision-rationale recall, rejected-option suppression, explicit next-step precision,
inferred next-step labeling, handoff source-ref coverage, redaction, and janitor
false-promotion, Work Continuity hard-fail counters, and
private-corpus redaction policy. The fixtures include negative traps for stale
blockers, unsupported prior claims, stale deleted facts, stale historical facts,
cross-project preference leakage, private/redacted text leakage, obsolete retrieval
context, project-decision stale reuse, missing rationale, uncited current policy
claims, overconfident unsupported decision answers, distractor context,
index-only restore claims, private-corpus pass claims without a manifest, checked-in
credential leakage, and adversarial stale or unsupported scoreboard claims.

Current checked-in Work Continuity increment:

```sh
cargo make real-world-memory-work-continuity
```

This parses `apps/elf-eval/fixtures/real_world_memory/work_continuity/`, writes
`tmp/real-world-memory/work-continuity/report.json`, and renders
`tmp/real-world-memory/work-continuity/report.md`. The slice scores eight
fixture-backed jobs for reset/resume readback, decision rationale, rejected-option
suppression, explicit next-step precision, inferred next-step labeling, handoff
source-ref coverage, redaction, and janitor false-promotion prevention. It does not
claim diary-product behavior, current-fact authority from journal-only content,
provider-backed private-corpus quality, or competitor-wide superiority.

Current checked-in adversarial quality increment:

```sh
cargo make real-world-memory-adversarial-quality
```

This parses
`apps/elf-eval/fixtures/real_world_memory/adversarial_quality/`, writes
`tmp/real-world-memory/adversarial-quality/report.json`, and renders
`tmp/real-world-memory/adversarial-quality/report.md`.

The slice scores five fixture-backed jobs for stale fact suppression,
unsupported-claim refusal, conflicting source authority, private/excluded spans, and
correction persistence. The report is deliberately narrow: it proves that the
scoreboard grammar and adversarial traps catch stale or unsupported behavior, not that
ELF has a live-adapter, private-corpus, provider-backed, or broad competitor win.

Current checked-in P1 closeout increment:

```sh
cargo make real-world-memory-p1-closeout
```

This parses `apps/elf-eval/fixtures/real_world_memory/p1_closeout/`, writes
`tmp/real-world-memory/p1-closeout/report.json`, and renders
`tmp/real-world-memory/p1-closeout/report.md`. The checked-in evidence report is
`docs/evidence/benchmarking/2026-06-22-p1-memory-authority-closeout-report.md`, and
the checked-in JSON snapshot is
`apps/elf-eval/fixtures/report_snapshots/2026-06-22-p1-memory-authority-closeout-report.json`.

The slice scores four jobs as pass with zero wrong results, zero unsupported claims,
zero stale answers, two conflict detections, two update rationales, two history
readbacks, full evidence/source-ref/quote coverage, one recall/debug trace, and zero
source mutations. It is fixture-backed closeout evidence only; it does not claim a
live adapter sweep, private-corpus quality, provider-backed quality, or broad
competitor wins.

Current checked-in P2 Knowledge Workspace closeout increment:

```sh
cargo make real-world-memory-p2-knowledge-closeout
```

This runs the checked-in Source Library and Knowledge Workspace fixture slices:

```text
tmp/real-world-memory/source-library-report.json
tmp/real-world-memory/source-library-report.md
tmp/real-world-memory/knowledge-report.json
tmp/real-world-memory/knowledge-report.md
```

The checked-in evidence report is
`docs/evidence/benchmarking/2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.md`,
and the checked-in JSON snapshot is
`apps/elf-eval/fixtures/report_snapshots/2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.json`.

The increment scores the Source Library slice as 2 pass and the Knowledge Workspace
slice as 3 pass. It covers long-document source refs, hydrated excerpts,
project/entity/concept/issue pages, stale lint, changed-source watch/rebuild,
previous-version diff metadata, and reviewable memory-candidate boundaries. VectifyAI
PageIndex and OpenKB stay `not_tested` reference-only rows until contained adapters
emit comparable tree/wiki artifacts, source refs, lint/watch output, and typed
benchmark states. This closeout does not queue any P3 issue.

Current checked-in P3 mem0/OpenMemory and Letta adapter increment:

```sh
cargo make real-world-memory-mem0-openmemory-letta
```

This parses
`apps/elf-eval/fixtures/real_world_external_adapters/mem0_openmemory_letta/`, writes
`tmp/real-world-memory/mem0-openmemory-letta/report.json`, and renders
`tmp/real-world-memory/mem0-openmemory-letta/report.md`. The checked-in evidence
report is
`docs/evidence/benchmarking/2026-06-22-mem0-openmemory-letta-memory-history-core-archive-report.md`,
and the checked-in JSON snapshot is
`apps/elf-eval/fixtures/report_snapshots/2026-06-22-mem0-openmemory-letta-memory-history-core-archive-report.json`.

The increment scores four jobs: one mem0 SDK history/export job as pass and three
typed blockers for OpenMemory UI/export, Letta core block export, and Letta archival
readback/search. It maps mem0 SDK `Memory.history`, scoped search, and local
`Memory.get_all` output to source ids, preserves OpenMemory UI/export as a product
container/app-database blocker, preserves Letta core/archive as export/readback
blockers, and makes no hosted/product parity claim.

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

The public quality scoreboard renders the existing manifest evidence bucket
`live_baseline_only` as the public evidence class `live_baseline`. When the default
external adapter manifest is loaded, the scoreboard's typed non-pass count includes
adapter coverage and scenario rows as well as fixture jobs.

Current fixture state: `cargo make real-world-memory-json` covers 81 jobs across 19
suites, with 74 pass and 7 blocked. The adversarial quality slice contributes five
passing fixture-backed jobs that exercise stale fact suppression, unsupported-claim
refusal, source-authority conflicts, private-span exclusion, and correction
persistence. The P1 closeout fixture slice contributes four passing jobs for
memory-authority closeout evidence. The `core_archival_memory` suite
contributes six passing fixture jobs for core block attachment, scope, provenance,
stale-core detection, archival fallback, and project-decision recovery. The
`memory_summary` suite
contributes one passing fixture-backed source-trace job for reviewable current,
background, stale, superseded, tombstoned, and derived project-profile entries. The
`work_continuity` suite contributes eight passing fixture-backed Work Journal rows
for reset/resume, rationale, rejected-option, explicit/inferred next-step, handoff
source-ref, redaction, and janitor false-promotion boundaries. The
`proactive_brief` suite contributes four passing source-linked proactive suggestions
and one typed private-corpus refresh blocker tied to XY-930. The blocked jobs are
production-ops operator boundaries, the private-corpus refresh blocker, the
private/provider scheduler blocker, plus the XY-928 OpenViking `context_trajectory`
gates for staged retrieval, hierarchy selection, and recursive context expansion.
The `scheduled_memory` suite contributes four passing source-linked scheduled task
readbacks plus one typed private/provider scheduler blocker tied to XY-930; it is not
hosted scheduler, ChatGPT Tasks, Pulse, notification, or provider-backed private-corpus
parity evidence.

Current live-adapter state: the `elf_live_real_world` and `qmd_live_real_world` adapters run a full
checked-in suite sweep through `cargo make real-world-memory-live-adapters`. Each adapter
materializes generated runtime answers for 55 jobs across 13 suites before scoring,
including the operator-debug fixture tree.
The original targeted `work_resume`, `retrieval`, and `project_decisions` slice still
passes. ELF now also passes live `capture_integration` self-checks for redaction,
exclusions, source ids, evidence binding, and no secret leakage; live consolidation
proposal review; live knowledge-page rebuild/lint; and live operator-debug trace
metadata. The full sweep is still not a full-suite pass: memory_evolution is
`wrong_result`, production_ops keeps operator-owned blocked boundaries,
core_archival_memory remains typed `not_encoded` for this live adapter path, and
context_trajectory remains blocked. qmd keeps `capture_integration`, consolidation,
knowledge_compilation, and core_archival_memory typed non-pass, is `wrong_result` for
operator-debug trace hydration, and still also keeps its separate `live_baseline_only`
same-corpus record for update/delete/cold-start checks; that record is not a
real-world suite win. agentmemory is blocked on durable upstream storage for lifecycle
proof and capture breadth. mem0/OpenMemory, memsearch, and claude-mem no longer share
one live-baseline boundary: mem0/OpenMemory and memsearch now pass scoped local
baseline paths, while OpenMemory product UI/export, hosted
Platform behavior, optional graph memory, memsearch real-world prompt/TTL coverage,
and claude-mem hook/viewer capture remain blocked, unsupported, not encoded, or
wrong-result for the checked-in adapter evidence. OpenViking now reaches its pinned
Docker local embedding setup but remains a same-corpus `wrong_result` until it
returns evidence-bearing retrieval output. The checked-in `context_trajectory`
fixtures keep OpenViking staged retrieval, hierarchy selection, and recursive/context
expansion blocked until same-corpus evidence ids match and staged artifacts are
materialized.
The expanded RAG and graph-memory records for RAGFlow, LightRAG, GraphRAG,
Graphiti/Zep, Letta, LangGraph, nanograph, llm-wiki, gbrain, graphify, and deeper
qmd/OpenViking profiles stay `research_gate`, typed non-pass, or not-encoded records
until Docker-contained or provider-backed evidence-linked outputs exist. XY-929 adds a
focused representative slice for graph/RAG navigation, citation mapping, graph
summaries, temporal validity, graph reports, stale-source lint, and unsupported-claim
handling:

```sh
cargo make real-world-memory-graph-rag
```

Artifacts:

```text
tmp/real-world-memory/graph-rag/report.json
tmp/real-world-memory/graph-rag/report.md
```

This slice is allowed to report blocked, incomplete, wrong_result, not_tested, and
non_goal outcomes. These typed states describe benchmark coverage; do not convert setup
weight, missing research, smoke output, or representative non-pass fixtures into broad
project quality rankings.

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

Graphiti/Zep temporal adapter refresh:

```sh
cargo make smoke-graphiti-zep-docker-temporal
```

Default artifacts:

```text
tmp/real-world-memory/graphiti-zep-smoke/graphiti-zep-report.json
tmp/real-world-memory/graphiti-zep-smoke/graphiti-zep-report.md
tmp/real-world-memory/graphiti-zep-smoke/summary.json
```

The default command emits a typed blocker. A live attempt is opt-in:

```sh
ELF_GRAPHITI_ZEP_SMOKE_START=1 ELF_GRAPHITI_ZEP_SMOKE_RUN=1 cargo make smoke-graphiti-zep-docker-temporal
```

The live path can pass only if generated current and historical temporal relation
facts map to validity windows and source evidence ids. Missing provider credentials,
FalkorDB startup failure, Graphiti setup failure, or unmapped validity windows stay
typed as `blocked`, `incomplete`, or `wrong_result`; no hosted Zep or ELF graph-memory
parity claim is allowed from this smoke.

OpenViking context-trajectory refresh:

```sh
cargo make real-world-memory-context-trajectory
```

The command scores the checked-in staged retrieval, hierarchy selection, and
recursive/context expansion fixtures. Current blocked fixtures include trace-stage
artifacts that name the same-corpus gate, missing stage/hierarchy/recursive artifact,
dropped decoy or rejected sibling evidence, and comparison gate. These trace stages
make the missing artifacts auditable; they do not create an ELF win, tie, or loss.

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
`docs/evidence/benchmarking/2026-06-09-operator-debugging-ux-report.md`.

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

Current live consolidation increment:

```sh
cargo make real-world-memory-live-consolidation
```

This runs only `apps/elf-eval/fixtures/real_world_memory/consolidation/` through the
ELF live service adapter and writes:

```text
tmp/real-world-memory/live-consolidation/elf-materialization.json
tmp/real-world-memory/live-consolidation/elf-report.json
tmp/real-world-memory/live-consolidation/elf-report.md
tmp/real-world-memory/live-consolidation/summary.json
```

The live increment proves service-backed proposal materialization and review audit for
the current checked-in consolidation jobs. It does not implement scheduled production
consolidation, live provider-generated proposal quality, source-of-truth rewrites, or
knowledge-page rebuild/lint scoring.

Current checked-in knowledge-compilation increment:

```sh
cargo make real-world-memory-knowledge
```

This parses `apps/elf-eval/fixtures/real_world_memory/knowledge/`, writes
`tmp/real-world-memory/knowledge-report.json`, and renders
`tmp/real-world-memory/knowledge-report.md`. The fixtures include synthetic project,
entity, concept, issue-timeline, and changed-source watch/rebuild page artifacts.
Generated pages are benchmark artifacts only: every section must cite source evidence
or timeline events, or it must be explicitly flagged unsupported. The report publishes
citation coverage, stale claim detection, rebuild determinism, aggregate backlink
counts and page coverage, previous-version diff coverage, page usefulness,
unsupported summary count, and untraced section count.

Current live knowledge-page rebuild/lint increment:

```sh
cargo make real-world-memory-live-knowledge
```

Artifacts:

```text
tmp/real-world-memory/live-knowledge/elf-materialization.json
tmp/real-world-memory/live-knowledge/elf-report.json
tmp/real-world-memory/live-knowledge/elf-report.md
tmp/real-world-memory/live-knowledge/summary.json
```

The live increment runs inside the Docker baseline runner and materializes the
knowledge fixtures through `ElfService::knowledge_page_rebuild`,
`knowledge_page_lint`, and `knowledge_pages_search` before scoring them with the
real-world job benchmark. It proves ELF service-native rebuild/lint/search behavior
for the checked-in `knowledge_compilation` pack. The current productized workspace
increment also requires `page_version_diff` artifacts under
`elf.knowledge_page.version_diff/v1` and reports `version_diff_coverage` in the
knowledge summary. It does not claim llm-wiki, gbrain, GraphRAG, RAGFlow, LightRAG,
or graphify parity unless those projects emit comparable page sections, source ids,
citation mappings, lint findings, previous-version diffs, and typed statuses.

Current checked-in production-ops increment:

```sh
cargo make real-world-memory-production-ops
```

Current P4 production-readiness evidence-gate slice:

```sh
cargo make real-world-memory-p4-production-readiness
```

Artifacts:

```text
tmp/real-world-memory/production-ops-report.json
tmp/real-world-memory/production-ops-report.md
tmp/real-world-memory/p4-production-readiness/report.json
tmp/real-world-memory/p4-production-readiness/report.md
```

The production-ops fixtures live under
`apps/elf-eval/fixtures/real_world_memory/production_ops/`. They encode user-job
readback over existing public benchmark and restore evidence: interrupted backfill
resume from checkpoint, clean-run comparison, backup/restore readback, Qdrant rebuild
from Postgres-held vectors, cold-start search recovery, recoverable authority-plane
drills, and resource-envelope interpretation. Authority recovery drills use
`elf.authority_recovery_drill/v1` under `adapter_response.answer.recovery_drills[]`
to report topology, failure injection, backup/PITR, degraded-read labels, RPO/RTO
targets and measurements, before/after authority record counts, idempotent outbox
replay, Qdrant rebuild completeness, migration repair, and dead-letter handling. The
P4 slice also encodes the operator-approved public-proxy production-private addendum
and emits `elf.operational_evidence_gates/v1` so local fixture, public-proxy,
private-corpus, and provider-backed evidence remain separate.

The same slice deliberately keeps non-pass boundaries typed. A missing private
production manifest is `blocked`, unavailable provider credentials are `blocked`, and
the OpenViking cold-start dependency fixture now records a pinned Docker-local
embedding path that reaches `OpenViking.add_resource` and `OpenViking.find` but returns
`wrong_result` evidence for the smoke queries. If the pinned wheel cannot install or
import on a Docker platform, that setup boundary remains `incomplete`. These states
are evidence for operator caveats, not proof of private-corpus, provider-backed
production, or external-adapter quality success.

Public-proxy passes are useful production-readiness signals, but they do not satisfy a
real private-corpus gate. Local-hash or fixture-backed cost/latency records are
operational accounting evidence, not hosted provider-spend or provider-quality proof.

This suite does not run private corpus data, does not require or publish credentials,
does not perform live Docker restore/backfill work, and does not reinterpret older
live-baseline reports as real-world production-ops wins. For personal production
adoption, cite both the relevant live-baseline or restore proof and this real-world
fixture report; rerun `baseline-production-private` with an operator-owned manifest
before claiming private-corpus retrieval quality.

Do not treat the full live adapter sweep as a private-corpus or production-ops
adoption verdict. It is a full-suite sweep with typed non-pass states, not a
full-suite pass.
