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

Do not generate large fixtures or update production-adoption verdicts while adding the
contract. The current adoption gate remains an existing benchmark decision until new
real-world job reports are implemented and published.
