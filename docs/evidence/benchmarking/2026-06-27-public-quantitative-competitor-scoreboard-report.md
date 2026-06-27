---
type: Evidence
title: "Public Quantitative Competitor Scoreboard Report - June 27, 2026"
description: "Public evidence report for the ELF agent-memory quantitative competitor scoreboard and row-level comparability blockers."
resource: docs/evidence/benchmarking/2026-06-27-public-quantitative-competitor-scoreboard-report.md
status: active
authority: evidence
owner: benchmarking
last_verified: 2026-06-27
tags:
  - docs
  - evidence
  - benchmarking
  - competitor-scoreboard
source_refs:
  - apps/elf-eval/fixtures/report_snapshots/2026-06-27-public-quantitative-competitor-scoreboard-report.json
  - apps/elf-eval/fixtures/real_world_memory/
  - apps/elf-eval/fixtures/real_world_external_adapters/
code_refs:
  - Makefile.toml
  - apps/elf-eval/src/bin/real_world_job_benchmark.rs
  - apps/elf-eval/tests/real_world_job_benchmark.rs
  - docs/spec/agent_memory_quantitative_benchmark_v1.md
related:
  - docs/spec/real_world_agent_memory_benchmark_v1.md
  - docs/evidence/benchmarking/2026-06-23-p4-quality-hardening-productization-readiness-report.md
  - docs/evidence/benchmarking/2026-06-23-p3-competitor-strength-absorption-report.md
drift_watch:
  - docs/evidence/benchmarking/2026-06-27-public-quantitative-competitor-scoreboard-report.md
  - apps/elf-eval/fixtures/report_snapshots/2026-06-27-public-quantitative-competitor-scoreboard-report.json
  - apps/elf-eval/src/bin/real_world_job_benchmark.rs
  - docs/spec/agent_memory_quantitative_benchmark_v1.md
  - docs/evidence/benchmarking/index.md
  - README.md
---
# Public Quantitative Competitor Scoreboard Report - June 27, 2026

Purpose: Publish the public quantitative competitor scoreboard for agent-memory
retrieval and memory-quality evidence without turning typed blockers into broad
leaderboard claims.
Status: evidence
Read this when: You need the June 27 public scoreboard rows, quantitative metrics,
competitor strengths, typed non-pass states, or next optimization direction.
Not this document: Private-corpus production proof, provider-backed private quality
proof, or universal product-superiority evidence.
Inputs: `apps/elf-eval/fixtures/report_snapshots/2026-06-27-public-quantitative-competitor-scoreboard-report.json`.

## Commands

The checked-in snapshot was generated from the full real-world memory fixture pack:

```sh
cargo make real-world-memory-quantitative-scoreboard
```

That task writes the reproducible working artifacts to
`tmp/real-world-memory/quantitative-scoreboard/report.json` and
`tmp/real-world-memory/quantitative-scoreboard/report.md`.

The checked-in snapshot uses the same runner and arguments with an evidence output
path:

```sh
cargo run -p elf-eval --bin real_world_job_benchmark -- run \
  --fixtures apps/elf-eval/fixtures/real_world_memory \
  --out apps/elf-eval/fixtures/report_snapshots/2026-06-27-public-quantitative-competitor-scoreboard-report.json \
  --run-id public-quantitative-competitor-scoreboard \
  --adapter-id elf_real_world_memory_fixture \
  --adapter-name "ELF real-world memory fixture"
```

The Markdown renderer was also exercised against the snapshot:

```sh
cargo run -p elf-eval --bin real_world_job_benchmark -- publish \
  --report apps/elf-eval/fixtures/report_snapshots/2026-06-27-public-quantitative-competitor-scoreboard-report.json \
  --out tmp/real-world-memory/public-quantitative-competitor-scoreboard-report.md
```

The source JSON remains the authoritative machine-readable evidence for
`source_provenance[]`, row strengths, weaknesses, metrics, and `next_evidence[]`.

## Scoreboard Basis

- Schema: `elf.quality_scoreboard/v1`.
- Metric basis: `produced_evidence_order`.
- Retrieval `k`: `5`.
- Fixture run: `82` jobs across `19` encoded suites.
- Encoded job status: `75` pass, `7` blocked, `0` wrong_result, `0` incomplete,
  `0` not_encoded, and `0` unsupported_claim.
- Aggregate expected evidence recall: `1.000` (`172/172`).
- Aggregate source-ref coverage: `1.000` (`180/180`).
- Aggregate quote coverage: `1.000` (`180/180`).
- Mean fixture latency: `2.885 ms`; fixture cost: `0.000 USD`.
- Scoreboard rows: `20` tracked products.
- Aggregate scoreboard claim: `typed_non_pass_present`; unqualified win claim allowed:
  `false`.

## Public Rows

No row is comparable in this snapshot. ELF has same-corpus source-id-mapped fixture
metrics, but it is not a held-out, leakage-audited, Docker-contained product-runtime
row with container digest evidence. qmd and graphify have `live_real_world` row
evidence, but digest evidence, held-out/leakage audits, and pass-state comparable
metrics are still missing.

| Product | State | Evidence | Comparable | Runtime/Digest | Quantitative score or typed blocker | Primary source provenance |
| --- | --- | --- | --- | --- | --- | --- |
| ELF | `blocked` | `fixture_backed` | `false` | `false` / `false` | recall@5 `0.988`, precision@5 `0.415`, MRR `0.988`, nDCG `0.985`, stale/update/delete/source-ref rates `1.000`; 7 encoded blockers remain. | `apps/elf-eval/fixtures/real_world_memory/` |
| GraphRAG | `blocked` | `research_gate` | `false` | `false` / `false` | Research/setup blocker; no comparable retrieval metrics. | GraphRAG smoke/research-gate artifacts in snapshot `source_provenance[]`. |
| Graphiti/Zep | `blocked` | `research_gate` | `false` | `false` / `false` | Temporal graph validity blocker; no comparable retrieval metrics. | Graphiti/Zep smoke/research-gate artifacts in snapshot `source_provenance[]`. |
| LangGraph | `not_encoded` | `research_gate` | `false` | `false` / `false` | Persistence/work-resume scoring not encoded. | LangGraph persistence source in snapshot `source_provenance[]`. |
| Letta | `blocked` | `research_gate` | `false` | `false` / `false` | Core/archive and project-decision readback blockers remain. | Letta docs/export-readback artifacts in snapshot `source_provenance[]`. |
| LightRAG | `blocked` | `research_gate` | `false` | `false` / `false` | Retrieval/context-source adapter blocker; no comparable retrieval metrics. | LightRAG smoke/research-gate artifacts in snapshot `source_provenance[]`. |
| OpenViking | `wrong_result` | `live_baseline` | `false` | `false` / `false` | Local embedding setup passes, but retrieval/context-trajectory rows include wrong-result, blocked, and not-encoded evidence. | Live-baseline and OpenViking report artifacts in snapshot `source_provenance[]`. |
| RAGFlow | `blocked` | `research_gate` | `false` | `false` / `false` | RAGFlow retrieval/production-ops adapter blockers remain. | RAGFlow smoke artifacts in snapshot `source_provenance[]`. |
| VectifyAI OpenKB | `blocked` | `research_gate` | `false` | `false` / `false` | Same-corpus OpenKB fixture provenance exists, but product-runtime wiki/entity/concept output, source-id mapping, held-out/leakage evidence, and digest metadata remain blocked. | `apps/elf-eval/fixtures/real_world_external_adapters/pageindex_openkb/openkb_wiki_recompile_blocked.json`; `docs/evidence/benchmarking/2026-06-22-pageindex-openkb-same-corpus-adapter-report.md`. |
| VectifyAI PageIndex | `blocked` | `research_gate` | `false` | `false` / `false` | Same-corpus PageIndex fixture provenance exists, but product-runtime tree artifacts, cited node paths, traversal output, source-id mapping, held-out/leakage evidence, and digest metadata remain blocked. | `apps/elf-eval/fixtures/real_world_external_adapters/pageindex_openkb/pageindex_long_document_tree_blocked.json`; `docs/evidence/benchmarking/2026-06-22-pageindex-openkb-same-corpus-adapter-report.md`. |
| agentmemory | `blocked` | `live_baseline` | `false` | `false` / `false` | Same-corpus retrieval has evidence, but lifecycle/capture/work-resume blockers remain. | agentmemory live-baseline artifacts in snapshot `source_provenance[]`. |
| claude-mem | `wrong_result` | `live_baseline` | `false` | `false` / `false` | Durable/progressive-disclosure strengths exist, but live-baseline, capture, and operator rows remain wrong-result or blocked. | claude-mem live-baseline artifacts in snapshot `source_provenance[]`. |
| gbrain | `blocked` | `research_gate` | `false` | `false` / `false` | Knowledge-compilation/operator-debug scoring not encoded. | gbrain source records in snapshot `source_provenance[]`. |
| graphify | `wrong_result` | `live_real_world` | `false` | `true` / `false` | Docker graph-report generation reaches runtime, but current scored row has wrong-result plus blocked/not-encoded evidence and lacks digest evidence. | graphify smoke artifacts in snapshot `source_provenance[]`. |
| llm-wiki | `not_encoded` | `research_gate` | `false` | `false` / `false` | Knowledge/work-resume scoring not encoded. | llm-wiki source record in snapshot `source_provenance[]`. |
| mem0/OpenMemory | `blocked` | `live_baseline` | `false` | `false` / `false` | History, personalization, delete audit, and local export readback strengths exist; product UI/export and broader personalization rows remain blocked/not encoded. | mem0/OpenMemory live-baseline artifacts in snapshot `source_provenance[]`. |
| memsearch | `not_encoded` | `live_baseline` | `false` | `false` / `false` | Markdown store, reindex, and same-corpus retrieval strengths exist; retrieval/evolution/trust rows are not encoded as comparable product runtime. | memsearch live-baseline artifacts in snapshot `source_provenance[]`. |
| nanograph | `not_encoded` | `research_gate` | `false` | `false` / `false` | Memory-evolution and retrieval scoring not encoded. | nanograph source record in snapshot `source_provenance[]`. |
| plastic-labs Honcho | `blocked` | `research_gate` | `false` | `false` / `false` | Requested public comparison row with source provenance, but no same-corpus benchmark adapter, product-runtime output, source-id mapping, held-out/leakage evidence, latency/cost/resource metrics, or digest evidence is checked in. | Honcho repository and documentation source records in snapshot `source_provenance[]`. |
| qmd | `wrong_result` | `live_real_world` | `false` | `true` / `false` | CLI retrieval/replay and targeted live-pass strengths exist, but current full live-real-world rows include wrong-result/not-encoded/blocker states and no digest evidence. | qmd live-adapter and live-baseline artifacts in snapshot `source_provenance[]`. |

## Strengths and Weaknesses

ELF strengths in this snapshot:

- Complete expected-evidence, source-ref, and quote coverage for encoded fixture jobs.
- Zero unsupported claims, wrong results, stale answers, stale retrievals, redaction
  leaks, or scope violations in the generated fixture report.
- Work Continuity readback metrics are encoded: reset/resume, decision-rationale
  recall, rejected-option suppression, explicit next-step precision, inferred-step
  labeling, handoff source-ref coverage, redaction, and janitor false-promotion
  boundaries are all reported.
- Lifecycle scoreboard metrics for stale suppression, update correctness, and delete
  correctness are all `1.000` for encoded rows.

Competitor strengths preserved by the scoreboard:

- qmd remains a strong local CLI retrieval/replay reference with live-real-world
  adapter evidence.
- mem0/OpenMemory keeps measured strengths for history, entity-scoped
  personalization, deletion audit, and local export-style readback.
- claude-mem keeps progressive-disclosure and durable local repository strengths.
- memsearch keeps Markdown canonical-store, reindex, and same-corpus retrieval
  strengths.
- graphify has Docker runtime graph-report generation evidence.
- OpenViking has local embedding setup evidence and remains the trajectory reference,
  but its context-trajectory comparison rows are still blocked.
- VectifyAI PageIndex remains the long-document tree retrieval and PageIndex MCP
  reference, now represented as a typed same-corpus blocker row.
- VectifyAI OpenKB remains the compiled wiki, concept/entity index, lint, watch, and
  recompile workflow reference, now represented as a typed same-corpus blocker row.
- plastic-labs Honcho is tracked as a requested public comparison target with source
  provenance only; no product-runtime strength is scored in this snapshot.

Shared weaknesses and claim boundaries:

- No tracked product row is a comparable product-runtime pass in this snapshot.
- Missing held-out split and leakage-audit evidence block all rows.
- Missing container image digest evidence blocks live-real-world rows from
  comparability.
- Missing product-runtime source-id mapping blocks every external row from
  comparability, including PageIndex, OpenKB, and Honcho.
- Research-gate, fixture-backed, live-baseline, blocked, not-encoded, and
  wrong-result evidence must remain visible and cannot be collapsed into wins or
  parity.

## Optimization Direction

Next optimization work should:

- Capture Docker image digests and runtime metadata for product-runtime rows.
- Add held-out and leakage-audit manifests before broad competitor comparisons.
- Promote external adapters from typed blockers to same-corpus source-id-mapped
  runtime rows only after they emit comparable evidence.
- Add Honcho runtime adapter output before scoring Honcho retrieval, memory-quality,
  or work-continuity behavior.
- Use row-level metrics for optimization direction; do not claim a universal
  leaderboard.

This report supports a public quantitative scoreboard shape and a current evidence
snapshot. It does not prove private-corpus, provider-backed, hosted managed-memory,
or universal product-superiority claims.
