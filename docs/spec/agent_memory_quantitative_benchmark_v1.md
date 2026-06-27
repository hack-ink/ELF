---
type: Spec
title: "Agent Memory Quantitative Benchmark v1"
description: "Define the public quantitative competitor scoreboard row contract and claim boundaries."
resource: docs/spec/agent_memory_quantitative_benchmark_v1.md
status: active
authority: normative
owner: spec
last_verified: 2026-06-27
tags:
  - docs
  - spec
  - benchmarking
  - agent-memory
source_refs:
  - XY-1098
  - XY-1120
code_refs:
  - apps/elf-eval/src/bin/real_world_job_benchmark.rs
  - apps/elf-eval/tests/real_world_job_benchmark.rs
related:
  - docs/spec/real_world_agent_memory_benchmark_v1.md
  - docs/evidence/benchmarking/2026-06-27-public-quantitative-competitor-scoreboard-report.md
drift_watch:
  - docs/spec/agent_memory_quantitative_benchmark_v1.md
  - docs/spec/real_world_agent_memory_benchmark_v1.md
  - apps/elf-eval/src/bin/real_world_job_benchmark.rs
  - apps/elf-eval/fixtures/report_snapshots/2026-06-27-public-quantitative-competitor-scoreboard-report.json
---
# Agent Memory Quantitative Benchmark v1

Purpose: Define the public quantitative competitor scoreboard row contract and claim
boundaries.
Status: normative
Read this when: You are implementing, validating, or publishing the public
competitor-quality scoreboard for agent memory systems.
Not this document: Real-world job fixture schema, Work Journal behavior, operational
runbooks, or external adapter setup procedures.
Defines: `elf.quality_scoreboard/v1` quantitative rows, metrics, comparability gates,
typed non-pass behavior, and optimization-direction metadata.

## Scope

The quantitative scoreboard turns `real_world_job` reports and external adapter
manifest records into public product rows. It is a row-level evidence contract, not a
universal leaderboard. It is allowed to say which metrics are proven for a row, which
competitor strengths remain visible, and which evidence is missing before a row can be
treated as comparable.

This contract applies to reports with schema `elf.quality_scoreboard/v1`.

## Scoreboard Report

A report MUST include:

- `schema`: exactly `elf.quality_scoreboard/v1`.
- `result_states`: the public row-state enum.
- `evidence_classes`: the public evidence-class enum.
- `metric_basis`: the ranking basis used for retrieval metrics.
- `retrieval_k`: the `k` used for recall, precision, MRR, and nDCG.
- typed non-pass counts and visible typed non-pass states for encoded jobs, external
  adapter rows, and the aggregate report.
- evidence-class counts.
- bounded encoded-job and aggregate summary claims.
- `unqualified_win_claim_allowed`, which MUST be `false` when any typed non-pass row
  or non-comparable row exists.
- `claim_boundary`, a human-readable statement that prevents typed blockers or
  fixture-only evidence from becoming broad superiority claims.
- `rows`: one row for ELF plus one row for each tracked external product represented
  by the loaded adapter manifest.
- `optimization_roadmap`: concrete next optimization directions derived from missing
  row evidence, not from hidden assumptions.

## Public Row States

| State | Meaning |
| --- | --- |
| `pass` | The row has a scored pass under its evidence class. A pass is comparable only when every comparability gate is also true. |
| `wrong_result` | The adapter or job reached the behavioral check but selected the wrong answer, evidence, lifecycle state, or action. |
| `incomplete` | Setup, build, parse, adapter wiring, or runtime execution did not reach the behavioral check. |
| `blocked` | The row cannot be completed safely without missing credentials, private input, durable runtime integration, Docker evidence, or manual product setup. |
| `not_tested` | No benchmark execution or comparable adapter output exists for the row. |
| `not_encoded` | The suite, scoring dimension, or adapter path is not implemented in the runner. |
| `not_comparable` | The row has useful evidence but lacks one or more required comparability gates, so it must not be used as a product-runtime comparison pass. |
| `unsupported_claim` | The row or source report made a substantive claim not supported by corpus evidence, source refs, or report metadata. |

`not_comparable` is a public row state only. It is not a `real_world_job` status and
must not be written back into job or suite outcome fields.

## Evidence Classes

| Evidence class | Meaning |
| --- | --- |
| `fixture_backed` | Checked-in fixtures were scored. This is regression evidence, not live product-runtime evidence. |
| `live_baseline` | Docker live-baseline retrieval or lifecycle evidence exists, but the row is not a real-world product-runtime scoreboard pass. |
| `live_real_world` | A live adapter executed real-world job paths and emitted typed outcomes. |
| `research_gate` | Research, source mapping, setup, credential, or resource gates are recorded before fair scoring can run. |

## Row Fields

Each `rows[]` entry MUST include:

- `product_id` and `product_name`.
- `row_source`: stable source label, such as `elf_report` or
  `external_adapter_manifest`.
- `evidence_class`.
- `result_state`.
- `comparable`: true only when all comparability gates are satisfied and the row has a
  pass state with quantitative metrics.
- comparability gates:
  - `same_corpus`
  - `source_id_mapped`
  - `held_out`
  - `leakage_audited`
  - `product_runtime`
  - `container_digest_identified`
- `metrics`.
- `strengths`: product strengths supported by the row source.
- `weaknesses`: typed weaknesses, blockers, or non-pass evidence from the row source.
- `next_evidence`: row-level evidence needed before the row can become comparable.
- `source_provenance`: bounded source pointers to the input report, adapter record, or
  suite records.

`same_corpus = true` requires positive row evidence that the product or checked-in
adapter is mapped to the benchmark corpus. A blocker sentence that says same-corpus
evidence is missing is not sufficient. A typed same-corpus setup-blocker adapter may
set this gate to true only when its source provenance identifies the intended shared
benchmark corpus and the remaining blocker is runtime/source-id output, not corpus
selection.

## Metrics

The `metrics` object MUST include `retrieval`, `lifecycle`, `answer_safety`,
`operations`, and `coverage` sub-objects.

`retrieval` MUST include:

- `k`.
- `metric_basis`.
- `recall_at_k`, `precision_at_k`, `mrr`, and `ndcg`, or `null` when the row lacks
  ranked produced evidence.
- `expected_evidence_recall`.
- `citation_source_ref_coverage`.
- matched, total, and produced evidence counts.

For `metric_basis = "produced_evidence_order"`, ranked retrieval metrics use the
ordered `produced_evidence` list in the scored job output as the retrieved list.
Expected evidence ids are the relevance set. Relevance is binary. `recall_at_k` and
`precision_at_k` use the first `k` produced evidence ids. MRR is reciprocal rank of
the first relevant produced evidence id. nDCG uses binary gains with the ideal DCG
bounded by `min(k, expected_evidence_total)`.

`lifecycle` MUST include:

- stale suppression rate and counts.
- update correctness rate and counts.
- delete correctness rate and counts.
- rollback/history readback rate and counts.

`answer_safety` MUST include:

- unsupported-claim rate and count.
- stale-answer rate and count.
- hallucinated-evidence rate when measurable.
- redaction leak count.
- irrelevant-context ratio.

`operations` MUST include:

- mean latency in milliseconds when measured.
- total cost when cost accounting exists.
- resource-envelope status, encoded job count, and pass count.

`coverage` MUST include:

- job count.
- encoded suite count.
- pass count.
- typed non-pass count.
- source-ref coverage.
- evidence coverage.
- evidence class.

## Comparability Rules

A row is comparable only when all of the following are true:

- `same_corpus = true`.
- `source_id_mapped = true`.
- `held_out = true`.
- `leakage_audited = true`.
- `product_runtime = true`.
- `container_digest_identified = true`.
- `result_state = "pass"`.
- `recall_at_k`, `precision_at_k`, `mrr`, and `ndcg` are present.

If any required gate is false, the report MUST set `comparable = false`, add a
specific `next_evidence` entry for each missing gate, and avoid any win, parity, or
rank claim for that row. If an otherwise passing row is missing a required gate, the
public row state SHOULD be `not_comparable` so the report is explicit about the
reason no product-runtime comparison claim is allowed.

## Report Claim Rules

- A row with `fixture_backed`, `live_baseline`, or `research_gate` evidence MUST NOT
  be described as a comparable product-runtime pass.
- A row with `blocked`, `incomplete`, `not_tested`, `not_encoded`, `not_comparable`,
  or `unsupported_claim` MUST remain visible as a non-pass row.
- External competitors MUST have either comparable product-runtime evidence or an
  explicit typed non-pass/blocker row with source provenance.
- Missing Docker image digest evidence is a blocker for comparability, even if a live
  adapter executed.
- Public-proxy, fixture-only, local-mock, diagnostic, blocked, and not-encoded rows
  MUST NOT be promoted into universal product superiority claims.
- Optimization direction MUST be tied to row-level `next_evidence`, metrics, or typed
  non-pass states.
