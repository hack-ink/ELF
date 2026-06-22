---
type: Evidence
title: "P2 Knowledge Workspace PageIndex/OpenKB Closeout Report - June 22, 2026"
description: "Self-assessment and benchmark evidence for the P2 Knowledge Workspace closeout against PageIndex/OpenKB-style strengths."
resource: docs/evidence/benchmarking/2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.md
status: active
authority: evidence
owner: benchmarking
last_verified: 2026-06-22
tags:
  - docs
  - evidence
  - benchmarking
  - knowledge-workspace
source_refs:
  - apps/elf-eval/fixtures/report_snapshots/2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.json
code_refs:
  - Makefile.toml
  - apps/elf-eval/fixtures/real_world_memory/source_library/
  - apps/elf-eval/fixtures/real_world_memory/knowledge/
related:
  - docs/spec/agent_memory_knowledge_system_v1.md
  - docs/spec/system_knowledge_pages_v1.md
  - docs/spec/real_world_agent_memory_benchmark_v1.md
  - docs/runbook/benchmarking/real_world_agent_memory_benchmark.md
drift_watch:
  - docs/evidence/benchmarking/2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.md
  - apps/elf-eval/fixtures/report_snapshots/2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.json
  - apps/elf-eval/fixtures/real_world_memory/source_library/
  - apps/elf-eval/fixtures/real_world_memory/knowledge/
  - Makefile.toml
---
# P2 Knowledge Workspace PageIndex/OpenKB Closeout Report - June 22, 2026

Purpose: Close XY-1066 by measuring ELF Knowledge Workspace and Source Library
behavior against PageIndex/OpenKB-style strengths without converting reference-only
competitors into wins.
Status: evidence
Read this when: You need to decide what ELF can prove for long-document sources,
derived pages, lint, watch/rebuild, and source refs before queuing P3 adapter work.
Not this document: A contained PageIndex/OpenKB adapter result, live private-corpus
proof, or product UI readback.
Inputs: `apps/elf-eval/fixtures/real_world_memory/source_library/`,
`apps/elf-eval/fixtures/real_world_memory/knowledge/`, and
`apps/elf-eval/fixtures/report_snapshots/2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.json`.

## Command

```sh
cargo make real-world-memory-p2-knowledge-closeout
```

The command runs the same-corpus ELF fixture slices and writes:

- `tmp/real-world-memory/source-library-report.json`
- `tmp/real-world-memory/source-library-report.md`
- `tmp/real-world-memory/knowledge-report.json`
- `tmp/real-world-memory/knowledge-report.md`

The checked-in JSON snapshot for this closeout is:

- `apps/elf-eval/fixtures/report_snapshots/2026-06-22-p2-knowledge-workspace-pageindex-openkb-closeout-report.json`

## Result

The P2 closeout self-assessment passes for ELF-owned fixture evidence:

| Slice | Status | Jobs | Pass | Wrong result | Incomplete | Blocked | Not tested |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `source_library` | `pass` | 2 | 2 | 0 | 0 | 0 | 0 |
| `knowledge_compilation` | `pass` | 3 | 3 | 0 | 0 | 0 | 0 |
| VectifyAI PageIndex | `not_tested` | 0 | 0 | 0 | 0 | 0 | 1 |
| VectifyAI OpenKB | `not_tested` | 0 | 0 | 0 | 0 | 0 | 1 |
| P3 adapter queue | `blocked` | 0 | 0 | 0 | 0 | 1 | 0 |

Typed state summary: 2 pass, 0 wrong_result, 0 incomplete, 1 blocked, and 2
not_tested rows.

## What ELF Can Prove

- Long-document source handling: Source Library fixtures preserve canonical source
  metadata, source refs, hydrated excerpts, and the source-only boundary before any
  memory promotion.
- Derived pages: Knowledge fixtures compile project, entity, concept, and issue
  timeline pages with citations, backlinks, unsupported-section flags, and stale lint.
- Watch/rebuild: The changed-source fixture selects cited pages, reports changed and
  stale sections, emits previous-version diff metadata, and routes memory candidates
  through review instead of mutating source records or Memory Notes.
- Source refs: Both slices require source refs and quote-backed evidence; generated
  pages and source records stay pointer-backed benchmark artifacts.

## PageIndex/OpenKB Boundary

PageIndex remains a reference for vectorless long-document tree retrieval,
long-PDF traversal, cited node paths, and MCP product behavior. This closeout does
not run PageIndex and does not score PageIndex artifacts.

OpenKB remains a reference for document-to-wiki compilation, concept/entity pages,
saved explorations, lint, watch, and recompile workflows. This closeout does not run
OpenKB and does not score OpenKB artifacts.

Because no contained PageIndex/OpenKB adapter emits same-corpus artifacts here, both
reference projects remain `not_tested`. That is intentional: the report compares ELF
outputs to reference expectations, not to product outputs.

## Self-Assessment

Verdict: `pass_with_reference_only_competitor_boundary`.

Improved:

- The closeout now has a dedicated `cargo make real-world-memory-p2-knowledge-closeout`
  command that reruns the source-library and knowledge fixture slices together.
- The knowledge slice now includes a changed-source watch/rebuild fixture with stale
  lint, version diff, and reviewed memory-candidate boundaries.
- The report names PageIndex/OpenKB expectations without upgrading them to win/tie/loss
  claims.

Stayed bounded:

- This is fixture-backed same-corpus ELF evidence, not a live external adapter run.
- PageIndex and OpenKB remain `not_tested` until contained adapters emit comparable
  tree/wiki artifacts, source refs, lint/watch output, and typed benchmark states.
- This does not prove private-corpus, hosted-provider, or product UI quality.

Regressed:

- No regression is detected in this closeout slice: source-library and knowledge
  fixture rows are pass, with zero wrong_result and zero unsupported-claim states.

## P3 Queue Decision

P3 PageIndex/OpenKB adapter work is decision-ready after main-thread acceptance of
this closeout. The next issue should be one contained adapter task that emits
same-corpus PageIndex tree artifacts and OpenKB wiki artifacts with source refs,
lint/watch output, and typed pass/wrong_result/incomplete/blocked/not_tested states.

This report does not apply `decodex:queued:elf` to any P3 issue.

## Claim Boundary

Allowed:

- ELF P2 Knowledge Workspace closeout passes its checked-in source-library and
  knowledge fixture self-assessment.
- ELF can compare its outputs to PageIndex/OpenKB-style expectations only as reference
  expectations in this closeout.
- P3 adapter work is decision-ready after main-thread acceptance.

Not allowed:

- Do not claim ELF beats PageIndex or OpenKB.
- Do not treat reference-only PageIndex/OpenKB rows as pass evidence.
- Do not queue a P3 issue in this lane.
- Do not claim live private-corpus, hosted-provider, or product UI quality from this
  fixture-backed closeout.
