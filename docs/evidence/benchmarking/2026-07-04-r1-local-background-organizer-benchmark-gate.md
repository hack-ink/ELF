---
type: Evidence
title: "R1 Local Background Organizer Benchmark Gate - July 4, 2026"
description: "Records the fixture-backed R1 model-ladder and proposal-only local background organizer benchmark gate."
resource: docs/evidence/benchmarking/2026-07-04-r1-local-background-organizer-benchmark-gate.md
status: active
authority: evidence
owner: benchmarking
last_verified: 2026-07-04
tags:
  - docs
  - evidence
  - benchmarking
  - model-ladder
source_refs:
  - https://linear.app/hack-ink/issue/XY-1163/r1-define-model-ladder-and-local-background-organizer-benchmark-gate
code_refs:
  - makefiles/benchmark-memory-b.toml
  - apps/elf-eval/src/bin/real_world_job_benchmark/main.rs
  - apps/elf-eval/src/bin/real_world_job_benchmark/artifacts/local_organizer.rs
  - apps/elf-eval/fixtures/real_world_memory/local_background_organizer/proposal_only_gate.json
related:
  - docs/spec/agent_memory_knowledge_system_v1.md
  - docs/spec/system_consolidation_proposals_v1.md
drift_watch:
  - makefiles/benchmark-memory-b.toml
  - apps/elf-eval/src/bin/real_world_job_benchmark/main.rs
  - apps/elf-eval/src/bin/real_world_job_benchmark/artifacts/local_organizer.rs
  - apps/elf-eval/fixtures/real_world_memory/local_background_organizer/proposal_only_gate.json
---
# R1 Local Background Organizer Benchmark Gate - July 4, 2026

Purpose: Record the fixture-backed R1 model-ladder and proposal-only local
background organizer benchmark gate.
Read this when: You need evidence for XY-1163 R1 model ladder and local organizer
claims.
Not this document: Provider-runtime quality proof, hosted managed-memory parity, or
Memory Authority write authorization.
Evidence command: `cargo make real-world-memory-r1-local-organizer`.

## Result

The R1 gate is fixture-backed and proposal-only. The canonical task runs:

```sh
cargo make real-world-memory-r1-local-organizer
```

The task materializes `tmp/real-world-memory/r1-local-organizer/report.json`, validates
it with `validate-local-organizer`, and publishes
`tmp/real-world-memory/r1-local-organizer/report.md`.

The checked-in fixture reports:

| Metric | Value |
| --- | ---: |
| Jobs | 1 |
| Passed jobs | 1 |
| Extraction F1 | `not_encoded` with explicit blocker |
| JSON/schema valid outputs | 1 |
| Citation/source-ref coverage | 1.000 |
| Unsupported claim rate | 0.000 |
| Stale/correction/delete score | 1.000 |
| Source mutation count | 0 |
| Silent Memory Authority mutation count | 0 |
| Escalation rate | 1.000 |
| Mean latency | 42.000 ms |
| P95 latency | 42.000 ms |
| Model tier reported | L2 |
| Runtime commit provenance | `fixture-runtime@2026-07-04` |

The resource footprint is fixture-local CPU evidence with `peak_memory_mb = 64`,
`network_required = false`, and zero USD runtime cost.

## Claim Boundary

This report supports only these claims:

- ELF defines the L0/L1/L2/L3/L4 model ladder.
- ELF has a fixture-backed R1 benchmark gate for proposal-only local/background
  organizer output.
- L2 local/small models may draft low-risk structured proposals only after schema,
  provenance, unsupported-claim, stale/delete/correction, source-immutability,
  escalation, latency, cost, and resource gates pass.

This report does not support claims that small/local models are safe defaults for
Memory Authority writes, graph facts, current top-of-mind summaries, contradiction
repair, autonomous consolidation, benchmark judging, public claims, OpenAI Dreaming,
Letta sleep-time organization, LangMem/LangChain memory manager parity, mem0/OpenMemory
parity, Graphiti/Zep parity, or hosted managed-memory parity.
