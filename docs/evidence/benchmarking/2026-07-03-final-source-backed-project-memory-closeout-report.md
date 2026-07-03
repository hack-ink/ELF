---
type: Evidence
title: "Final Source-Backed Project Memory Closeout Report - July 3, 2026"
description: "XY-1157 final review-readiness evidence for ELF as source-backed project memory for AI agents."
resource: docs/evidence/benchmarking/2026-07-03-final-source-backed-project-memory-closeout-report.md
status: active
authority: evidence
owner: benchmarking
last_verified: 2026-07-03
tags:
  - docs
  - evidence
  - benchmarking
  - source-backed-project-memory
source_refs:
  - https://linear.app/hack-ink/issue/XY-1157/run-independent-review-and-decodex-closeout-for-final-elf-memory-system
code_refs:
  - Makefile.toml
  - makefiles/benchmark-memory-b.toml
  - apps/elf-eval/src/bin/real_world_job_benchmark/source_backed_quality.rs
  - apps/elf-eval/tests/real_world_job_benchmark/source_backed_quality.rs
related:
  - docs/spec/agent_memory_knowledge_system_v1.md
  - docs/spec/system_context_pack_v1.md
  - docs/spec/system_recall_debug_panel_v1.md
  - docs/spec/system_work_journal_v1.md
  - docs/spec/system_knowledge_pages_v1.md
  - docs/spec/system_consolidation_proposals_v1.md
  - docs/evidence/benchmarking/2026-07-03-source-backed-quality-benchmark-harness.md
  - docs/evidence/benchmarking/2026-07-03-qmd-candidate-replay-comparability-gate.md
  - docs/evidence/benchmarking/2026-06-27-public-quantitative-competitor-scoreboard-report.md
  - docs/evidence/benchmarking/2026-06-23-p4-quality-hardening-productization-readiness-report.md
drift_watch:
  - docs/spec/agent_memory_knowledge_system_v1.md
  - docs/evidence/benchmarking/
  - Makefile.toml
---
# Final Source-Backed Project Memory Closeout Report - July 3, 2026

Purpose: Record the XY-1157 implementation-to-validation-ready closeout evidence for
ELF as open-source, source-backed project memory for AI agents.
Status: evidence
Read this when: You are reviewing final source-backed memory readiness, claim
boundaries, independent review coverage, or next optimization tasks.
Not this document: Low-level service API semantics, fixture schemas, or operational
setup steps.

## Scope

This closeout is scoped to the accepted XY-1150 product direction and generated issue
XY-1157. It does not rename ELF into a generic Knowledge OS, broad RAG platform,
wiki compiler, hosted memory SDK, graph database, Notion clone, or document-search
replacement.

The final source-backed project memory surface is:

- Source Library
- Memory Authority
- Source-to-Memory Authority loop
- Knowledge Workspace
- Work Journal
- Dreaming Review
- Context Pack v1
- Automatic Context Routing
- Recall Engine
- Recall Debug
- benchmark harness and comparison evidence

## Changed Implementation And Evidence

XY-1151 through XY-1156 left the tree with the following checked-in implementation
and report evidence:

| Area | Evidence | Current result |
| --- | --- | --- |
| Product boundary | `docs/spec/agent_memory_knowledge_system_v1.md` | ELF is explicitly scoped as source-backed project memory for AI agents, with non-goals against generic Knowledge OS/RAG positioning. |
| Source Library and Memory Authority | `docs/evidence/benchmarking/2026-06-22-p1-memory-authority-closeout-report.md` | Source capture, memory candidate approval, recall/debug, stale suppression, correction, and rollback are covered by the P1 closeout slice. |
| Knowledge Workspace | `docs/evidence/benchmarking/2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.md` | Derived pages, citations, stale-source lint, version diffs, and changed-source watch/rebuild are evidenced without promoting pages to authoritative memory. |
| Work Journal | `apps/elf-eval/fixtures/real_world_memory/work_continuity/` and `docs/spec/system_work_journal_v1.md` | Journal readback supports continuity while journal-only facts remain non-authoritative unless promoted through memory authority. |
| Dreaming Review | `docs/evidence/benchmarking/2026-06-20-dreaming-review-queue-report.md` | Proposals expose source refs, affected refs, lint, diff, policy, and review audit; source mutation remains disallowed. |
| Context Pack v1 and routing | `docs/spec/system_context_pack_v1.md` and `docs/evidence/benchmarking/2026-07-03-source-backed-quality-benchmark-harness.md` | Packs are ephemeral read-time transport artifacts with cited items, routing traces, privacy boundaries, and activation/suppression metrics. |
| Recall Engine and Recall Debug | `docs/evidence/benchmarking/2026-06-20-recall-debug-panel-report.md` | Recall/debug reports selected, dropped, stale, blocked, and not-requested context with source refs, replay aids, and authority labels. |
| Benchmark harness | `cargo make source-backed-memory-quality` | The source-backed quality gate validates required scenarios, hard-fail counters, Context Pack routing decisions, latency, and cost. |
| qmd comparability | `docs/evidence/benchmarking/2026-07-03-qmd-candidate-replay-comparability-gate.md` | qmd candidate replay may be compared only when same-corpus mapping, held-out/leakage audit, replay rows, digest, and product commit gates pass. |

## Benchmark Metrics

The current source-backed quality report records
`source_backed_quality.result_state = "pass"` for the executable fixture/product
runtime gate. Its published July 3 run reports:

| Metric | Value |
| --- | --- |
| expected evidence recall | `1.0` |
| precision@5 | `0.414` |
| irrelevant context ratio | `0.0` |
| source-ref coverage | `1.0` |
| stale suppression rate | `1.0` |
| correction persistence rate | `1.0` |
| delete/tombstone suppression rate | `1.0` |
| unsupported claim rate | `0.0` |
| cross-scope leak count | `0` |
| journal-only authority claim count | `0` |
| Context Pack activation precision | `1.0` |
| Context Pack activation recall | `1.0` |
| activation trace coverage | `1.0` |
| mean latency | `2.864ms` |

The benchmark preserves typed non-pass evidence elsewhere in the aggregate reports.
Missing, blocked, incomplete, wrong-result, not-tested, public-proxy, local fixture,
or reference-only evidence is not treated as a pass.

## Independent Review Coverage

The final review contract for XY-1157 must explicitly cover:

- Source Library source capture, excerpt hydration, lifecycle, and no implicit memory
  promotion.
- Memory Authority note/core-block history, policy decisions, provenance, correction,
  rollback, active-only recall, and source-of-truth boundaries.
- Source-to-Memory loop proposal, approval, promotion, correction, rollback, and
  audit transitions.
- Knowledge Workspace citation, lint, stale-source, version-diff, and derived-only
  boundaries.
- Work Journal continuity readback and journal-only non-authority boundaries.
- Dreaming Review proposal queue, unsupported-claim lint, source mutation
  prohibition, and explicit review actions.
- Context Pack v1 read-time-only assembly, citations, scope/lifecycle eligibility,
  privacy-safe debug output, and no shadow memory.
- Automatic Context Routing rationale, layer selection, suppression, stale handling,
  disabled layers, blocked layers, and pinned-ineligible behavior.
- Recall Engine typed authority/freshness labels, authoritative revalidation, and
  non-pass context handling.
- Recall Debug selected/dropped/stale/blocked/not-requested evidence, replay aids,
  and privacy boundaries.
- benchmark validity, including required scenario coverage, hard-fail counters,
  typed non-pass preservation, qmd replay comparability gates, and no unqualified
  leaderboard claims.
- Decodex lifecycle/status accuracy, including the fact that `In Review` is a
  PR-backed handoff state and not phase acceptance by itself.

Any P0 or P1 finding in those areas remains a blocker for final issue completion.

## Strengths

ELF is strongest in the checked-in evidence on:

- evidence-linked memory writes;
- deterministic memory authority and policy decisions;
- source-to-memory promotion, correction, and rollback;
- Postgres source-of-truth plus rebuildable derived indexes;
- cited derived knowledge and reviewable proposal surfaces;
- Context Pack and Recall Debug authority labels, traces, and privacy boundaries;
- executable source-backed quality benchmarks with hard-fail counters.

These strengths are source-backed project memory claims, not broad product-market or
generic RAG superiority claims.

## Competitor And Unsupported Claim Boundaries

Competitor strengths remain optimization inputs:

- qmd still has a short local replay/debug ergonomics edge unless ELF emits
  comparable replay artifacts for the exact same claim.
- PageIndex/OpenKB tree/wiki artifacts remain reference or typed non-pass until
  same-corpus source-id-mapped outputs exist.
- mem0/OpenMemory history, hosted ecosystem, and UI/export surfaces remain separate
  from local SDK or fixture evidence.
- Letta core/archive parity remains blocked until exported core block and archival
  readback artifacts map to ELF source ids.
- Graphiti/Zep and graph/RAG citation/navigation strengths remain typed blockers or
  non-comparable rows unless contained same-corpus product-runtime artifacts exist.
- OpenViking context trajectory remains blocked until staged, hierarchy, and
  recursive/context expansion artifacts are available.

Unsupported claims for this closeout:

- no universal leaderboard;
- no broad "ELF beats every competitor" claim;
- no private-corpus or provider-backed production quality claim from local fixture or
  public-proxy evidence;
- no hosted managed-memory, UI/export, graph/RAG, core/archive, PageIndex/OpenKB, or
  OpenViking parity claim without comparable product-runtime evidence.

## Decodex Status Accuracy

`decodex status --json` is part of the XY-1157 validation evidence. A degraded
operator snapshot or control-plane environment warning is a Decodex runtime/status
condition, not proof that the product implementation or benchmark evidence failed.
The final issue state must still be recorded through Decodex tracker checkpoints,
repo-native validation, independent review, and PR-backed handoff.

Do not mark XY-1157 complete while any P0/P1 review finding remains unresolved, while
required validation evidence is missing, or while Decodex lifecycle/status evidence
contradicts the closeout claim.

## Next Optimization Tasks

Recommended follow-up work remains optimization, not closeout-blocking evidence:

- improve qmd-style local replay/debug ergonomics without weakening source
  authority;
- materialize PageIndex/OpenKB same-corpus tree/wiki artifacts;
- materialize OpenViking staged trajectory, hierarchy, and recursive expansion
  outputs;
- deepen mem0/OpenMemory UI/export and hosted-boundary evidence;
- deepen Letta core/archive export/readback evidence;
- add contained graph/RAG citation/navigation adapters;
- run private-corpus and provider-backed production quality gates only with
  operator-owned manifests and credentials.

## Current Validation For This Closeout

The implement-to-validation-ready lane must run at least:

- `cargo make check-docs`
- `cargo test -p elf-eval --test real_world_job_benchmark source_backed_quality`
- `cargo test -p elf-eval --test real_world_job_benchmark closeout_reports`
- `decodex status --json`

Before PR handoff or any push that refreshes a PR head, run the registered Decodex
repo gate: `cargo make fmt`, `cargo make lint-fix`, then `cargo make checks`.
