---
type: Evidence
title: "Dreaming Review Queue Report - June 20, 2026"
description: "Checked-in benchmark evidence record for the source-backed Dreaming review queue."
resource: docs/evidence/benchmarking/2026-06-20-dreaming-review-queue-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-20
tags:
  - docs
  - evidence
  - benchmarking
---
# Dreaming Review Queue Report - June 20, 2026

Goal: Close XY-1021 by expanding Dreaming from derived readback and consolidation
proposal storage into a single source-backed review queue for organization and
correction proposals.

Inputs:
`packages/elf-service/src/dreaming_review_queue.rs`,
`apps/elf-api/src/routes.rs`, `apps/elf-mcp/src/server.rs`,
`docs/spec/system_consolidation_proposals_v1.md`, and
`apps/elf-eval/fixtures/report_snapshots/2026-06-20-dreaming-review-queue-report.json`.

## Executive Judgment

ELF now has a service-native Dreaming review queue surface over
`consolidation_proposals`. This is not a new mutating worker and not a live provider
Dreaming loop. It is the readback and policy layer that lets an agent or operator
inspect proposed tags, duplicate merges, page rebuilds, memory promotions, graph
facts, proactive briefs, scheduled outputs, and corrections before any downstream
derived artifact is applied.

## Contract Coverage

| Requirement | ELF coverage |
| --- | --- |
| Source refs | Queue items expose proposal `source_refs` and immutable `source_snapshot`. |
| Affected refs | Queue items expose `target_ref` plus payload-level affected pages, memories, facts, and notes. |
| Confidence | Queue items preserve proposal confidence and use it in auto-apply policy. |
| Unsupported-claim lint | Queue items expose `unsupported_claim_flags`, contradiction markers, and staleness markers. |
| Diff | Queue items expose the reviewable proposal `diff`. |
| Review audit | Queue items include current review state, available actions, last reviewer metadata, and append-only review events. |
| Source mutation safety | Queue policy returns `source_mutation_allowed = false` and blocks source-mutation-key payloads from auto-apply. |

## Variant Coverage

| Variant | Coverage source | Auto-apply policy |
| --- | --- | --- |
| `memory_summary` | Existing service-native Dreaming suite | Reviewable derived output only. |
| `proactive_brief` | Existing service-native Dreaming suite | Reviewable derived output only. |
| `scheduled_memory` | Existing service-native Dreaming suite | Reviewable derived output only. |
| `tag` | Queue contract and benchmark snapshot | Candidate only after approval, confidence `>= 0.9`, no lint, and no source mutation request. |
| `duplicate_merge` | Queue contract and benchmark snapshot | Candidate only after approval, confidence `>= 0.9`, no lint, and no source mutation request. |
| `page_rebuild` | Queue contract, knowledge-page reports, and benchmark snapshot | Reviewable derived page output only. |
| `memory_promotion` | Queue contract and benchmark snapshot | High-impact review-gated memory proposal. |
| `graph_fact` | Queue contract, graph report surface, and benchmark snapshot | High-impact review-gated proposal. |
| `correction` | Queue contract and benchmark snapshot | High-impact review-gated proposal. |

## Command Evidence

| Command | Status | Purpose |
| --- | --- | --- |
| `cargo test -p elf-service dreaming_review_queue -- --nocapture` | pass | Unit-check queue variant normalization, source-mutation detection, review actions, affected-ref extraction, and auto-apply policy decisions. |
| `cargo test -p elf-mcp registers_all_tools -- --nocapture` | pass | Guard MCP tool registration for `elf_dreaming_review_queue`. |
| `cargo test -p elf-eval --test real_world_job_benchmark dreaming_review_queue_report_wires_reviewable_policy_contract -- --nocapture` | pass | Guard service/API/MCP/docs/snapshot coverage for XY-1021. |
| `cargo make check` | pass | Full repo gate: fmt/check-docs/check/clippy/vstyle plus 311 nextest tests, 87 skipped. |

## Claim Boundaries

Allowed:

- ELF exposes `elf.dreaming_review_queue/v1` as a source-backed queue over
  consolidation proposals.
- Queue items include source refs, affected refs, confidence, unsupported-claim lint,
  diff, policy, and review audit.
- Auto-apply is limited to approved low-risk derived organization candidates and
  never permits authoritative source mutation.

Not allowed:

- Do not claim provider-backed private corpus Dreaming readiness from this queue
  surface.
- Do not claim source documents, notes, traces, or graph facts can be silently mutated
  by the queue.
- Do not claim broad Dreaming product superiority without comparable external
  competitor queue artifacts.

## Next Optimization Direction

The next useful product layer is an operator UI that groups these queue items by
variant, risk, affected target, and review action. Provider-backed Dreaming remains a
separate gate: it should generate proposals into this same queue, not bypass the
review and source-mutation policy.
