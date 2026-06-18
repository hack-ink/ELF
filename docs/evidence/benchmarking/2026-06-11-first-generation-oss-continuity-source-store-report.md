---
type: Evidence
title: "First-Generation OSS Continuity and Source-Store Report - June 11, 2026"
description: "Checked-in benchmark evidence record: First-Generation OSS Continuity and Source-Store Report - June 11, 2026."
resource: docs/evidence/benchmarking/2026-06-11-first-generation-oss-continuity-source-store-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-18
tags:
  - docs
  - evidence
  - benchmarking
---
# First-Generation OSS Continuity and Source-Store Report - June 11, 2026

Goal: Expand first-generation OSS adapter coverage for durable continuity,
canonical source-store, retrieval-debug, progressive-disclosure, hook capture, and
viewer/operator surfaces without promoting smoke evidence into real-world suite pass
evidence.
Read this when: You need the XY-925 result for agentmemory, memsearch, and
claude-mem after the XY-898 first-generation adapter promotion.
Inputs: `cargo make real-world-first-generation-oss`, the external adapter manifest,
and the June 11 first-generation OSS adapter promotion report.
Outputs: Fixture-backed prompt coverage, scenario-level comparison outcomes, typed
blockers, and updated claim boundaries.

## Scope Boundary

This is benchmark/report coverage only. It does not change ELF retrieval behavior,
external project code, or baseline adapter runtime behavior.

The new first-generation fixture slice lives outside
`apps/elf-eval/fixtures/real_world_memory/`, so it is not counted as the aggregate ELF
real-world suite. The slice exists to encode comparable prompt shapes and blockers for
external OSS adapter surfaces while the external adapter manifest keeps evidence
classes explicit.

## Fresh Run

| Command | Result | Artifact |
| --- | --- | --- |
| `cargo make real-world-first-generation-oss` | pass | `tmp/real-world-memory/first-generation-oss/report.json` |

Generated report summary:

| Metric | Value |
| --- | ---: |
| Jobs | 6 |
| Encoded suites | 4 |
| Pass | 4 |
| Blocked | 2 |
| Evidence coverage | 12/12 |
| Source-ref coverage | 12/12 |
| Quote coverage | 12/12 |
| Operator-debug jobs | 2 |
| Raw SQL needed | 0 |

External adapter manifest scenario outcomes now preserve every normalized outcome:

| Outcome | Count |
| --- | ---: |
| win | 9 |
| tie | 9 |
| loss | 1 |
| not_tested | 8 |
| blocked | 6 |
| non_goal | 3 |

## Scenario Additions

| Project | Scenario | Status | Outcome | Evidence |
| --- | --- | --- | --- | --- |
| agentmemory | `durable_work_resume_local_path` | `blocked` | `blocked` | The selected comparable path is a Docker-local session directory that persists the SDK KV/index and observation log across a fresh process. |
| agentmemory | `capture_write_policy_hooks` | `blocked` | `blocked` | Live hook observations and write-policy audit evidence are required before scoring capture/write-policy jobs. |
| memsearch | `markdown_source_store_rebuild_reload_prompt` | `pass` | `not_tested` | The prompt fixture covers canonical Markdown as source of truth and `memsearch index` as derived rebuild/reload behavior. |
| memsearch | `markdown_retrieval_debug_prompt` | `pass` | `not_tested` | The prompt fixture covers CLI replay plus Markdown source inspection while keeping staged trace bundles not encoded. |
| claude-mem | `retrieval_repair_artifact_path` | `wrong_result` | `win` | The repair prompt preserves the same-corpus retrieval miss and names rerun/inspection targets `tmp/live-baseline/claude-mem.log` and `tmp/live-baseline/claude-mem-checks.json`. |
| claude-mem | `progressive_disclosure_prompt` | `pass` | `not_tested` | The prompt fixture covers repository search-to-detail/source hydration on durable SQLite. |
| claude-mem | `hook_capture_viewer_workflow` | `blocked` | `blocked` | The current Docker baseline uses repository classes only and does not execute hooks, timeline capture, or viewer workflows. |
| claude-mem | `viewer_operator_workflow` | `blocked` | `blocked` | A fair viewer/operator comparison needs Docker-contained readback over the same durable SQLite corpus. |

## Claim Boundaries

Allowed:

- agentmemory has a selected durable local path for future work-resume and
  capture/write-policy scoring.
- memsearch now has checked-in source-store and retrieval-debug prompt coverage over
  the canonical Markdown store.
- claude-mem has checked-in progressive-disclosure and retrieval-repair prompt
  coverage for the Docker-contained repository path.
- claude-mem hook capture and viewer/operator workflows remain typed blockers.

Not allowed:

- Do not claim agentmemory durable continuity from the in-memory same-corpus smoke.
- Do not claim memsearch full real-world suite parity from Markdown reindex/reload
  smoke or fixture-backed prompt coverage.
- Do not claim claude-mem retrieval passed; same-corpus retrieval remains
  `wrong_result`.
- Do not claim claude-mem hooks or viewer workflows pass from repository
  class-level hydration evidence.

## Touched Artifacts

- `Makefile.toml`: adds `cargo make real-world-first-generation-oss`.
- `apps/elf-eval/fixtures/real_world_external_adapters/first_generation_oss/`:
  checked-in prompt and blocker fixtures.
- `apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`:
  updated scenario rows and explicit `comparison_outcome` values.
- `docs/evidence/benchmarking/2026-06-11-first-generation-oss-continuity-source-store-report.md`:
  machine-readable companion report.
