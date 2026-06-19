---
type: Evidence
title: "Live Knowledge-Page Rebuild/Lint Report - June 20, 2026"
description: "Checked-in benchmark evidence record: Live Knowledge-Page Rebuild/Lint Report - June 20, 2026."
resource: docs/evidence/benchmarking/2026-06-20-live-knowledge-page-rebuild-lint-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-20
tags:
  - docs
  - evidence
  - benchmarking
---
# Live Knowledge-Page Rebuild/Lint Report - June 20, 2026

Goal: Close XY-935 by moving ELF knowledge-page rebuild/lint scoring from fixture-only
evidence into a Docker-contained service materialization command.
Read this when: You need to know whether ELF has service-native evidence for
derived knowledge pages, citation coverage, stale-source lint, unsupported sections,
rebuild metadata, previous-version diffs, backlinks, and page search.
Inputs: `cargo make real-world-memory-knowledge`,
`cargo make real-world-memory-live-knowledge`,
`apps/elf-eval/fixtures/real_world_memory/knowledge/`, and
`apps/elf-eval/src/bin/real_world_live_adapter.rs`.
Outputs: A narrow live knowledge-page benchmark command and typed comparison
boundaries for wiki, graph, and RAG-style knowledge systems.

## Executive Judgment

ELF now has a dedicated service-native knowledge-page rebuild/lint benchmark command.
The command materializes the checked-in `knowledge_compilation` jobs through
`ElfService::knowledge_page_rebuild`, `knowledge_page_lint`, and
`knowledge_pages_search`, then scores the generated real-world job fixtures.

This improves ELF's own knowledge-page authority from fixture-only page artifacts to
service-backed rebuild/lint/search evidence. It does not prove parity or superiority
against llm-wiki, gbrain, GraphRAG, RAGFlow, LightRAG, or graphify. Those comparisons
remain valid only when a contained adapter emits comparable page sections, source ids,
citation mappings, lint findings, previous-version diffs, and typed benchmark statuses.

## Command Evidence

| Command | Expected result | Artifact |
| --- | --- | --- |
| `cargo make real-world-memory-knowledge` | Fixture knowledge page gate passes. | `tmp/real-world-memory/knowledge-report.json` |
| `cargo make real-world-memory-live-knowledge` | Docker-contained ELF service materialization and scored report pass for the encoded knowledge fixture pack. | `tmp/real-world-memory/live-knowledge/summary.json` |

## Live Materialization Contract

`cargo make real-world-memory-live-knowledge` publishes:

| Artifact | Purpose |
| --- | --- |
| `tmp/real-world-memory/live-knowledge/elf-materialization.json` | Records live adapter materialization, generated fixtures, per-job evidence ids, and service-path metadata. |
| `tmp/real-world-memory/live-knowledge/elf-report.json` | Scores generated jobs with normal real-world job benchmark status and knowledge metrics. |
| `tmp/real-world-memory/live-knowledge/elf-report.md` | Human-readable report for citation coverage, stale lint, rebuild determinism, backlinks, and unsupported sections. |
| `tmp/real-world-memory/live-knowledge/summary.json` | Aggregates materialization and report summaries under `elf.real_world_knowledge_live_adapter_sweep/v1`. |

The command is intentionally Docker-scoped. Host execution is refused unless
`ELF_KNOWLEDGE_LIVE_ALLOW_HOST=1` is set for an explicit local diagnostic run.

## Scored Dimensions

| Dimension | Evidence requirement |
| --- | --- |
| Citation coverage | Page sections cite source evidence or timeline events, or are explicitly flagged unsupported. |
| Stale-source lint | Stale source updates after rebuild produce lint findings instead of silently rewriting truth. |
| Unsupported sections | Unsupported summaries remain visible as unsupported, not hidden claims. |
| Rebuild metadata | First and second rebuild hashes, deterministic status, and allowed variance remain explicit. |
| Previous-version diff | Repeated rebuilds expose `elf.knowledge_page.version_diff/v1` metadata without changing page content hashes. |
| Backlinks and search | Page artifacts expose backlinks, and `knowledge_pages_search` returns the materialized page surface. |
| Source-of-truth boundary | Knowledge pages remain derived benchmark artifacts and do not replace Memory Notes or source records. |

## Comparison Boundary

| Compared target | Current position | Why |
| --- | --- | --- |
| llm-wiki | `product_reference` | Query-save/lint and wiki maintenance are useful reference patterns, but no contained llm-wiki adapter emits comparable scored pages here. |
| gbrain | `product_reference` | Timeline and compiled-truth page shape remain references until a contained runner emits source-linked pages and lint output. |
| GraphRAG/RAGFlow/LightRAG | `blocked_or_incomplete_reference` | Graph/RAG outputs need document/text-unit/source-id mappings before they can be scored as knowledge pages. |
| graphify | `wrong_result_reference` | Existing representative graphify evidence remains typed `wrong_result`; stale-source lint and unsupported-summary handling are not passing. |
| qmd | `not_encoded` | qmd live adapter retrieves evidence-linked answers but does not generate derived knowledge pages. |

## Follow-Up Queue

| Follow-up | Reason |
| --- | --- |
| XY-1019 | Productize Knowledge Workspace pages with rebuild diffs, citation lint, unsupported-claim warnings, and previous-version diffs. |
| XY-1020 | Add graph-lite temporal facts and source-backed reports after knowledge pages remain derived and citation-checked. |
| Graph/RAG contained adapters | Promote external comparison only when adapters emit comparable source ids, page sections, citation mappings, and lint findings. |

## Claims Allowed

- ELF has a dedicated Docker-contained service-native knowledge-page rebuild/lint
  command for the checked-in `knowledge_compilation` fixture pack.
- The command exercises `knowledge_page_rebuild`, `knowledge_page_lint`, and
  `knowledge_pages_search` before scoring.
- The current service-native artifact includes previous-version diff metadata and
  reports `version_diff_coverage = 1.000`.
- ELF's own knowledge-page evidence is stronger than fixture-only proof for this
  narrow slice.

## Claims Not Allowed

- Do not claim ELF beats llm-wiki, gbrain, GraphRAG, RAGFlow, LightRAG, or graphify on
  broad knowledge products from this command alone.
- Do not treat generated knowledge pages as authoritative storage.
- Do not mark Knowledge Workspace productization complete; XY-1019 still owns page
  version diffs, product-quality rebuild metadata, broader page types, and recall
  integration.
