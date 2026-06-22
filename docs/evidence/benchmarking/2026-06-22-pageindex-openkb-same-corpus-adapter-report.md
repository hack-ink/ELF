---
type: Evidence
title: "PageIndex/OpenKB Same-Corpus Adapter Report - June 22, 2026"
description: "Typed setup-blocker evidence for PageIndex/OpenKB same-corpus comparison against ELF Source Library and Knowledge Workspace outputs."
resource: docs/evidence/benchmarking/2026-06-22-pageindex-openkb-same-corpus-adapter-report.md
status: active
authority: evidence
owner: benchmarking
last_verified: 2026-06-22
tags:
  - docs
  - evidence
  - benchmarking
  - pageindex
  - openkb
source_refs:
  - apps/elf-eval/fixtures/report_snapshots/2026-06-22-pageindex-openkb-same-corpus-adapter-report.json
  - apps/elf-eval/fixtures/real_world_external_adapters/pageindex_openkb/
code_refs:
  - Makefile.toml
related:
  - docs/evidence/benchmarking/2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.md
  - docs/spec/agent_memory_knowledge_system_v1.md
  - docs/spec/real_world_agent_memory_benchmark_v1.md
drift_watch:
  - docs/evidence/benchmarking/2026-06-22-pageindex-openkb-same-corpus-adapter-report.md
  - apps/elf-eval/fixtures/report_snapshots/2026-06-22-pageindex-openkb-same-corpus-adapter-report.json
  - apps/elf-eval/fixtures/real_world_external_adapters/pageindex_openkb/
  - Makefile.toml
---
# PageIndex/OpenKB Same-Corpus Adapter Report - June 22, 2026

Purpose: Close XY-1068 by turning the P2 reference-only PageIndex/OpenKB rows into
same-corpus typed setup blockers with explicit source ids and required materialized
outputs.
Status: evidence
Read this when: You need to know what PageIndex/OpenKB comparison evidence exists
after the P2 Knowledge Workspace closeout.
Not this document: A PageIndex product run, OpenKB product run, parity result, or
win/tie/loss comparison.
Inputs: `apps/elf-eval/fixtures/real_world_external_adapters/pageindex_openkb/`
and `apps/elf-eval/fixtures/report_snapshots/2026-06-22-pageindex-openkb-same-corpus-adapter-report.json`.

## Command

```sh
cargo make real-world-memory-pageindex-openkb
```

The command writes generated runner output to:

- `tmp/real-world-memory/pageindex-openkb/report.json`
- `tmp/real-world-memory/pageindex-openkb/report.md`

Checked-in evidence is:

- `apps/elf-eval/fixtures/real_world_external_adapters/pageindex_openkb/pageindex_long_document_tree_blocked.json`
- `apps/elf-eval/fixtures/real_world_external_adapters/pageindex_openkb/openkb_wiki_recompile_blocked.json`
- `apps/elf-eval/fixtures/report_snapshots/2026-06-22-pageindex-openkb-same-corpus-adapter-report.json`

## Result

The same-corpus PageIndex/OpenKB slice is runnable and scores as typed blockers:

| Target | Suite | Status | Jobs | Pass | Wrong result | Incomplete | Blocked | Not encoded |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| VectifyAI PageIndex | `source_library` | `blocked` | 1 | 0 | 0 | 0 | 1 | 0 |
| VectifyAI OpenKB | `knowledge_compilation` | `blocked` | 1 | 0 | 0 | 0 | 1 | 0 |

Typed state summary: 0 pass, 0 wrong_result, 0 incomplete, 2 blocked, and 0
not_encoded rows for this two-job slice. The generated runner report still marks
unrelated suites as `not_encoded`, because this task intentionally runs only the
PageIndex/OpenKB fixtures.

## Same-Corpus Outputs

PageIndex comparison now points at the ELF Source Library long-document corpus:

| Source id | Materialized ELF output | Required PageIndex output |
| --- | --- | --- |
| `article-source-record` | Long-document Source Library record with canonical URI, source kind, author, capture timestamp, and `elf_doc_ext/v1` source ref. | PageIndex tree node or path that maps back to this source id. |
| `article-hydrated-excerpt` | Hydrated excerpt with `verified=true`, content hash, excerpt hash, and source-ref hydration pointer. | Long-document traversal output with cited node path and excerpt/source-id mapping. |

OpenKB comparison now points at the ELF Knowledge Workspace corpus:

| Source/page id | Materialized ELF output | Required OpenKB output |
| --- | --- | --- |
| `project:elf-benchmark-suite` | Project page with source ids `elf-knowledge-current-truth`, `elf-knowledge-history`, and `xy848-issue-timeline`. | OpenKB wiki page export citing matching source ids. |
| `entity:qdrant-rebuild`, `concept:derived-knowledge-pages`, `issue:xy848-knowledge-pages` | Entity, concept, and issue pages with source ids `qdrant-rebuild-entity`, `derived-pages-concept`, and `xy848-current-timeline`. | OpenKB entity/concept index export with citations. |
| `project:knowledge-watch-rebuild` | Changed-source watch/rebuild output with `watch-source-original`, `watch-source-updated`, `watch-lint-output`, and `watch-memory-candidate-proposal`. | OpenKB lint output, saved exploration state, and watch/recompile trace mapped to those source ids. |

## Blockers

PageIndex remains `blocked` because no contained PageIndex installation, MCP
readback, tree artifact, cited node path output, or traversal report is checked in for
this corpus.

OpenKB remains `blocked` because no contained OpenKB product run, wiki export,
entity/concept index export, lint output, saved exploration state, or watch/recompile
trace is checked in for this corpus.

## Requirements Refinement

- Source Library comparison jobs must name source ids and source-ref hydration outputs
  before asking an external long-document tree adapter to score.
- Knowledge Workspace comparison jobs must name generated page ids, source ids,
  lint/watch outputs, and recompile traces before asking an external wiki adapter to
  score.
- Reference-only PageIndex/OpenKB rows may become typed blocked adapter jobs only when
  the blocker records the exact missing materialized outputs and preserves no-parity
  claim boundaries.

## Claim Boundary

Allowed:

- The PageIndex/OpenKB P3 fixture slice emits two same-corpus typed setup blockers.
- The blockers identify ELF materialized outputs and required PageIndex/OpenKB outputs
  for future scoring.
- The repo has a repeatable `cargo make real-world-memory-pageindex-openkb` task.

Not allowed:

- Do not claim ELF beats PageIndex or OpenKB.
- Do not claim PageIndex or OpenKB parity, win, tie, or loss.
- Do not treat blocked PageIndex/OpenKB adapter jobs as weakness or strength evidence.
- Do not claim PageIndex MCP or OpenKB product UI behavior was run.
