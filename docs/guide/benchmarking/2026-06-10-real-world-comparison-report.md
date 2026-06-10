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
Verification: The original commands listed below were run from branch `y/elf-xy-865`.
XY-881 refreshed `cargo make real-world-memory`, `cargo make real-world-memory-production-ops`,
and `ELF_BASELINE_PROJECTS=OpenViking cargo make baseline-live-docker` from branch
`y/elf-xy-881`. Tables below include that refresh where the OpenViking cold-start
dependency boundary is discussed.

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
| `cargo make real-world-memory` | `tmp/real-world-memory/real-world-memory-report.{json,md}` | `real-world-memory` | `2026-06-10T08:47:44.086502Z` |
| `cargo make real-world-memory-project-decisions` | `tmp/real-world-memory/project-decisions/report.{json,md}` | `real-world-memory-project-decisions` | `2026-06-10T04:21:52.403238Z` |
| `cargo make real-world-memory-production-ops` | `tmp/real-world-memory/production-ops-report.{json,md}` | `real-world-memory-production-ops` | `2026-06-10T08:47:18.205778Z` |
| `cargo make real-world-memory-evolution` | `tmp/real-world-memory/evolution-report.{json,md}` | `real-world-memory-evolution` | `2026-06-10T04:22:06.325152Z` |
| `cargo make real-world-job-operator-ux` | `tmp/real-world-job/real-world-job-operator-ux-report.{json,md}` | `real-world-job-operator-ux` | `2026-06-10T04:22:12.28938Z` |

The refreshed real-world-memory reports used runner version
`0.2.0-a8b25d00880bd3cf04707c3b2b328cd20a585396-aarch64-apple-darwin`.

## Aggregate Result

`cargo make real-world-memory` now reports `38` jobs across all `11` encoded real-world
suites:

| Metric | Value |
| --- | ---: |
| Pass | `36` |
| Incomplete | `0` |
| Blocked | `2` |
| Wrong result | `0` |
| Lifecycle fail | `0` |
| Not encoded | `0` |
| Unsupported claim | `0` |
| Mean score | `0.947` |
| Evidence coverage | `84/84` (`1.000`) |
| Source-ref coverage | `84/84` (`1.000`) |
| Quote coverage | `84/84` (`1.000`) |
| Expected evidence recall | `77/77` (`1.000`) |
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
| `production_ops` | 6 | `blocked` | `0.667` | Four jobs passed, including the pinned OpenViking cold-start classification, and two operator-owned boundaries remain `blocked`. |
| `personalization` | 1 | `pass` | `1.000` | Scoped preference correction passed. |

## Focused P1 Slices

| Command | Jobs | Status summary | Evidence notes |
| --- | ---: | --- | --- |
| `cargo make real-world-memory-project-decisions` | 5 | `5` pass | Current decision, historical/reversed decision, validation gate, tradeoff rationale, and private-manifest caveat all passed. |
| `cargo make real-world-memory-evolution` | 5 | `5` pass | Temporal relation validity is now encoded and passing; stale answers `0`, conflict detections `5`, update rationales `5`. |
| `cargo make real-world-job-operator-ux` | 5 | `5` pass | Dropped evidence, rerank promotion, provider latency, rebuild change, and misleading relation-context debug cases passed with raw SQL needed `0`. |
| `cargo make real-world-memory-production-ops` | 6 | `4` pass, `0` incomplete, `2` blocked | Restore/Qdrant rebuild, interrupted backfill resume, resource envelope, and pinned OpenViking cold-start classification passed; provider credentials and private manifest remain typed non-pass boundaries. |

## External Adapter Evidence

The real-world runner loads
`apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`.
That manifest is an evidence ledger, not a leaderboard. It keeps four evidence classes
separate:

| Evidence class | Count | Meaning |
| --- | ---: | --- |
| `fixture_backed` | 1 | ELF fixture scoring through checked-in real-world jobs. |
| `live_baseline_only` | 6 | Docker same-corpus/lifecycle evidence from the live-baseline runner only. |
| `live_real_world` | 2 | ELF and qmd adapters execute the full encoded-suite `real_world_job` sweep with typed non-pass states preserved. |
| `research_gate` | 12 | Source/setup/runtime/resource/retry metadata for future adapter paths; not fixture-backed or live execution evidence. |

XY-882 added D1/D2 feasibility verdicts inside the research-gate lane. RAGFlow
([XY-885](https://linear.app/hack-ink/issue/XY-885/elf-benchmark-adapter-implement-ragflow-docker-evidence-smoke-adapter)),
LightRAG
([XY-886](https://linear.app/hack-ink/issue/XY-886/elf-benchmark-adapter-implement-lightrag-docker-context-export-adapter)),
GraphRAG
([XY-887](https://linear.app/hack-ink/issue/XY-887/elf-benchmark-adapter-implement-graphrag-cost-bounded-docker-adapter)),
Graphiti/Zep
([XY-888](https://linear.app/hack-ink/issue/XY-888/elf-benchmark-adapter-implement-graphitizep-temporal-graph-adapter)),
and graphify
([XY-889](https://linear.app/hack-ink/issue/XY-889/elf-benchmark-adapter-implement-graphify-docker-graph-report-adapter))
are now adapter implementation candidates because they have scoped Docker boundaries
and evidence-linked output contracts. Letta, LangGraph, nanograph, and llm-wiki remain
`research_only`; gbrain remains `blocked` until a Docker-local brain repo and database
path is proven. These verdicts do not change any record into live adapter pass
evidence.

Adapter-level status after refreshing the manifest:

| Project | Evidence class | Overall status | What is proven | What is not proven |
| --- | --- | --- | --- | --- |
| ELF | `fixture_backed` | `blocked` | Fixture-backed real-world scoring passes every non-operator-owned suite and preserves the production-ops credential/private-manifest boundaries. | Fixture-backed scoring is not live-service behavior; cite `elf_live_real_world` for service-runtime sweep evidence. |
| ELF | `live_real_world` | `wrong_result` | The Docker live sweep materializes all encoded real_world_job records through ElfService, worker indexing, and search_raw; the original targeted answer-retrieval slice still passes. | This is not a full-suite live pass or private-corpus proof; typed wrong_result, incomplete, blocked, and not_encoded states remain visible. |
| qmd | `live_baseline_only` | `pass` | Docker same-corpus retrieval, update, delete, and cold-start live-baseline checks pass. | Same-corpus checks are not real-world job scoring; cite `qmd_live_real_world` for service-runtime sweep evidence. |
| qmd | `live_real_world` | `wrong_result` | The Docker live sweep indexes the encoded real_world_job corpora through qmd collection add/update/embed/query and preserves per-suite scoring evidence. | This is not a full-suite live pass or broad RAG/graph adapter coverage; typed wrong_result, incomplete, blocked, and not_encoded states remain visible. |
| agentmemory | `live_baseline_only` | `lifecycle_fail` | Same-corpus retrieval can run through current adapter. | Durable storage/cold-start lifecycle and real-world suites are blocked by the current in-memory adapter path. |
| mem0/OpenMemory | `live_baseline_only` | `wrong_result` | Local OSS setup is represented separately from hosted/OpenMemory claims. | Same-corpus retrieval was not a clean pass and no real-world job adapter is encoded. |
| memsearch | `live_baseline_only` | `wrong_result` | Markdown-first design remains a source-of-truth ergonomics reference. | Same-corpus retrieval was not a clean pass and real-world suites are incomplete/not encoded. |
| OpenViking | `live_baseline_only` | `wrong_result` | The Docker local-embedding setup is pinned and reaches `add_resource`/`find`. | The same-corpus smoke still misses expected evidence terms; no real-world job adapter or context-trajectory suite is claimed. |
| claude-mem | `live_baseline_only` | `wrong_result` | Progressive disclosure and local viewer remain UX references. | Current Docker evidence is not a clean same-corpus pass and progressive disclosure jobs are not encoded. |
| qmd deep profile | `research_gate` | `not_encoded` | The stress-profile command path and source metadata are recorded for a future deeper retrieval-debug run. | No expanded qmd stress artifact or broader real-world suite pass is checked in. |
| OpenViking deep profile | `research_gate` | `not_encoded` | The deeper context-trajectory gate can reuse the pinned Docker local-embedding setup path. | No hierarchical trajectory suite result is claimed until evidence-bearing same-corpus output is fixed. |
| RAGFlow, LightRAG, GraphRAG | `research_gate` | `blocked` | Official sources, setup/resource/retry expectations, and XY-882 adapter-candidate verdicts are recorded. | Docker runtime proof and real_world_job evidence-output mapping are still required before any live adapter claim. |
| Graphiti/Zep, Letta, LangGraph, nanograph, llm-wiki, gbrain, graphify | `research_gate` | `not_encoded` | XY-882 records Graphiti/Zep and graphify as adapter candidates, Letta/LangGraph/nanograph/llm-wiki as research-only, and gbrain as blocked. | No Docker-isolated `real_world_job` adapter has run for these projects. |

External summary counters: `21` adapter records, `19` non-ELF adapter records,
`21` Docker-default, `0` host-global-install requirements, `2` live real-world
adapters, and `12` research-gate records. Overall adapter statuses are `1` pass,
`6` wrong_result, `1` lifecycle_fail, `0` incomplete, `4` blocked, and
`9` not_encoded.
Real-world suite statuses are tracked separately as `20` pass, `3` wrong_result,
`7` incomplete, `11` blocked, and `40` not_encoded, so a setup boundary is not hidden
behind an aggregate status.

## Remaining Gaps

Every remaining non-pass state is either a follow-up or an explicit non-goal for this
report:

| Gap | Status | Follow-up or non-goal |
| --- | --- | --- |
| ELF production-ops cold-start dependency fixture | `pass` | XY-881 pins the Docker OpenViking local embedding path and preserves setup failures as `incomplete` if the wheel/import boundary fails on another platform. |
| ELF provider-backed production-ops gate | `blocked` | Run only with routed operator credentials; credentials were not supplied for this report. |
| ELF private production corpus | `blocked` | Supply an operator-owned sanitized private manifest; private-corpus checks were a non-goal without that manifest. |
| Full ELF live-service real-world sweep | `wrong_result` | XY-880 expanded `elf_live_real_world` to the full encoded suite corpus; the result is intentionally typed non-pass rather than a full-suite live pass. |
| Full qmd real-world job sweep | `wrong_result` | XY-880 expanded `qmd_live_real_world` to the full encoded suite corpus; the result is intentionally typed non-pass rather than broad real-world suite parity. |
| agentmemory durable lifecycle | `lifecycle_fail` / `blocked` | `[ELF benchmark P0] Make agentmemory adapter lifecycle-durable and fail-typed`. |
| mem0/OpenMemory same-corpus and real-world coverage | `wrong_result` / `not_encoded` | Add/fix a local OSS adapter before claiming lifecycle, personalization, or OpenMemory UI parity. |
| memsearch same-corpus and real-world coverage | `wrong_result` / `incomplete` | Fix Docker same-corpus retrieval/reindex evidence before scoring Markdown-first real-world jobs. |
| OpenViking Docker local embedding path | `wrong_result` | The pinned dependency path reaches `add_resource`/`find`; the remaining follow-up is evidence-bearing retrieval output, not setup. |
| claude-mem durable/progressive-disclosure adapter | `wrong_result` / `not_encoded` | Add durable local repository and progressive-disclosure job coverage before UX parity claims. |
| RAGFlow, LightRAG, GraphRAG, Graphiti/Zep, and graphify adapters | `research_gate` adapter candidates | Follow-up issues [XY-885](https://linear.app/hack-ink/issue/XY-885/elf-benchmark-adapter-implement-ragflow-docker-evidence-smoke-adapter), [XY-886](https://linear.app/hack-ink/issue/XY-886/elf-benchmark-adapter-implement-lightrag-docker-context-export-adapter), [XY-887](https://linear.app/hack-ink/issue/XY-887/elf-benchmark-adapter-implement-graphrag-cost-bounded-docker-adapter), [XY-888](https://linear.app/hack-ink/issue/XY-888/elf-benchmark-adapter-implement-graphitizep-temporal-graph-adapter), and [XY-889](https://linear.app/hack-ink/issue/XY-889/elf-benchmark-adapter-implement-graphify-docker-graph-report-adapter) must run only Docker-contained adapter smokes that emit evidence-linked outputs before any live result claim. |
| Letta, LangGraph, nanograph, and llm-wiki adapters | `research_only` research gates | Keep as architecture or workflow references until a contained output contract is selected. |
| gbrain adapter | `blocked` research gate | Revisit only after a Docker-local brain repo and database path can be proven without operator-owned state. |

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
