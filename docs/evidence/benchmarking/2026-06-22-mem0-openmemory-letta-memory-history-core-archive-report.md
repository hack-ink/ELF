---
type: Evidence
title: "mem0/OpenMemory and Letta Memory-History/Core-Archive Adapter Report - June 22, 2026"
description: "Same-corpus adapter fixture evidence for mem0 SDK history/export plus OpenMemory UI/export and Letta core/archive typed blockers."
resource: docs/evidence/benchmarking/2026-06-22-mem0-openmemory-letta-memory-history-core-archive-report.md
status: active
authority: evidence
owner: benchmarking
last_verified: 2026-06-22
tags:
  - docs
  - evidence
  - benchmarking
  - mem0
  - openmemory
  - letta
source_refs:
  - apps/elf-eval/fixtures/report_snapshots/2026-06-22-mem0-openmemory-letta-memory-history-core-archive-report.json
  - apps/elf-eval/fixtures/real_world_external_adapters/mem0_openmemory_letta/
code_refs:
  - Makefile.toml
related:
  - docs/evidence/benchmarking/2026-06-11-mem0-openmemory-history-ui-export-report.md
  - docs/evidence/benchmarking/2026-06-19-openmemory-ui-export-product-readback-report.md
  - docs/evidence/benchmarking/2026-06-19-letta-core-archive-export-readback-report.md
  - docs/evidence/benchmarking/2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.md
  - docs/spec/agent_memory_knowledge_system_v1.md
  - docs/spec/real_world_agent_memory_benchmark_v1.md
drift_watch:
  - docs/evidence/benchmarking/2026-06-22-mem0-openmemory-letta-memory-history-core-archive-report.md
  - apps/elf-eval/fixtures/report_snapshots/2026-06-22-mem0-openmemory-letta-memory-history-core-archive-report.json
  - apps/elf-eval/fixtures/real_world_external_adapters/mem0_openmemory_letta/
  - Makefile.toml
---
# mem0/OpenMemory and Letta Memory-History/Core-Archive Adapter Report - June 22, 2026

Purpose: Close XY-1069 by adding same-corpus adapter fixture evidence for mem0 SDK
memory history/export and typed blockers for OpenMemory product UI/export plus Letta
core/archive readback.
Status: evidence
Read this when: You need the current P3 adapter evidence for mem0/OpenMemory history
and Letta core/archive strengths after the P2 Knowledge Workspace closeout.
Not this document: A hosted mem0 Platform result, an OpenMemory browser/UI product
pass, a live Letta core/archive export, or any broad parity/win/loss claim.
Inputs: `apps/elf-eval/fixtures/real_world_external_adapters/mem0_openmemory_letta/`
and `apps/elf-eval/fixtures/report_snapshots/2026-06-22-mem0-openmemory-letta-memory-history-core-archive-report.json`.

## Command

```sh
cargo make real-world-memory-mem0-openmemory-letta
```

The command writes generated runner output to:

- `tmp/real-world-memory/mem0-openmemory-letta/report.json`
- `tmp/real-world-memory/mem0-openmemory-letta/report.md`

Checked-in evidence is:

- `apps/elf-eval/fixtures/real_world_external_adapters/mem0_openmemory_letta/mem0_sdk_history_export.json`
- `apps/elf-eval/fixtures/real_world_external_adapters/mem0_openmemory_letta/openmemory_ui_export_blocked.json`
- `apps/elf-eval/fixtures/real_world_external_adapters/mem0_openmemory_letta/letta_core_blocks_blocked.json`
- `apps/elf-eval/fixtures/real_world_external_adapters/mem0_openmemory_letta/letta_archival_readback_blocked.json`
- `apps/elf-eval/fixtures/report_snapshots/2026-06-22-mem0-openmemory-letta-memory-history-core-archive-report.json`

## Result

The same-corpus mem0/OpenMemory/Letta slice is runnable and scores as one pass plus
three typed blockers:

| Target | Surface | Suite | Status | Jobs | Pass | Wrong result | Incomplete | Blocked |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| mem0 SDK | Entity-scoped memory history and local `get_all` export-style readback | `memory_evolution` | `pass` | 1 | 1 | 0 | 0 | 0 |
| OpenMemory product | UI/export product readback | `production_ops` | `blocked` | 1 | 0 | 0 | 0 | 1 |
| Letta | Core block export/readback | `core_archival_memory` | `blocked` | 1 | 0 | 0 | 0 | 1 |
| Letta | Archival passage/readback/search for fallback, stale-core, and project-decision recovery | `core_archival_memory` | `blocked` | 1 | 0 | 0 | 0 | 1 |

Typed state summary: 4 jobs, 1 pass, 0 wrong_result, 0 incomplete, 3 blocked, 0
not_encoded, 0 unsupported_claim, 1 history readback encoded, 1 conflict detection,
1 update rationale, and 14/14 evidence, source-ref, and quote coverage.

## Same-Corpus Mapping

mem0 SDK output maps local OSS history/export behavior to source ids:

| Source id | mem0 SDK evidence |
| --- | --- |
| `elf-old-preference` | `Memory.history` ADD event for the original preference. |
| `elf-current-stable-preference` | `Memory.history` UPDATE event, scoped search result, and local `Memory.get_all` export-style readback. |
| `mem0-deleted-demo-preference` | `Memory.history` DELETE event plus post-delete search suppression. |
| `other-project-preference` | Scope filter decoy omitted from ELF-scoped search and `get_all`. |

OpenMemory UI/export remains a separate product surface:

| Required product output | Current state |
| --- | --- |
| Running OpenMemory product container and app database | Blocked; the probe found product files, but no running product export maps same-corpus rows. |
| Browser/API/export-helper readback with source ids | Blocked; local mem0 SDK `get_all` is not UI/export proof. |

Letta core block output must map the ELF core block source ids:

| ELF source id | Required Letta output |
| --- | --- |
| `core-attachment-active` | Exported Letta core block JSON for explicit attachment readback. |
| `core-scope-project-shared-readable`, `core-scope-private-owner` | Visibility metadata for read profile and private-owner boundaries. |
| `core-provenance-source-ref`, `core-provenance-audit-events` | Source-id provenance and audit-equivalent fields. |

Letta archival output must map fallback, stale-core, and project-decision source ids:

| ELF source id | Required Letta output |
| --- | --- |
| `fallback-archival-runbook` | Archival passage/readback/search output for fallback from insufficient core memory. |
| `archival-current-validation-gate`, `archival-supersedes-core-rationale` | Archival evidence superseding a stale core block. |
| `decision-core-routing-block`, `decision-archival-outcome-policy` | Core routing plus archival rationale for project-decision recovery. |

## Evidence Separation

- mem0 SDK evidence is local OSS SDK evidence. It strengthens the memory-history and
  export/readback comparison only for `Memory.history`, scoped `search`, and
  `Memory.get_all`.
- OpenMemory UI/export evidence is product evidence. It remains blocked until a
  running product container and app database export same-corpus rows.
- Letta core block evidence is core-memory export evidence. It remains blocked until
  exported core block JSON maps the attachment, scope, provenance, and audit source
  ids.
- Letta archival readback evidence is archival passage/readback/search evidence. It
  remains blocked until Letta output maps archival fallback, stale-core, and
  project-decision recovery ids.

## Claim Boundary

Allowed:

- The new P3 slice maps mem0 SDK same-corpus history/export outputs to source ids.
- The mem0 SDK fixture encodes history readback for ADD, UPDATE, and DELETE events.
- OpenMemory UI/export remains a typed blocker with a concrete product-container and
  app-database export requirement.
- Letta core block and archival readback comparisons remain typed blockers with
  explicit required source ids.

Not allowed:

- Do not claim hosted mem0 Platform behavior.
- Do not claim OpenMemory UI/export quality from mem0 SDK `Memory.get_all`.
- Do not claim Letta pass, parity, win, tie, or loss from ELF fixture passes alone.
- Do not claim broad memory-history or core/archive product superiority from this
  four-job adapter slice.
