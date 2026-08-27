---
type: Evidence
title: "qmd Candidate-Replay Comparability Gate"
description: "Evidence report for the XY-1156 qmd candidate-replay comparability gate and claim boundary."
resource: docs/evidence/benchmarking/2026-07-03-qmd-candidate-replay-comparability-gate.md
status: active
authority: evidence
owner: benchmarking
last_verified: 2026-07-03
tags:
  - docs
  - evidence
  - benchmarking
  - qmd
  - comparability
source_refs: []
code_refs:
  - Makefile.toml
  - scripts/materialize-qmd-candidate-replay-gate.py
  - apps/elf-eval/src/bin/real_world_job_benchmark/main.rs
  - apps/elf-eval/src/bin/real_world_job_benchmark/quantitative.rs
  - apps/elf-eval/src/bin/real_world_job_benchmark/quantitative_reports.rs
  - docs/spec/agent_memory_quantitative_benchmark_v1.md
related:
  - docs/evidence/benchmarking/2026-06-27-public-quantitative-competitor-scoreboard-report.md
  - docs/spec/agent_memory_quantitative_benchmark_v1.md
drift_watch:
  - Makefile.toml
  - scripts/materialize-qmd-candidate-replay-gate.py
  - apps/elf-eval/src/bin/real_world_job_benchmark/quantitative.rs
  - apps/elf-eval/src/bin/real_world_job_benchmark/quantitative_reports.rs
  - docs/spec/agent_memory_quantitative_benchmark_v1.md
---
# qmd Candidate-Replay Comparability Gate - July 3, 2026

## Purpose

XY-1156 adds a qmd-specific comparability gate for the Docker-owned quantitative
benchmark path. The gate prevents qmd candidate replay evidence from becoming a
general leaderboard claim unless the artifact has the evidence needed for a
qualified same-corpus candidate-replay comparison.

## Command Surface

Primary command:

```bash
cargo make real-world-memory-quantitative-docker
```

The command runs inside `docker-compose.baseline.yml` through the baseline runner.
It refuses to run the aggregate outside Docker and requires the baseline runner
image digest before benchmark work starts.

The Docker aggregate now emits:

| Artifact | Purpose |
| --- | --- |
| `tmp/real-world-memory/quantitative-docker/qmd-quantitative-product-manifest.json` | qmd product-runtime quantitative row and per-query replay rows exported from the live explicit-qrels adapter report. |
| `tmp/real-world-memory/quantitative-docker/quantitative-artifact-freshness-manifest.json` | Docker runner, repository, input manifest, artifact digest, container image digest, and product commit reproducibility evidence. |
| `tmp/real-world-memory/quantitative-docker/qmd-candidate-replay-comparability-gate.json` | qmd-specific typed pass/blocked gate for candidate replay comparability. |

## Gate Contract

`scripts/materialize-qmd-candidate-replay-gate.py` reads the qmd quantitative
product manifest and the freshness manifest, then checks:

| Gate | Required Evidence |
| --- | --- |
| `product_manifest_schema` | Product manifest uses `elf.agent_memory_quantitative_product_manifest/v1`. |
| `freshness_manifest_schema` | Freshness manifest uses `elf.quantitative_artifact_freshness_manifest/v1`. |
| `qmd_row_present` | At least one qmd product row exists. |
| `qmd_typed_result_state_present` | The qmd row keeps a typed pass or non-pass state. |
| `qmd_result_state_pass` | The qmd product row is a measured `pass`, not only a typed blocker. |
| `qmd_source_id_mapped` | Every qmd product and per-query row maps to the manifest `corpus_id`. |
| `qmd_held_out` | The qmd row declares held-out status. |
| `qmd_leakage_audited` | The qmd row declares leakage audit status. |
| `qmd_audit_manifest_present` | The qmd row names the audit manifest. |
| `qmd_replay_artifact_present` | Per-query rows pass and contain positive runtime candidate counts, positive expected relevance counts, and explicit qrels. |
| `qmd_candidate_replay_complete` | Every qmd per-query row has a complete passing replay artifact. |
| `qmd_aggregate_replay_fields_complete` | Aggregate ranking count, sample size, explicit-qrel count, coverage state, ranked-candidate source, and qrel source match the per-query replay rows. |
| `qmd_container_digest_present` | Matching qmd freshness evidence contains a valid hex runner image digest. |
| `qmd_product_commit_present` | Matching qmd freshness evidence contains a structured 40-hex qmd product commit. |
| `qmd_reproducibility_row_bound` | The same matching qmd freshness row contains both the runner image digest and product commit. |

If any gate fails, the output `result_state` is `blocked` and `comparable` is
`false`. The output always sets `unqualified_leaderboard_claim_allowed` to
`false`.

## Validation

Targeted tests cover:

- passing output when all source-id, held-out, leakage, replay, digest, and commit
  evidence is present;
- blocked output when audit, replay, and digest evidence are missing;
- Docker aggregate wiring so the qmd gate is emitted alongside the freshness
  manifest.

The current change intentionally does not rerun the full Docker quantitative
benchmark because the aggregate is slow. The executable contract is covered by the
materializer tests and Docker script wiring; a later production run should publish
the generated JSON artifact from the container path.

## Claim Boundary

This gate can support only this narrow claim:

> qmd candidate replay is comparable for the same corpus when all gate checks pass.

It does not support:

- an unqualified product leaderboard;
- a broad ELF-vs-qmd product win;
- qmd public reproducibility when the freshness manifest lacks image digest or
  product commit evidence;
- candidate replay comparability when per-query runtime candidate rows or explicit
  qrels are missing.
