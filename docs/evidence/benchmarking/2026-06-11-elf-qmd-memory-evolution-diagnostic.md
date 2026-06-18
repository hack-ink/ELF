---
type: Evidence
title: "ELF/qmd Memory-Evolution Diagnostic - June 11, 2026"
description: "Checked-in benchmark evidence record: ELF/qmd Memory-Evolution Diagnostic - June 11, 2026."
resource: docs/evidence/benchmarking/2026-06-11-elf-qmd-memory-evolution-diagnostic.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-18
tags:
  - docs
  - evidence
  - benchmarking
---
# ELF/qmd Memory-Evolution Diagnostic - June 11, 2026

Goal: Explain the fresh live memory-evolution failures for ELF and qmd, and turn the
measured gaps into benchmark and optimization directions without implementing those
optimizations here.
Read this when: You need to decide whether ELF currently beats qmd on
current-vs-historical memory, supersession, delete/tombstone handling, or temporal
relation validity.
Inputs: Fresh local runs of `cargo make real-world-memory-evolution` and
`cargo make real-world-memory-live-adapters` on commit `87a388b`.
Outputs: Fixture evidence, live ELF/qmd job-level diagnosis, claim boundaries, and
future iteration directions.

## Executive Judgment

ELF does not yet have a production-quality live memory-evolution win. The fixture
suite passes, but the live adapter path still fails five of six current-vs-historical
jobs.

The narrow fresh result is:

- Fixture memory-evolution: `5/5` pass.
- ELF live memory-evolution: `1/6` pass, `5/6` wrong_result.
- qmd live memory-evolution: `0/6` pass, `6/6` wrong_result.

ELF is better than qmd on this fresh live slice only in a limited sense: ELF retrieves
all required memory-evolution evidence and passes the delete/TTL tombstone job; qmd
misses three required evidence links and fails the delete/TTL job.

That is not enough to claim ELF has solved memory evolution. The main live ELF gap is
not basic retrieval. ELF retrieves the current evidence, rationale evidence, and often
the relevant historical evidence, but the answer and trace do not explicitly encode
that a historical fact was superseded, invalidated, or preserved as history. The
scorer therefore records no conflict detection and assigns `0.0` lifecycle behavior
on the five supersession jobs.

For a memory system meant to support real agents, this is a P0 product-quality gap:
users do not only ask for the newest note. They ask what changed, why, what used to be
true, which source is current, and whether an old conclusion is stale.

## Fresh Runs

| Command | Result | Runtime |
| --- | --- | ---: |
| `cargo make real-world-memory-evolution` | pass | 50.34 seconds |
| `cargo make real-world-memory-live-adapters` | pass | 112.26 seconds |

The live adapter command emitted repeated Qdrant client/server compatibility warnings,
but it completed and wrote ELF and qmd reports. Treat the warning as benchmark-harness
risk, not as a run failure.

## Fixture Baseline

`cargo make real-world-memory-evolution` proves the benchmark contract itself can
score the intended behavior:

| Metric | Value |
| --- | ---: |
| Jobs | `5` |
| Pass | `5` |
| Wrong result | `0` |
| Mean score | `1.000` |
| Expected evidence recall | `11/11` |
| Evidence coverage | `11/11` |
| Conflict detections | `5` |
| Update rationales available | `5` |
| History-readback encoded jobs | `1` |

This is fixture evidence. It proves the scenario contract is encoded and scored. It
does not prove the ELF live service or qmd CLI path can produce the same behavior.

## Live Full-Sweep Context

The fresh live sweep changed the qmd full-suite shape compared with the previous
coverage audit:

| Adapter | Jobs | Pass | Wrong result | Blocked | Not encoded | Mean score | Mean latency | Expected evidence recall | Evidence coverage |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ELF live service adapter | `38` | `18` | `5` | `2` | `13` | `0.525` | `8.620 ms` | `41/77` | `48/84` |
| qmd live CLI adapter | `38` | `17` | `6` | `2` | `13` | `0.486` | `691.163 ms` | `38/77` | `45/84` |

Do not turn this into a broad win claim. The difference is explained by this
memory-evolution slice: qmd failed the delete/TTL job that ELF passed.

## Live Memory-Evolution Result

| Adapter | Jobs | Pass | Wrong result | Mean score | Expected evidence matched | Produced evidence |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| ELF live service adapter | `6` | `1` | `5` | `0.492` | `13/13` | `13` |
| qmd live CLI adapter | `6` | `0` | `6` | `0.325` | `10/13` | `10` |

### Job Matrix

| Job | ELF status | ELF score | qmd status | qmd score | Diagnosis |
| --- | --- | ---: | --- | ---: | --- |
| `memory-evolution-benchmark-verdict-001` | wrong_result | `0.40` | wrong_result | `0.15` | ELF retrieved current verdict, caveat, and rationale, but did not cite the old not-ready verdict as historical. qmd also missed the private-corpus caveat evidence. |
| `memory-evolution-deploy-method-001` | wrong_result | `0.40` | wrong_result | `0.40` | Both retrieved current production runbook and supersession rationale, but neither explicitly preserved the old quickstart path as historical conflict evidence. |
| `memory-evolution-issue-state-001` | wrong_result | `0.40` | wrong_result | `0.40` | Both answered the current done state and resolution rationale, but neither surfaced the earlier blocked state as superseded history. |
| `memory-evolution-preference-001` | wrong_result | `0.40` | wrong_result | `0.15` | ELF retrieved current preference and rationale, but did not preserve the old terse preference as historical. qmd only returned the rationale evidence. |
| `memory-evolution-relation-temporal-001` | wrong_result | `0.35` | wrong_result | `0.35` | Both retrieved current and historical owners, but neither produced a scored temporal-validity explanation or update rationale. |
| `memory-evolution-delete-ttl-001` | pass | `1.00` | wrong_result | `0.50` | ELF retrieved both tombstone and current plan evidence. qmd retrieved only the current plan and missed the tombstone. |

### Dimension Pattern

For ELF's five wrong-result jobs, the pattern is consistent:

| Dimension | Score pattern |
| --- | --- |
| `answer_correctness` | `0.0` on all five wrong-result jobs |
| `evidence_grounding` | `1.0` on all five wrong-result jobs |
| `lifecycle_behavior` | `0.0` on all five wrong-result jobs |
| `trap_avoidance` | `1.0` on all five wrong-result jobs |

That means ELF usually finds the right evidence and avoids stale facts as current, but
the answer is not lifecycle-aware enough. It does not represent the historical version
as a first-class part of the answer, so the benchmark cannot credit conflict
detection.

qmd has the same lifecycle pattern, plus evidence misses:

| qmd miss | Effect |
| --- | --- |
| `verdict-bounded-private-caveat` missing | Benchmark verdict job drops to `0.15`. |
| `pref-current-concise-rationale` missing | Preference job drops to `0.15`. |
| `delete-tombstone` missing | Delete/TTL job is `wrong_result` despite answering the current plan. |

## What This Says About ELF

ELF currently looks strong at current-fact retrieval and typed source-of-truth
discipline. It is not yet strong enough at memory evolution.

The missing product behavior is a temporal reconciliation layer:

1. Detect that current and historical evidence both relate to the same claim.
2. Explain which evidence is current and which is historical.
3. Preserve old facts when the user asks what changed.
4. Mark superseded facts as no longer current without deleting their historical value.
5. Expose tombstones and invalidation evidence as answerable lifecycle facts.
6. Emit trace artifacts that show conflict candidates, current winner, historical
   loser, and update rationale.

This is why the fixture can pass while the live path fails. The fixture response is a
curated memory-evolution answer. The live adapters are retrieval-backed materializers,
not full temporal reconciliation engines.

## What ELF Should Borrow

These are optimization directions, not implemented changes in this report:

| Source/reference | Useful idea for ELF | Benchmark gate before claiming progress |
| --- | --- | --- |
| Graphiti/Zep | Temporal fact validity windows, invalidation, and current/historical graph facts. | Run the Graphiti/Zep temporal graph adapter and compare current, historical, and future-validity jobs. |
| mem0/OpenMemory | Entity-scoped memory history and user-visible memory lifecycle inspection. | Add entity/preference history readback and UI/export evidence checks. |
| Letta | Core memory blocks separate from archival memory. | Add core-vs-archival jobs that distinguish always-loaded operating context from retrieved history. |
| qmd | Local replay and candidate inspection ergonomics. | Emit ELF trace hydration with conflict candidates, demoted historical facts, and replay commands. |
| Existing ELF production ops | Tombstone and deletion semantics. | Extend delete/TTL scoring from one isolated job into update/delete/recreate history cases. |

## Next Benchmark And Report Directions

1. Live temporal reconciliation report
   - Score whether ELF can answer "what changed?" with current evidence,
     historical evidence, and update rationale in the same answer.
   - Include trace hydration for current winner, historical loser, and conflict
     resolution reason.

2. Graphiti/Zep temporal graph comparison
   - Use the existing Graphiti/Zep research gate as the next real adapter target.
   - The goal is not to copy a graph database blindly; it is to measure validity
     windows and supersession semantics against ELF.

3. mem0/OpenMemory history comparison
   - Measure preference/entity history, correction, deletion, and user-visible
     inspection.
   - This directly maps to personal agent-memory expectations.

4. qmd tombstone/delete diagnostic
   - qmd is already the retrieval-debug reference, but it missed the delete tombstone
     in this run.
   - Keep this as a measured qmd gap before using qmd as a lifecycle reference.

5. ELF trace-candidate conflict profile
   - Add a report that shows top candidates for conflict jobs, not only final mapped
     evidence ids.
   - This should make it obvious whether historical evidence was absent, present but
     unselected, or selected but not narrated.

## Claim Boundaries

Allowed claims:

- The fixture memory-evolution suite passes.
- In the fresh live memory-evolution run, ELF outscored qmd and passed one job qmd
  failed.
- ELF retrieved all required memory-evolution evidence in the live run.
- ELF still failed five of six live memory-evolution jobs because current-vs-historical
  conflict detection was not encoded in the answer behavior.

Not allowed:

- Do not claim ELF has solved memory evolution.
- Do not claim ELF broadly beats qmd as a memory system.
- Do not promote fixture memory-evolution pass into live production proof.
- Do not treat Graphiti/Zep, mem0/OpenMemory, or Letta as beaten; their strongest
  scenarios still need comparable adapter reports.

## Bottom Line

The next ELF iteration direction should prioritize temporal reconciliation over more
generic retrieval work. Retrieval is good enough to find the needed evidence in this
slice; the failing behavior is deciding and explaining how current, historical,
deleted, and superseded memories relate.
