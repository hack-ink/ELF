---
type: Evidence
title: "Live Consolidation Proposal Scoring Report - June 16, 2026"
description: "Checked-in benchmark evidence record: Live Consolidation Proposal Scoring Report - June 16, 2026."
resource: docs/evidence/benchmarking/2026-06-16-live-consolidation-proposal-scoring-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-18
tags:
  - docs
  - evidence
  - benchmarking
---
# Live Consolidation Proposal Scoring Report - June 16, 2026

Goal: Record the XY-934 live consolidation proposal scoring evidence and product
reference boundaries.
Read this when: You need to know whether ELF has live evidence for reviewable
consolidation proposal generation, source lineage, confidence, unsupported-claim
flags, and apply/defer/discard review audit transitions.
Inputs: `cargo make real-world-memory-consolidation`,
`cargo make real-world-memory-live-consolidation`,
`apps/elf-eval/fixtures/real_world_memory/consolidation/`,
`apps/elf-eval/src/bin/real_world_live_adapter.rs`, and
`docs/spec/system_consolidation_proposals_v1.md`.
Outputs: Scenario-level consolidation results, live artifacts, and typed comparison
boundaries for managed dreaming and Always-On Memory Agent style references.

## Verdict

ELF now has service-backed live consolidation proposal scoring. The narrow live
command materializes all 4 `consolidation` jobs through `ElfService` consolidation
run creation, worker proposal materialization, and review-action audit transitions.

This is not scheduled production consolidation and not live provider generation. The
run uses the deterministic fixture/manual proposal payload boundary required by
`elf.consolidation/v1`: source notes are immutable, proposals are derived outputs, and
review actions are explicit artifacts.

## Fresh Runs

| Command | Result | Artifact |
| --- | --- | --- |
| `cargo make real-world-memory-consolidation` | pass | `tmp/real-world-memory/consolidation/report.json` |
| `cargo make real-world-memory-live-consolidation` | pass | `tmp/real-world-memory/live-consolidation/summary.json` |

## ELF Live Consolidation Results

| Job | Live status | Source refs | Review action | Final review state | Unsupported claims | Source mutations |
| --- | --- | ---: | --- | --- | ---: | ---: |
| `consolidation-project-summary-apply-001` | `pass` | `2` | `apply` | `applied` | `0` | `0` |
| `consolidation-weekly-decision-summary-apply-001` | `pass` | `2` | `apply` | `applied` | `0` | `0` |
| `consolidation-preference-candidate-defer-001` | `pass` | `2` | `defer` | `archived` | `0` | `0` |
| `consolidation-contradiction-report-discard-001` | `pass` | `3` | `discard` | `rejected` | `1` | `0` |

The generated benchmark report keeps the same consolidation metrics as the fixture
report:

- `proposal_count = 4`
- `lineage_completeness = 1.0`
- `review_action_correctness = 1.0`
- `proposal_unsupported_claim_count = 1`
- `source_mutation_count = 0`
- `executable_gap_count = 0`

The materialization artifact records service-backed run ids, proposal ids, source
lineage counts, unsupported-claim flag counts, review-event counts, review actions,
and final review states. It does not claim source memory rewrites.

## Comparison Boundary

| Compared target | Position | Reason |
| --- | --- | --- |
| qmd live real-world adapter | `untested` | qmd keeps consolidation jobs typed `not_encoded`; no qmd consolidation proposal generator or review-action audit runner exists in this benchmark. |
| Managed dreaming memory systems | `product_reference` | Managed dreaming motivates the proposal-review shape, but no contained runner emits comparable source ids, confidence, unsupported-claim flags, and review audit artifacts. |
| Always-On Memory Agent patterns | `product_reference` | Always-on scheduling remains a reference only. XY-934 does not implement scheduled consolidation and does not allow silent source-of-truth rewrites. |

## Claims Allowed

- ELF live consolidation self-checks pass for proposal materialization, source
  lineage, confidence/usefulness thresholds, unsupported-claim flags, and
  apply/defer/discard audit transitions.
- Fixture consolidation passes and live service-backed consolidation evidence are
  separate evidence classes.
- qmd and other tracked projects remain untested or reference-only for live
  consolidation proposal scoring until a contained runner emits comparable artifacts.
- Derived-output safety claims are tied to source lineage, immutable source snapshots,
  zero source mutations, and review-action artifacts.

## Claims Not Allowed

- Do not claim scheduled production consolidation exists.
- Do not claim live provider-generated consolidation quality; the accepted
  `elf.consolidation/v1` service boundary is deterministic fixture/manual proposal
  materialization.
- Do not claim ELF broadly beats managed dreaming, Always-On Memory Agent,
  agentmemory, qmd, or llm-wiki on consolidation without comparable contained live
  runners.
- Do not mix knowledge-page rebuild/lint scoring into the consolidation claim.
