# Real-World Comparison Report - June 10, 2026

Goal: Publish the post-P1 real-world agent memory benchmark evidence and adoption
implications.
Read this when: You need the checked-in evidence behind README-level real-world
benchmark claims after XY-833 and XY-861 through XY-864 landed.
Inputs: Generated reports under `tmp/real-world-memory/` and `tmp/real-world-job/`,
`apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`,
and the live-baseline reports linked from this guide.
Depends on: `docs/spec/real_world_agent_memory_benchmark_v1.md`,
`docs/guide/benchmarking/real_world_agent_memory_benchmark.md`, and
`docs/guide/benchmarking/live_baseline_benchmark.md`.
Verification: The commands listed below were run from branch `y/elf-xy-865`. The
generated reports used runner version
`0.2.0-89d30dc04a854771f2a62f607e1d13498ccb3073-aarch64-apple-darwin`; the working
tree also contained the adapter manifest refresh recorded here.

Postscript: XY-880 superseded the live-adapter state in this report for ELF and qmd.
The successor evidence is
`docs/guide/benchmarking/2026-06-10-live-real-world-sweep-report.md`: ELF and qmd now
emit full-suite live sweep records, but neither has a full-suite live pass.

## Context

Dependency batch state at report time:

| Issue | Result | PR |
| --- | --- | --- |
| XY-833 operator-debugging UX repair | Done | `https://github.com/hack-ink/ELF/pull/147` |
| XY-861 project-decision suite | Done | `https://github.com/hack-ink/ELF/pull/151` |
| XY-862 production-ops suite | Done | `https://github.com/hack-ink/ELF/pull/148` |
| XY-863 graph temporal validity | Done | `https://github.com/hack-ink/ELF/pull/150` |
| XY-864 external adapter comparison contract | Done | `https://github.com/hack-ink/ELF/pull/149` |

This report is for the XY-865 branch `y/elf-xy-865` and PR title
`XY-865: [ELF benchmark vNext P1] Publish real-world comparison report and adoption plan`.

No private-corpus or credentialed provider checks were run for this report because no
operator-owned private manifest or routed provider credentials were supplied. Those
paths remain typed `blocked` boundaries, not passes.

## Commands

| Command | Generated artifact | Run ID | Generated at |
| --- | --- | --- | --- |
| `cargo make real-world-memory` | `tmp/real-world-memory/real-world-memory-report.{json,md}` | `real-world-memory` | `2026-06-10T04:21:32.545027Z` |
| `cargo make real-world-memory-project-decisions` | `tmp/real-world-memory/project-decisions/report.{json,md}` | `real-world-memory-project-decisions` | `2026-06-10T04:21:52.403238Z` |
| `cargo make real-world-memory-production-ops` | `tmp/real-world-memory/production-ops-report.{json,md}` | `real-world-memory-production-ops` | `2026-06-10T04:21:59.520163Z` |
| `cargo make real-world-memory-evolution` | `tmp/real-world-memory/evolution-report.{json,md}` | `real-world-memory-evolution` | `2026-06-10T04:22:06.325152Z` |
| `cargo make real-world-job-operator-ux` | `tmp/real-world-job/real-world-job-operator-ux-report.{json,md}` | `real-world-job-operator-ux` | `2026-06-10T04:22:12.28938Z` |

All generated reports used runner version
`0.2.0-89d30dc04a854771f2a62f607e1d13498ccb3073-aarch64-apple-darwin`.

## Aggregate Result

`cargo make real-world-memory` now reports `38` jobs across all `11` encoded real-world
suites:

| Metric | Value |
| --- | ---: |
| Pass | `35` |
| Incomplete | `1` |
| Blocked | `2` |
| Wrong result | `0` |
| Lifecycle fail | `0` |
| Not encoded | `0` |
| Unsupported claim | `0` |
| Mean score | `0.921` |
| Evidence coverage | `82/82` (`1.000`) |
| Source-ref coverage | `82/82` (`1.000`) |
| Quote coverage | `82/82` (`1.000`) |
| Expected evidence recall | `75/75` (`1.000`) |
| Redaction leaks | `0` |
| Scope violations | `0` |
| Temporal validity gaps | `0` |
| Qdrant rebuild cases | `2/2` pass |

Suite-level outcomes:

| Suite | Jobs | Status | Mean score | Interpretation |
| --- | ---: | --- | ---: | --- |
| `trust_source_of_truth` | 1 | `pass` | `1.000` | Source-of-truth rebuild fixture passed. |
| `work_resume` | 5 | `pass` | `1.000` | Resume and exact next-action fixtures passed. |
| `project_decisions` | 5 | `pass` | `1.000` | Current decisions, reversals, rationale, and caveats passed. |
| `retrieval` | 5 | `pass` | `1.000` | Retrieval fixtures with distractors and obsolete context passed. |
| `memory_evolution` | 6 | `pass` | `1.000` | Current-vs-historical and temporal relation validity passed. |
| `consolidation` | 4 | `pass` | `1.000` | Proposal-only consolidation passed with `0` source mutations. |
| `knowledge_compilation` | 2 | `pass` | `1.000` | Derived page fixtures passed with citation/rebuild checks. |
| `operator_debugging_ux` | 1 | `pass` | `1.000` | Aggregate stage-attribution fixture passed. |
| `capture_integration` | 2 | `pass` | `1.000` | Redaction and capture-boundary fixtures passed. |
| `production_ops` | 6 | `incomplete` | `0.500` | Three jobs passed, one is a typed dependency `incomplete`, and two are typed operator `blocked`. |
| `personalization` | 1 | `pass` | `1.000` | Scoped preference correction passed. |

## Focused P1 Slices

| Command | Jobs | Status summary | Evidence notes |
| --- | ---: | --- | --- |
| `cargo make real-world-memory-project-decisions` | 5 | `5` pass | Current decision, historical/reversed decision, validation gate, tradeoff rationale, and private-manifest caveat all passed. |
| `cargo make real-world-memory-evolution` | 5 | `5` pass | Temporal relation validity is now encoded and passing; stale answers `0`, conflict detections `5`, update rationales `5`. |
| `cargo make real-world-job-operator-ux` | 5 | `5` pass | Dropped evidence, rerank promotion, provider latency, rebuild change, and misleading relation-context debug cases passed with raw SQL needed `0`. |
| `cargo make real-world-memory-production-ops` | 6 | `3` pass, `1` incomplete, `2` blocked | Restore/Qdrant rebuild, interrupted backfill resume, and resource envelope passed; local embedding dependency, provider credentials, and private manifest remain typed non-pass boundaries. |

## External Adapter Evidence

The real-world runner loads
`apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`.
That manifest is an evidence ledger, not a leaderboard. It keeps four evidence classes
separate:

| Evidence class | Count | Meaning |
| --- | ---: | --- |
| `fixture_backed` | 1 | ELF fixture scoring through checked-in real-world jobs. |
| `live_baseline_only` | 6 | Docker same-corpus/lifecycle evidence from the live-baseline runner only. |
| `live_real_world` | 2 | Targeted ELF and qmd adapters execute representative `real_world_job` prompts and scoring. |
| `research_gate` | 12 | Source/setup/runtime/resource/retry metadata for future adapter paths; not fixture-backed or live execution evidence. |

Adapter-level status after refreshing the manifest:

| Project | Evidence class | Overall status | What is proven | What is not proven |
| --- | --- | --- | --- | --- |
| ELF | `fixture_backed` | `incomplete` | Fixture-backed real-world scoring passes 10 of 11 suites, with production-ops typed boundaries preserved. | Fixture-backed scoring is not live-service behavior; cite `elf_live_real_world` for the targeted live slice. |
| ELF | `live_real_world` | `pass` | The targeted Docker slice materializes real_world_job answers through ElfService, worker indexing, and search_raw for work_resume, retrieval, and project_decisions. | This is not yet a full 11-suite live-service run or private-corpus proof. |
| qmd | `live_baseline_only` | `pass` | Docker same-corpus retrieval, update, delete, and cold-start live-baseline checks pass. | Same-corpus checks are not real-world job scoring; cite `qmd_live_real_world` for the targeted live slice. |
| qmd | `live_real_world` | `pass` | The targeted Docker slice indexes real_world_job corpora through qmd collection add/update/embed/query and scores generated answers. | This is not yet broad RAG/graph adapter coverage or full-suite external parity. |
| agentmemory | `live_baseline_only` | `lifecycle_fail` | Same-corpus retrieval can run through current adapter. | Durable storage/cold-start lifecycle and real-world suites are blocked by the current in-memory adapter path. |
| mem0/OpenMemory | `live_baseline_only` | `wrong_result` | Local OSS setup is represented separately from hosted/OpenMemory claims. | Same-corpus retrieval was not a clean pass and no real-world job adapter is encoded. |
| memsearch | `live_baseline_only` | `wrong_result` | Markdown-first design remains a source-of-truth ergonomics reference. | Same-corpus retrieval was not a clean pass and real-world suites are incomplete/not encoded. |
| OpenViking | `live_baseline_only` | `incomplete` | Hierarchical context trajectory remains a reference direction. | Docker local-embedding setup must be pinned before fair retrieval or real-world jobs can run. |
| claude-mem | `live_baseline_only` | `wrong_result` | Progressive disclosure and local viewer remain UX references. | Current Docker evidence is not a clean same-corpus pass and progressive disclosure jobs are not encoded. |
| qmd deep profile | `research_gate` | `not_encoded` | The stress-profile command path and source metadata are recorded for a future deeper retrieval-debug run. | No expanded qmd stress artifact or broader real-world suite pass is checked in. |
| OpenViking deep profile | `research_gate` | `incomplete` | The deeper context-trajectory gate inherits the current Docker local-embedding setup blocker. | No hierarchical trajectory suite result is claimed. |
| RAGFlow, LightRAG, GraphRAG | `research_gate` | `blocked` | Official sources and setup/resource/retry expectations are recorded. | D1/D2 research, Docker runtime proof, and evidence-output mapping are required before adapter implementation. |
| Graphiti/Zep, Letta, LangGraph, nanograph, llm-wiki, gbrain, graphify | `research_gate` | `not_encoded` | D1/D2-inspired adapter directions have source/setup/runtime/resource/retry metadata. | No Docker-isolated `real_world_job` adapter has run for these projects. |

External summary counters: `21` adapter records, `19` non-ELF adapter records,
`21` Docker-default, `0` host-global-install requirements, `2` live real-world
adapters, and `12` research-gate records. Overall adapter statuses are `3` pass,
`3` wrong_result, `1` lifecycle_fail, `3` incomplete, `3` blocked, and
`8` not_encoded.

## Remaining Gaps

Every remaining non-pass state is either a follow-up or an explicit non-goal for this
report:

| Gap | Status | Follow-up or non-goal |
| --- | --- | --- |
| ELF production-ops cold-start dependency fixture | `incomplete` | `[ELF benchmark P0] Pin Docker-compatible local embedding dependency for cold-start adapter checks`. |
| ELF provider-backed production-ops gate | `blocked` | Run only with routed operator credentials; credentials were not supplied for this report. |
| ELF private production corpus | `blocked` | Supply an operator-owned sanitized private manifest; private-corpus checks were a non-goal without that manifest. |
| Full ELF live-service real-world sweep | `not_encoded` beyond targeted slice | Expand `elf_live_real_world` beyond representative work_resume, retrieval, and project_decisions jobs before claiming full live-service suite coverage. |
| Full qmd real-world job sweep | `not_encoded` beyond targeted slice | Expand `qmd_live_real_world` beyond the representative targeted slice before claiming broad real-world suite parity. |
| agentmemory durable lifecycle | `lifecycle_fail` / `blocked` | `[ELF benchmark P0] Make agentmemory adapter lifecycle-durable and fail-typed`. |
| mem0/OpenMemory same-corpus and real-world coverage | `wrong_result` / `not_encoded` | Add/fix a local OSS adapter before claiming lifecycle, personalization, or OpenMemory UI parity. |
| memsearch same-corpus and real-world coverage | `wrong_result` / `incomplete` | Fix Docker same-corpus retrieval/reindex evidence before scoring Markdown-first real-world jobs. |
| OpenViking Docker local embedding path | `incomplete` | `[ELF benchmark adapter] Pin OpenViking Docker local embedding dependency path`. |
| claude-mem durable/progressive-disclosure adapter | `wrong_result` / `not_encoded` | Add durable local repository and progressive-disclosure job coverage before UX parity claims. |
| RAGFlow, LightRAG, and GraphRAG adapter feasibility | `blocked` research gates | Run D1/D2 research on setup, resource envelope, corpus ingest, query output, source mapping, and Docker retry path before implementation. |
| Graphiti/Zep, Letta, LangGraph, nanograph, llm-wiki, gbrain, and graphify adapters | `not_encoded` research gates | Implement only after a scoped Docker path can emit evidence-linked outputs for the relevant real-world suites. |

## Adoption Implications

What ELF is better at in the current evidence:

- Evidence-bound writes, deterministic ingestion boundaries, source-of-truth discipline,
  rebuildable Qdrant indexing, scoped service APIs, and audited fixture-backed real-world
  provenance are stronger than the currently tested alternatives.
- The P1 fixture batch removed the previous real-world `wrong_result` and `not_encoded`
  aggregate gaps for project decisions, temporal relation validity, and operator
  debugging UX.

Where ELF is comparable or still being tested:

- qmd remains the strongest local retrieval-debug baseline. It passes current
  live-baseline checks and now has targeted live real-world job evidence, while ELF has
  the stronger evidence/provenance service contract.
- The fixture-backed retrieval and memory-evolution suites pass, but this is not the
  same as proving every external project on the same real-world jobs.

Where ELF is behind or not yet proven:

- Only ELF and qmd have targeted live real-world adapter evidence; no external project
  has full-suite live real-world parity yet.
- Production-ops is intentionally not a full pass because credentialed and private
  corpus checks need operator-owned inputs.
- ELF still needs to absorb external strengths: qmd-style local debug knobs,
  agentmemory/claude-mem/OpenMemory-style continuity and viewer ergonomics,
  OpenViking-style context trajectory, mem0-style entity history, and memsearch-style
  canonical local-store ergonomics.

The current adoption statement is therefore: ELF is the best-supported foundation in
this repository for high-trust evidence-linked agent memory, but this report does not
claim overall external superiority or private-corpus production proof.
