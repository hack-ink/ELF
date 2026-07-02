---
type: Spec
title: "Agent Memory Quantitative Benchmark v1"
description: "Define quantitative same-corpus memory benchmark metrics, formulas, evidence classes, and claim boundaries."
resource: docs/spec/agent_memory_quantitative_benchmark_v1.md
status: active
authority: normative
owner: spec
last_verified: 2026-06-23
tags:
  - docs
  - spec
  - benchmarking
  - agent-memory
source_refs: []
code_refs:
  - Makefile.toml
  - makefiles/benchmark-memory-a.toml
  - makefiles/benchmark-memory-b.toml
  - scripts/materialize-explicit-qrels.py
  - scripts/materialize-quantitative-artifact-freshness.py
  - scripts/real-world-explicit-qrels.sh
  - scripts/real-world-docker.sh
  - scripts/real-world-quantitative-docker.sh
  - scripts/real-world-live-explicit-qrels.sh
  - apps/elf-eval/src/app.rs
  - apps/elf-eval/src/bin/real_world_job_benchmark/main.rs
  - apps/elf-eval/fixtures/real_world_memory/p1_closeout/source_candidate_approval_recall.json
  - apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json
related:
  - docs/spec/agent_memory_knowledge_system_v1.md
  - docs/spec/real_world_agent_memory_benchmark_v1.md
  - docs/evidence/benchmarking/2026-06-23-p4-quality-hardening-productization-readiness-report.md
  - docs/evidence/benchmarking/2026-06-23-p3-competitor-strength-absorption-report.md
drift_watch:
  - docs/spec/agent_memory_quantitative_benchmark_v1.md
  - Makefile.toml
  - makefiles/benchmark-memory-a.toml
  - makefiles/benchmark-memory-b.toml
  - scripts/materialize-explicit-qrels.py
  - scripts/materialize-quantitative-artifact-freshness.py
  - scripts/real-world-explicit-qrels.sh
  - scripts/real-world-docker.sh
  - scripts/real-world-quantitative-docker.sh
  - scripts/real-world-live-explicit-qrels.sh
  - docs/spec/agent_memory_knowledge_system_v1.md
  - docs/spec/real_world_agent_memory_benchmark_v1.md
  - apps/elf-eval/src/bin/real_world_job_benchmark/main.rs
  - apps/elf-eval/src/app.rs
  - docs/evidence/benchmarking/index.md
---
# Agent Memory Quantitative Benchmark v1

Purpose: Define the quantitative scoreboard that must sit beside ELF's existing
typed real-world memory benchmark reports.
Status: normative
Read this when: You are adding or reviewing recall, freshness, update, delete,
expiry, latency, cost, or competitor-comparison metrics for agent memory systems.
Not this document: A finished benchmark report, a claim that current results beat
every competitor, or a replacement for typed non-pass outcome reporting.
Defines: `elf.agent_memory_quantitative_benchmark/v1`, required metric families,
formulas, denominators, evidence classes, comparability rules, and minimum report
rows.

## Core Rule

Quantitative memory comparison must measure the exact behavior users care about:
finding the right evidence, using current facts, suppressing stale or deleted facts,
showing citations, and staying within latency/cost/resource bounds.

A report must not use broad product labels such as "best memory" or "beats OpenKB"
unless the specific metric row is same-corpus, same-task, same-evidence-class,
same-candidate-source, same-denominator, and leaderboard eligible. Typed non-pass
states remain first-class results.

## Evidence Classes

Every quantitative row must declare one evidence class:

| Evidence class | Meaning | Comparable for leaderboard |
| --- | --- | --- |
| `fixture_backed` | Checked-in fixture scored by ELF's runner. | Only against other fixture rows with the same corpus and task. |
| `live_baseline` | Docker-contained baseline or smoke run that may not execute real-world answer jobs. | No, unless the report states the exact same scored task. |
| `live_real_world` | Runtime executed the same real-world job prompt and produced scored answer artifacts. | Yes, when same-corpus and same-task. |
| `public_proxy` | Local proxy contract based on public docs or expected artifact shape, not a product runtime. | No product leaderboard claim. |
| `private_corpus` | Operator-owned private corpus with publishable bounded metrics only. | Yes only for private-corpus rows with matching policy. |
| `provider_backed` | Provider credentials/models were used and cost/latency are measured. | Yes only against rows with equivalent provider boundary. |
| `research_gate` | Research-only, blocked, or reference-only evidence. | No. |
| `mixed_evidence` | Aggregate row blends multiple evidence classes. | No; split rows before leaderboard use. |

## Artifact Freshness And Public Reproducibility

Public reproducibility is a separate gate from metric quality. A quantitative row
may support a public reproducibility claim only when its provenance carries the
aggregate command, Docker runner, Compose file, repository head, environment
profile, input product-manifest SHA-256, artifact digest, container image digest,
and structured 40-hex product source commit.

`cargo make real-world-memory-quantitative-docker` runs the Docker-owned aggregate
entrypoint and fails before aggregate work starts if the `baseline-runner` image
digest is absent or malformed. `scripts/materialize-quantitative-artifact-freshness.py`
then materializes the row-level freshness manifest for Docker-contained
quantitative product manifests. Runtime-sensitive rows such as Honcho, Letta, and
RAGFlow must not accept a product commit unless a matching
`runtime_source_attestation` proves the pinned checkout or image revision was the
runtime that emitted the row. A non-pass attestation remains a failure even when a
commit-shaped value is present.

## Result States

Every row must declare one result state:

| State | Meaning |
| --- | --- |
| `pass` | The metric is measured and meets the row threshold. |
| `wrong_result` | The task ran but selected the wrong answer, wrong evidence, or wrong lifecycle state. |
| `incomplete` | Some required artifacts exist, but the metric denominator is not fully satisfied. |
| `blocked` | Required setup, credentials, corpus, exported artifact, or product readback is missing. |
| `not_encoded` | The adapter or benchmark does not implement this metric. |
| `not_comparable` | A metric exists but evidence class, corpus, task, or denominator differs. |
| `unsupported_claim` | The output makes a claim that the evidence cannot support. |

Metric states are separate from row result states. A metric state of `measured`
means the denominator is non-zero and the row has no typed non-pass state; it does
not mean the value passed a leaderboard threshold. If the row result is
`blocked`, `wrong_result`, `incomplete`, `not_encoded`, or `unsupported_claim`,
metric states for measured values must inherit that non-pass state.

Metric states may also use `partial_coverage` when a formula is computable for
some queries but the row lacks full ranked-candidate coverage or the minimum query
count required for leaderboard use. `partial_coverage` values are useful regression
evidence, not product-ranking proof.

## Retrieval Metrics

Retrieval metrics apply when a job has relevance labels and an ordered candidate
list. The report must name `k` for every `@k` metric. A row must also declare whether
ranked candidates came from a product/runtime trace or a fixture trace; fixture traces
are formula smoke tests unless the compared product emitted the same artifact shape.
Explicit qrels live in `expected_answer.relevance_judgments` as
`{ "evidence_id": "...", "grade": 0.0 }` records. If a legacy fixture omits qrels,
the runner may derive binary relevance from required evidence for regression use,
but that row must expose `qrel_source = expected_evidence_fallback` and must not
become leaderboard eligible.

`cargo make real-world-memory-explicit-qrels` is the deterministic qrel
materialization command for fixture-mechanics evidence. It derives positive qrels
from checked-in `expected_answer.evidence_links` and `required_evidence`, preserves
existing explicit zero-grade judgments, and leaves unmentioned corpus evidence
unjudged instead of converting it into synthetic negative labels. Its optional
oracle ranked candidates are allowed only to prove metric mechanics; they are not
product-runtime retrieval evidence and cannot satisfy leaderboard runtime, held-out,
or leakage-audit gates.

`cargo make real-world-memory-live-explicit-qrels` is the current product-runtime
bridge from deterministic qrel materialization to ELF/qmd live adapter scoring. It
must materialize explicit qrels with `--ranked-candidates-source none`, then let
the live adapters emit their own runtime ranked candidates. This command can close
the `qrel_source` gap for product-runtime rows, but it does not itself prove
held-out status, leakage audit status, or clean leaderboard eligibility.

| Metric | Formula | Required fields |
| --- | --- | --- |
| `recall_at_k` | `relevant_returned_in_top_k / expected_relevant_count` | relevance labels, explicit `ranked_candidate_evidence_ids`, `k` |
| `precision_at_k` | `relevant_returned_in_top_k / k` | ordered candidates, relevance labels |
| `mrr` | `1 / rank(first_relevant)` or `0` when no relevant item appears | ordered candidates, relevance labels |
| `ndcg_at_k` | `dcg_at_k / ideal_dcg_at_k` using graded relevance when available, binary otherwise | ordered candidates, relevance grades |
| `map` | Mean of per-query average precision values | ordered candidates, relevance labels |
| `average_precision` | Per-query sum of precision at each relevant hit divided by expected relevant count | ordered candidates, relevance labels |
| `success_at_k` | Query has at least one relevant candidate in the top `k` | ordered candidates, relevance labels, `k` |
| `expected_evidence_recall` | `produced_required_evidence_count / required_evidence_count` | required evidence map, produced evidence ids |
| `citation_coverage` | `claims_with_valid_citation / claims_requiring_citation` | claim list, citation validation result |
| `source_ref_coverage` | `claims_with_valid_source_ref / claims_requiring_source_ref` | source-ref validation result |

Retrieval metrics must not count redacted, excluded, deleted, expired, unreadable, or
non-captured source spans as relevant current evidence. Such candidates may be
reported separately as historical or diagnostic rows.

## Memory Lifecycle Metrics

Memory lifecycle metrics apply to jobs that encode state changes over time.

| Metric | Formula | What it proves |
| --- | --- | --- |
| `update_correctness_rate` | `jobs_selecting_current_superseding_fact / update_jobs` | New facts replace old facts for current answers. |
| `stale_suppression_rate` | `stale_facts_not_used_as_current / stale_fact_opportunities` | Stale facts do not pollute current answers. |
| `delete_suppression_rate` | `deleted_or_tombstoned_facts_not_used / delete_opportunities` | Deleted or tombstoned facts do not reappear as current context. |
| `expiry_suppression_rate` | `expired_facts_not_used / expiry_opportunities` | TTL or time-bounded facts are suppressed after expiry. |
| `rollback_readback_rate` | `rollback_events_with_readback / rollback_events_expected` | Rollback and prior versions remain auditable. |
| `history_readback_rate` | `history_events_readable / history_events_expected` | Add, update, ignore, reject, delete, restore, and derived transitions are visible. |
| `contradiction_resolution_rate` | `contradictions_resolved_to_current_supported_answer / contradiction_opportunities` | Mutually inconsistent memories are resolved with current source support instead of arbitrary retrieval order. |

The denominator must be explicit. A benchmark with no delete jobs must report
`delete_suppression_rate = not_encoded`, not `1.000`.

## Answer Safety Metrics

| Metric | Formula |
| --- | --- |
| `unsupported_claim_rate` | `unsupported_claim_count / answer_claim_count` |
| `stale_answer_rate` | `answers_using_stale_fact_as_current / answered_jobs` |
| `hallucinated_evidence_rate` | `citations_not_in_candidate_or_source_set / citation_count` |
| `redaction_leak_count` | Count of private, excluded, or redacted spans surfaced in public output. |
| `irrelevant_context_ratio` | `irrelevant_context_items / returned_context_items` |
| `scope_violation_count` | Count of unreadable cross-scope or grant-violating rows returned. |

Zero values are meaningful only when the denominator is non-zero and the checked row
actually exercises the failure mode.

## Operational Metrics

| Metric | Required unit |
| --- | --- |
| `ingestion_success_rate` | successful ingested records / records submitted |
| `indexing_coverage` | indexed records or spans / ingestible records or spans |
| `source_id_mapping_coverage` | returned candidates or generated claims mapped to benchmark source ids / candidates or claims requiring mapping |
| `query_latency_p50_ms`, `query_latency_p95_ms`, `query_latency_p99_ms` | milliseconds |
| `ingest_latency_ms` | milliseconds from submitted source to durable ingest acknowledgement |
| `update_propagation_latency_ms` | milliseconds from write/apply/delete to searchable/readable effect |
| `cold_start_recovery_seconds` | seconds |
| `restore_seconds` | seconds |
| `index_rebuild_seconds` | seconds |
| `cost_usd` | USD with input/output token counts where applicable |
| `available_context_token_count` | tokens available in the source corpus or memory store for the query |
| `answer_context_token_count` | tokens supplied to the answering model or final answer context |
| `context_token_efficiency` | `answer_context_token_count / available_context_token_count` |
| `resource_envelope_status` | pass, blocked, incomplete, not_encoded |

Provider-backed rows must include model/provider identifiers or must remain
`not_comparable`. Fixture zero-cost rows must not imply hosted provider cost.

## Quantitative Scoreboard Schema

Reports that implement this spec must emit:

```json
{
  "schema": "elf.agent_memory_quantitative_benchmark/v1",
  "generated_at": "...",
  "corpus_id": "...",
  "k_values": [1, 3, 5, 10],
  "rows": [
    {
      "product": "ELF",
      "adapter_id": "elf_live_real_world",
      "adapter_name": "ELF live real-world",
      "suite": "memory_evolution",
      "evidence_class": "live_real_world",
      "result_state": "pass",
      "comparable": true,
      "metric_comparable": true,
      "leaderboard_eligible": false,
      "held_out": false,
      "leakage_audited": false,
      "audit_manifest_id": null,
      "fixture_regression_only": false,
      "sample_size": 40,
      "ranking_query_count": 40,
      "ranking_coverage_state": "measured",
      "ranked_candidate_source": "runtime_trace",
      "qrel_source": "explicit_qrels",
      "explicit_qrel_query_count": 40,
      "metrics": {
        "recall_at_5": 1.0,
        "precision_at_5": 0.6,
        "mrr": 1.0,
        "ndcg_at_5": 1.0,
        "map": 1.0,
        "average_precision": 1.0,
        "success_at_5": 1.0,
        "explicit_qrel_query_coverage": 1.0,
        "relevance_judgment_count": 80,
        "relevance_grade_sum": 160,
        "update_correctness_rate": 1.0,
        "stale_suppression_rate": 1.0,
        "delete_suppression_rate": 1.0,
        "expected_evidence_recall": 1.0,
        "unsupported_claim_rate": 0.0,
        "stale_answer_rate": 0.0
      },
      "metric_states": {
        "recall_at_5": "measured",
        "precision_at_5": "measured",
        "mrr": "measured",
        "ndcg_at_5": "measured",
        "average_precision": "measured",
        "map": "measured",
        "success_at_5": "measured"
      },
      "denominators": {
        "recall_at_5": 80,
        "precision_at_5": 200,
        "map": 40,
        "success_at_5": 40,
        "update_correctness_rate": 2,
        "delete_suppression_rate": 1,
        "stale_answer_rate": 40
      },
      "confidence_intervals": {
        "recall_at_5": {
          "method": "wilson_score",
          "confidence": 0.95,
          "lower": 0.954,
          "upper": 1.0,
          "numerator": 80,
          "denominator": 80
        }
      },
      "claim_boundary": "Comparable only against same-corpus live_real_world rows."
    }
  ],
  "per_query_rows": [
    {
      "job_id": "memory-evolution-001",
      "suite": "memory_evolution",
      "evidence_class": "live_real_world",
      "result_state": "pass",
      "expected_relevant_count": 2,
      "candidate_count": 8,
      "qrel_source": "explicit_qrels",
      "relevance_grade_sum": 4.0,
      "product": "ELF",
      "adapter_id": "elf_live_real_world",
      "metrics": {
        "recall_at_5": 1.0,
        "precision_at_5": 0.4,
        "mrr": 1.0,
        "ndcg_at_5": 1.0,
        "average_precision": 1.0,
        "success_at_5": 1.0
      },
      "metric_states": {
        "recall_at_5": "measured",
        "precision_at_5": "measured",
        "mrr": "measured",
        "ndcg_at_5": "measured",
        "average_precision": "measured",
        "success_at_5": "measured"
      },
      "denominators": {
        "recall_at_5": 2,
        "precision_at_5": 5,
        "mrr": 1,
        "ndcg_at_5": 1,
        "average_precision": 1,
        "success_at_5": 1
      }
    }
  ],
  "ablation_rows": [
    {
      "product": "ELF",
      "adapter_id": "elf_live_real_world",
      "ablation_id": "raw_vector",
      "job_id": "memory-evolution-001",
      "suite": "memory_evolution",
      "evidence_class": "live_real_world",
      "result_state": "pass",
      "candidate_source": "runtime_trace_ablation",
      "qrel_source": "explicit_qrels",
      "expected_relevant_count": 2,
      "candidate_count": 8,
      "metrics": {
        "recall_at_5": 0.5,
        "precision_at_5": 0.2,
        "mrr": 0.5,
        "ndcg_at_5": 0.62,
        "average_precision": 0.5,
        "success_at_5": 1.0
      },
      "metric_states": {
        "recall_at_5": "measured",
        "precision_at_5": "measured",
        "mrr": "measured",
        "ndcg_at_5": "measured",
        "average_precision": "measured",
        "success_at_5": "measured"
      },
      "denominators": {
        "recall_at_5": 2,
        "precision_at_5": 5,
        "mrr": 1,
        "ndcg_at_5": 1,
        "average_precision": 1,
        "success_at_5": 1
      },
      "claim_boundary": "Ablation rows score explicitly supplied candidate orderings for diagnosis; they are not separate product-runtime rows unless the evidence class and candidate source say so."
    }
  ],
  "significance": {
    "method": "exact_two_sided_sign_test_on_same_query_metric_deltas",
    "state": "not_encoded_single_product_row",
    "eligible": false,
    "minimum_paired_query_count": 30,
    "comparable_product_row_count": 1,
    "paired_query_count": 0,
    "comparisons": [],
    "ablation_comparisons": [
      {
        "comparison_scope": "ablation",
        "baseline_id": "raw_vector",
        "candidate_id": "governed_memory",
        "baseline_product": "raw_vector",
        "candidate_product": "governed_memory",
        "metric": "ndcg_at_5",
        "paired_query_count": 1,
        "state": "measured",
        "effect_mean": 0.311,
        "p_value": 1.0,
        "win_count": 1,
        "loss_count": 0,
        "tie_count": 0
      }
    ],
    "claim_boundary": "Pairwise wins require at least two leaderboard-eligible rows with same-query per-query metrics; otherwise p-values and win claims stay not encoded."
  },
  "leakage_audit": {
    "state": "not_leaderboard_eligible",
    "held_out": false,
    "leakage_audited": false,
    "corpus_profile": "synthetic",
    "evidence_class": "fixture_backed",
    "qrel_source": "explicit_qrels",
    "fixture_regression_only": true,
    "ranking_coverage_state": "partial_coverage",
    "leaderboard_blocking_reasons": [
      "fixture_regression_only",
      "insufficient_query_count",
      "no_held_out_manifest",
      "no_leakage_audit_manifest",
      "not_live_real_world",
      "ranking_coverage_not_measured"
    ],
    "claim_boundary": "Held-out and leakage-audit fields are explicit gates; fixture or non-audited rows cannot become public leaderboard evidence by omission."
  },
  "non_comparable_rows": [
    {
      "product": "VectifyAI PageIndex",
      "adapter_id": "pageindex_public_proxy_contract",
      "result_state": "not_comparable",
      "reason": "public_proxy evidence class; no PageIndex product runtime output"
    }
  ],
  "controls": {
    "same_corpus_required": true,
    "same_task_required": true,
    "same_evidence_class_required": true,
    "same_budget_required": true,
    "ranked_candidates_required_for_ranking_metrics": true,
    "raw_ranked_candidate_artifacts_required": true,
    "held_out_or_leakage_audited_required": true,
    "explicit_relevance_judgments_required_for_leaderboard": true,
    "per_query_rows_required_for_significance": true,
    "minimum_query_count_for_leaderboard": 30,
    "current_query_count": 40,
    "current_ranking_query_count": 40,
    "current_explicit_qrel_query_count": 40,
    "comparable_product_row_count": 1,
    "leaderboard_claim_allowed": false,
    "statistical_significance": "not_encoded_until_at_least_two_same-corpus comparable product rows meet minimum query count, full ranking coverage, and explicit qrels",
    "uncertainty_reporting": "single-row rates include Wilson 95% confidence intervals; competitor win claims require same-query paired significance over per-query rows.",
    "leakage_control": "fixture rows are not public leaderboard proof; current product leaderboard rows require held-out and leakage-audited status plus an audit manifest id."
  }
}
```

## External Product Row Import

`real_world_job_benchmark run` may accept an optional
`--quantitative-product-manifest` file when a competitor adapter has already
materialized same-corpus product-runtime rows outside the current ELF fixture run.
The manifest schema is `elf.agent_memory_quantitative_product_manifest/v1`.
Generated reports infer the quantitative row `product` from the external adapter
manifest entry matching `--adapter-id`, with `--product` available only as an
explicit override for old or ad hoc reports.

Use `real_world_job_benchmark export-quantitative-product-manifest --report
<report.json>` to derive this manifest from a generated `elf.real_world_job_report/v1`
instead of hand-writing metric rows. The export command copies the report's primary
aggregate row and matching per-query rows, rejects `ELF` self rows, and then runs
the same manifest validation used by import. The live qmd adapter sweep writes
`qmd-quantitative-product-manifest.json` and a combined
`elf-qmd-quantitative-report.json` so the same-corpus qmd row is visible in
`quantitative_scoreboard.rows` when fresh live artifacts exist.

```json
{
  "schema": "elf.agent_memory_quantitative_product_manifest/v1",
  "manifest_id": "qmd-live-real-world-2026-06-23",
  "corpus_id": "...same value as quantitative_scoreboard.corpus_id...",
  "rows": [
    {
      "product": "qmd",
      "adapter_id": "qmd_live_real_world",
      "held_out": false,
      "leakage_audited": false,
      "audit_manifest_id": null,
      "metrics": {
        "recall_at_5": 0.75,
        "ndcg_at_5": 0.601,
        "average_precision": 0.608
      },
      "metric_states": {
        "recall_at_5": "measured",
        "ndcg_at_5": "measured",
        "average_precision": "measured"
      }
    }
  ],
  "per_query_rows": [
    {
      "product": "qmd",
      "adapter_id": "qmd_live_real_world",
      "job_id": "...",
      "metrics": {
        "recall_at_5": 0.75,
        "ndcg_at_5": 0.601,
        "average_precision": 0.608
      },
      "metric_states": {
        "recall_at_5": "measured",
        "ndcg_at_5": "measured",
        "average_precision": "measured"
      }
    }
  ]
}
```

The runner must reject imported rows unless:

- the manifest `corpus_id` exactly matches the current scoreboard `corpus_id`
- each `(product, adapter_id)` matches an external adapter manifest record
- the product is not `ELF`
- aggregate rows and per-query rows carry the paired-comparison metrics
  `recall_at_5`, `ndcg_at_5`, and `average_precision`
- ranked aggregate rows have at least `ranking_query_count` matching per-query rows

Imported rows replace the matching `non_comparable_rows` entry, but they do not
automatically authorize leaderboard claims. A row marked `leaderboard_eligible`
must also be product-runtime evidence with `result_state = pass`, minimum ranked
query coverage, `ranked_candidate_source = runtime_trace`, `qrel_source =
explicit_qrels`, enough explicit qrels for every ranked query, `held_out = true`,
`leakage_audited = true`, and a non-empty `audit_manifest_id`. The current runner
requires both held-out and leakage-audit fields, plus an audit manifest id, before
an imported product row can remain marked leaderboard eligible. This keeps
hand-written, public-proxy, or non-audited rows from becoming hidden wins.

## Minimum Rows For P6

The first implementation issue after this spec must produce a machine-readable
`quantitative_scoreboard` from `real_world_job_benchmark`. The initial runner row may
calculate ranking metrics only when the fixture or adapter emits explicit
`ranked_candidate_evidence_ids`; otherwise it must mark those metrics
`not_encoded`. If only a subset of queries emits ranked candidates, ranking metrics
must use `partial_coverage` and must not make the row leaderboard eligible. It must
publish metric states, denominators, sample size, ranked query count, per-query rows,
explicit-qrel coverage, qrel source, Wilson 95% intervals for measured or partial
rate metrics, ablation rows for explicitly supplied candidate orderings, diagnostic
ablation pairwise comparisons with exact two-sided sign-test p-values,
paired-significance gating state for product rows, held-out/leakage audit state, and
controls so missing rows cannot become hidden wins. The runner may also import
same-corpus external quantitative product rows through
`elf.agent_memory_quantitative_product_manifest/v1`; this is an adapter artifact
boundary, not a manual scoring exemption. It must also keep unimplemented but
required production-memory measures visible as `not_encoded`, including source-id
mapping coverage, ingestion/indexing coverage, contradiction resolution,
propagation latency, and context-token efficiency.

The full P6 scoreboard must produce rows for:

- ELF fixture-backed memory authority and knowledge workspace jobs.
- ELF live-real-world retrieval and memory-evolution jobs where artifacts exist.
- qmd live-real-world retrieval/debug rows where artifacts exist.
- mem0/OpenMemory local SDK history/export rows where artifacts exist.
- Honcho rows as typed same-corpus blockers plus `research_gate`/`not_comparable`
  external-adapter rows until peer/session outputs, background reasoning artifacts,
  source-id mapped search/chat/context results, and token/context efficiency
  measures exist for the same corpus.
- PageIndex/OpenKB rows as `blocked` or `not_comparable` until actual product
  artifacts exist.
- Letta, OpenViking, Graphiti/Zep, RAGFlow, GraphRAG, and LightRAG rows as
  `blocked`, `not_encoded`, or `not_comparable` unless same-corpus product artifacts
  are checked in.

## Research Alignment

This benchmark contract is aligned with established retrieval and memory-evaluation
practice, but it is not itself a public leaderboard until the controls permit one:

- BEIR-style retrieval evaluation requires a shared corpus/query/qrels format and
  rank-aware metrics such as nDCG@k, MAP, and success@k for comparable retrieval
  claims.
- RAGAS-style RAG evaluation separates retrieval context recall/precision from
  answer faithfulness and response quality.
- LoCoMo-style memory evaluation shows that long-term memory requires temporal,
  multi-session, summarization, and event-grounded reasoning slices, not only
  single-turn retrieval.
- Production memory comparisons must report token/cost/latency budgets; Mem0's
  public benchmark framing treats accuracy, token cost, and latency as coupled
  production dimensions.
- Honcho's public docs and benchmark materials position it as reasoning-first
  memory with peer/session representations, background reasoning/dreaming, LongMem,
  LoCoMo, BEAM, and token-efficiency framing. ELF must treat those as required
  benchmark surfaces, not as same-corpus product results, until a Honcho adapter
  emits source-id mapped artifacts on the benchmark corpus.
- Scientific comparison requires held-out and leakage-audited corpora with audit
  manifest ids, explicit qrels, raw per-query rows, repeated or paired comparable
  runs, confidence intervals for single-row estimates, and paired product-row
  significance tests before a leaderboard claim is allowed. Ablation pairwise tests
  are diagnostic optimization evidence, not product leaderboard evidence.

## Claim Boundaries

Allowed:

- "ELF has measured evidence recall, source-ref coverage, stale suppression, and
  update/delete correctness for the rows shown."
- "Product X is not comparable on metric Y because evidence class, corpus, or
  product artifact coverage differs."
- "Product X beats ELF on metric Y" only when both rows are same-corpus,
  same-evidence-class, same-task, and comparable.

Not allowed:

- A fixture-backed pass cannot beat a provider-backed or product-runtime row.
- A public-proxy pass cannot prove PageIndex, OpenKB, hosted memory, provider-backed,
  or private-corpus product quality.
- A missing denominator cannot be reported as `1.000`.
- A `blocked`, `not_encoded`, or `not_comparable` row cannot become a win by omission.
