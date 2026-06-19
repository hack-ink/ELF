---
type: Evidence
title: "Graph Topic-Map Report - June 20, 2026"
description: "Checked-in benchmark evidence record: Graph Topic-Map Report - June 20, 2026."
resource: docs/evidence/benchmarking/2026-06-20-graph-topic-map-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-20
tags:
  - docs
  - evidence
  - benchmarking
---
# Graph Topic-Map Report - June 20, 2026

Goal: Close XY-1020's graph-lite product increment by proving ELF can report
Postgres-backed temporal graph facts as source-backed topic maps without introducing
a separate graph database or hidden source of truth.
Read this when: You need to know whether graph facts expose current, historical,
future, inferred, ambiguous, stale, and superseded status markers.
Inputs: `packages/elf-service/src/graph_report.rs`, `/v2/graph/report`,
`elf_graph_report`, and `docs/spec/system_graph_memory_postgres_v1.md`.
Outputs: Service, HTTP, MCP, and documentation evidence for `elf.graph_report/v1`.

## Executive Judgment

ELF now has a first-class graph report surface for one subject entity. The report
uses existing Postgres graph-lite facts, evidence links, predicate registry metadata,
validity windows, and supersession rows. It returns a topic map plus fact rows with
status markers for `sourced`, `inferred`, `ambiguous`, `stale`, and `superseded`
states.

This is an ELF-native graph-memory readback improvement. It does not claim Graphiti,
Zep, GraphRAG, RAGFlow, LightRAG, llm-wiki, gbrain, or graphify parity. Graphiti/Zep
`valid_at` and `invalid_at` vocabulary remains adapter-boundary terminology only;
ELF internal schema and reports use `valid_from` and `valid_to`.

## Command Evidence

| Command | Result |
| --- | --- |
| `cargo test -p elf-service graph_report -- --nocapture` | Passed; proves temporal/source/supersession markers and topic-map edges are shaped by service code. |
| `cargo test -p elf-mcp registers_all_tools -- --nocapture` | Passed; guards that `elf_graph_report` remains registered. |
| `cargo test -p elf-eval --test real_world_job_benchmark graph_topic_map_report_wires_source_backed_graph_lite_readback -- --nocapture` | Passed; guards the service, HTTP, MCP, spec, README, and evidence-report wiring. |
| `cargo make check` | Passed; runs formatting, docs, clippy, vstyle, and workspace tests. |

## Contract Readback

| Surface | Contract |
| --- | --- |
| Service | `ElfService::graph_report(GraphReportRequest)` returns `elf.graph_report/v1`. |
| HTTP | `/v2/graph/report` builds a source-backed graph topic-map report under the authenticated read profile. |
| MCP | `elf_graph_report` forwards to `/v2/graph/report` for agent readback. |
| Storage | Existing Postgres graph-lite tables remain authoritative; no graph database is introduced. |
| Vocabulary | Internal schema uses `valid_from`/`valid_to`; Graphiti/Zep `valid_at`/`invalid_at` remains adapter-boundary vocabulary. |

## Status Markers

| Marker | Meaning |
| --- | --- |
| `sourced` | The fact has one or more `graph_fact_evidence.note_id` links. |
| `inferred` | The predicate is pending or unresolved rather than operator-activated. |
| `ambiguous` | Multiple current facts conflict under a single-cardinality predicate. |
| `stale` | The fact is historical at the report `as_of` timestamp. |
| `superseded` | A `graph_fact_supersessions` row links the fact to a replacement. |

## Follow-Up Queue

| Follow-up | Reason |
| --- | --- |
| XY-1021 | Dreaming/background proposal review can now cite graph report markers before recommending rebuilds or mutations. |
| XY-1022 | Plugin/admin surfaces can expose graph report readback without bypassing source evidence. |
| XY-1023 | Benchmark adapters can score graph report parity only after comparable external artifacts exist. |
