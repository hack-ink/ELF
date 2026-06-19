---
type: Evidence
title: "Knowledge Workspace Version-Diff Report - June 20, 2026"
description: "Checked-in benchmark evidence record: Knowledge Workspace Version-Diff Report - June 20, 2026."
resource: docs/evidence/benchmarking/2026-06-20-knowledge-workspace-version-diff-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-20
tags:
  - docs
  - evidence
  - benchmarking
---
# Knowledge Workspace Version-Diff Report - June 20, 2026

Goal: Close XY-1019's product-quality Knowledge Workspace increment by proving
derived pages expose previous-version diffs while preserving citations, lint,
rebuild determinism, search readback, and source-of-truth boundaries.
Read this when: You need to know whether ELF knowledge pages now show rebuild diffs
without turning derived pages into authoritative memory.
Inputs: `cargo make real-world-memory-live-knowledge`,
`packages/elf-service/src/knowledge.rs`,
`apps/elf-eval/src/bin/real_world_live_adapter.rs`, and
`apps/elf-eval/src/bin/real_world_job_benchmark.rs`.
Outputs: Service and benchmark evidence for `elf.knowledge_page.version_diff/v1`.

## Executive Judgment

ELF Knowledge Workspace pages now expose previous-version diff metadata under
`rebuild_metadata.previous_version_diff` and surface it as `page_version_diff` in
live benchmark artifacts. The diff records previous/new content and source hashes,
title/source/content change booleans, section added/removed/changed/unchanged counts,
section key lists, a summary, and `source_mutation_allowed = false`.

This is a product-quality readback improvement for ELF's derived knowledge pages. It
does not claim broad llm-wiki, gbrain, GraphRAG, RAGFlow, LightRAG, or graphify parity.
External comparisons still need contained adapters with comparable page sections,
source ids, citation mappings, lint findings, previous-version diffs, and typed
statuses.

## Command Evidence

| Command | Result |
| --- | --- |
| `cargo test -p elf-service knowledge::tests::previous_version_diff_records_delta_without_changing_content_hash -- --nocapture` | Passed; proves diff metadata does not perturb page content hashes. |
| `cargo test -p elf-eval --test real_world_job_benchmark live_knowledge_page_rebuild_lint_has_dedicated_docker_task -- --nocapture` | Passed; proves the live adapter and benchmark report keep the version-diff contract wired. |
| `cargo make real-world-memory-live-knowledge` | Passed; Docker-contained live materialization reports `version_diff_coverage = 1.000`. |

## Current Live Metrics

From `tmp/real-world-memory/live-knowledge/elf-report.json`:

| Metric | Value |
| --- | ---: |
| Knowledge jobs | 2 |
| Pages | 2 |
| Pages with version diff | 2 |
| Version diff coverage | 1.000 |
| Rebuild determinism | 1.000 |
| Stale claim detection | 1.000 |
| Backlink coverage | 1.000 |
| Page usefulness | 0.938 |

## Contract Boundary

| Allowed claim | Boundary |
| --- | --- |
| ELF derived pages expose previous-version diff metadata after repeated rebuilds. | The diff is readback metadata only; it must not mutate source memory. |
| Search and benchmark artifacts can show `page_version_diff`. | Page snippets remain derived artifacts and must carry citations/lint/source coverage. |
| Rebuild determinism remains stable when diff metadata is present. | The page content hash excludes previous-version diff metadata. |
| External knowledge-product comparison remains future work. | Competitors need comparable contained artifacts before any parity or win/loss claim. |

## Follow-Up Queue

| Follow-up | Reason |
| --- | --- |
| XY-1020 | Temporal graph-lite facts can now feed cited pages without making pages source truth. |
| XY-1021 | Dreaming review queue can propose page rebuilds using source-backed diffs and lint. |
| Graph/RAG contained adapters | External comparison needs comparable version-diff and citation/lint outputs. |

