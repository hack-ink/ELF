---
type: Spec
title: "Model Ladder and Background Organizer v1"
description: "Define the ELF model ladder, model profiles, and proposal-only local background organizer benchmark gate."
resource: docs/spec/system_model_ladder_background_organizer_v1.md
status: active
authority: normative
owner: spec
last_verified: 2026-07-04
tags:
  - docs
  - spec
  - model-ladder
  - background-organizer
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
  - docs/spec/system_memory_summary_v1.md
  - docs/spec/system_work_journal_v1.md
drift_watch:
  - docs/spec/system_model_ladder_background_organizer_v1.md
  - makefiles/benchmark-memory-b.toml
  - apps/elf-eval/src/bin/real_world_job_benchmark/main.rs
  - apps/elf-eval/src/bin/real_world_job_benchmark/artifacts/local_organizer.rs
  - apps/elf-eval/fixtures/real_world_memory/local_background_organizer/proposal_only_gate.json
---
# Model Ladder and Background Organizer v1

Purpose: Define the ELF model ladder, model profiles, and proposal-only local
background organizer benchmark gate.
Status: normative
Read this when: You are implementing, validating, or reviewing model-tier use,
background organization, local extraction, memory proposal generation, or claims
about small/local models.
Not this document: Provider adapter wire formats, full Memory Authority write
semantics, or broad competitor parity claims.
Defines: `elf.model_ladder/v1`, `elf.local_background_organizer_benchmark/v1`, model
profiles, escalation policy, and R1 benchmark acceptance rules.

## Core Rule

Small or local models are not authoritative for Memory Authority writes.

Local/background organizer output may draft low-risk structured proposals only. A
proposal is not current memory until an explicit review, strong-model gate, or policy
gate accepts the authority-changing action through the owning Memory Authority or
consolidation path.

## Model Ladder

The canonical model ladder is:

| Tier | Name | Allowed use | Authority boundary |
| --- | --- | --- | --- |
| L0 | Deterministic code | Parsing, validation, scoring, schema checks, redaction checks, source-ref matching, and fixture materialization. | May reject or score inputs; must not infer unsupported memory facts. |
| L1 | Heuristic/lightweight local | Routing hints, duplicate candidates, simple labels, low-risk grouping, and UI triage aids. | Must emit rationale and confidence; must not create authoritative notes. |
| L2 | Schema-constrained local/small model | Proposal drafting, extraction candidates, local background organization, low-risk tags, and stale/delete/correction prompts. | Proposal-only. Must pass schema, provenance, unsupported-claim, source-mutation, and escalation gates before use. |
| L3 | Strong model | Conflict analysis, multi-source synthesis, important correction drafts, promotion-review assistance, high-impact summaries, and benchmark judging assistance. | Still review/policy gated for authority mutation; may not silently write Memory Authority. |
| L4 | Explicit policy or human gate | Final authority decisions for promotion, correction, public claims, benchmark adoption, and safety-sensitive ambiguity. | Required for authority-changing work unless a narrower accepted spec explicitly delegates the transition. |

The tier is a capability and authority label, not a vendor claim. A model can be
configured into a lower tier when runtime evidence is weak or unavailable.

## Model Profiles

Model profiles are named policy slots used by benchmark and runtime configuration.
They do not require a specific provider.

| Profile | Minimum tier | Allowed output | Required gate |
| --- | --- | --- | --- |
| `memory_extraction` | L2 | Evidence-bound candidate notes or structured proposal payloads. | JSON/schema validity, source-ref coverage, unsupported-claim rate, stale/delete/correction scoring, and L3/L4 escalation for conflicts or high-impact writes. |
| `context_routing` | L1 | Layer, scope, and budget routing hints for recall or Context Pack assembly. | Scope/lifecycle enforcement remains deterministic; routing cannot bypass read_profile or authority checks. |
| `memory_summarization` | L2 | Reviewable summaries, background brief candidates, and source-linked stale markers. | Current top-of-mind summaries, contradiction repair, and multi-source synthesis require L3/L4. |
| `benchmark_judging` | L3 | Scoring assistance, rubric critique, and claim-boundary analysis. | Final benchmark adoption and public claims require L4 policy acceptance and checked-in reproducibility evidence. |
| `promotion_review` | L3 | Review assistance for applying, correcting, or rejecting proposals. | Applying a Memory Authority mutation requires explicit review or L4 policy gate through the owning service path. |

## Local Background Organizer

The R1 local background organizer is read-only with respect to source and Memory
Authority storage.

Allowed operations:

- read bounded source, journal, note-history, proposal, and benchmark fixture inputs;
- produce schema-constrained extraction candidates and reviewable proposals;
- attach source refs, source snapshots, unsupported-claim flags, staleness markers,
  contradiction markers, and escalation rationale;
- report latency, cost, and resource footprint by model tier;
- fail closed with `not_encoded`, `blocked`, or `wrong_result` evidence.

Forbidden operations:

- create, update, delete, deprecate, restore, or publish Memory Authority notes;
- mutate Source Library documents, Work Journal rows, source refs, search traces,
  graph facts, Qdrant indexes, or benchmark evidence;
- treat a fixture-only pass as product-runtime parity with OpenAI Memory/Dreaming,
  Letta sleep-time organization, LangMem/LangChain memory managers, mem0/OpenMemory,
  Graphiti/Zep, or hosted managed memory.

## Escalation Policy

L2 output must escalate to L3 or L4 for:

- conflict resolution;
- multi-source synthesis;
- promotion review;
- important corrections;
- benchmark judging;
- public claims;
- graph facts, current top-of-mind summaries, contradiction repair, or autonomous
  consolidation.

Escalation must be visible in proposal payloads, benchmark reports, or review traces.

## Benchmark Gate

The canonical R1 task is:

```sh
cargo make real-world-memory-r1-local-organizer
```

The task runs fixture-backed local/background organizer jobs, validates the gate, and
publishes a Markdown report under `tmp/real-world-memory/r1-local-organizer/`.

The generated report must include:

- extraction F1, or an explicit `not_encoded` blocker when no labeled extraction set
  exists;
- JSON/schema validity;
- citation/source-ref coverage;
- unsupported-claim rate;
- stale/correction/delete behavior;
- zero source mutation and zero silent Memory Authority mutation;
- escalation rate;
- mean and p95 latency;
- cost and resource footprint by model tier;
- product/runtime commit and reproducibility provenance when runtime adapters are
  used.

The gate fails when schema validity, citation/source-ref coverage, unsupported-claim
rate, stale/correction/delete behavior, source immutability, Memory Authority
immutability, escalation completion, latency reporting, tier resource reporting, or
runtime provenance is missing or below the fixture threshold.

## Claim Grammar

Allowed claims after the R1 benchmark passes:

- ELF has a defined model ladder.
- ELF has a benchmarked proposal-only local/background organizer slice.
- Small/local models may draft low-risk structured proposals after passing the slice
  gates.
- Strong models or explicit policy gates remain required for authority-changing work.

Disallowed claims:

- Small/local models are safe defaults for Memory Authority writes.
- Local extraction is broadly good enough for graph facts, current top-of-mind
  summaries, contradiction repair, autonomous consolidation, or public benchmark
  judging.
- ELF has OpenAI Dreaming, Letta sleep-time, LangMem/LangChain, mem0/OpenMemory,
  Graphiti/Zep, or hosted managed-memory parity from this slice.
- Fixture-only or blocked adapter rows are product-runtime wins.
