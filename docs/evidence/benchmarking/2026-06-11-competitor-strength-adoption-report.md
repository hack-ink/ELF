---
type: Evidence
title: "Competitor-Strength Adoption Report - June 11, 2026"
description: "Checked-in benchmark evidence record: Competitor-Strength Adoption Report - June 11, 2026."
resource: docs/evidence/benchmarking/2026-06-11-competitor-strength-adoption-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-18
tags:
  - docs
  - evidence
  - benchmarking
---
# Competitor-Strength Adoption Report - June 11, 2026

Goal: Publish the final benchmark vNext adoption decision and scenario matrix for
ELF against tracked open-source memory, RAG, graph, and agent-continuity projects.
Read this when: You need the current production-adoption answer, the scenario-level
win/tie/loss/not-tested matrix, or the optimization queue behind future ELF work.
Inputs: `2026-06-11-measurement-coverage-audit.md`,
`2026-06-11-first-generation-oss-adapter-promotion-report.md`,
`2026-06-11-qmd-openviking-strength-profile-report.md`,
`2026-06-11-temporal-history-competitor-gap-report.md`,
`2026-06-11-graph-rag-scored-smoke-adapter-report.md`,
`2026-06-11-mem0-openmemory-history-ui-export-report.md`,
`2026-06-11-first-generation-oss-continuity-source-store-report.md`, and
`2026-06-10-production-adoption-refresh.md`.
Depends on: `docs/spec/real_world_agent_memory_benchmark_v1.md` and the current
external adapter manifest.
Outputs: Adoption decision, evidence-class boundaries, scenario matrix, follow-up
optimization queue, and the machine-readable companion file
`docs/evidence/benchmarking/2026-06-11-competitor-strength-adoption-report.md`.

## Adoption Decision

ELF is adoptable for bounded personal production use.

The verdict is `adopt_with_bounded_caveats`, not broad competitor superiority. The
supporting evidence is strongest where ELF was designed to be strong: source-of-truth
discipline, evidence-bound writes, rebuildable Qdrant derivations, backup/restore,
backfill, and typed benchmark reporting. Those properties are stronger than the
measured alternatives in the current evidence set.

The remaining caveats are material:

- Full-suite live real-world pass parity is not proven.
- Live temporal reconciliation is still a measured loss: five of six
  `memory_evolution` jobs are `wrong_result`.
- Private-corpus production quality is blocked until an operator-owned manifest
  exists.
- Credentialed provider production-ops gates are blocked until explicit provider
  setup exists.
- Several competitor strengths remain `not_tested` or blocked: OpenMemory
  UI/export is blocked by the XY-931 export-helper setup probe, hosted mem0 Platform
  behavior remains a non-goal, and OpenViking trajectory, Letta core-vs-archival
  memory, and broad graph/RAG navigation remain unproven. XY-929 adds a
  representative graph/RAG fixture slice with typed blockers, one incomplete LightRAG
  job, and one graphify wrong_result job, but it does not create any broad graph/RAG
  win, tie, or loss claim. XY-928 encodes OpenViking staged trajectory, hierarchy
  selection, and recursive/context expansion as blocked fixtures
  behind same-corpus evidence output and missing staged artifacts. XY-927 adds
  fixture-only `core_archival_memory` coverage, but Letta scenario rows remain
  blocked or `not_tested` until the selected contained export/readback path exists.
  mem0 local OSS preference history is measured separately and is an ELF loss on the
  current correction history
  scenario. The XY-923 follow-up also scores qmd's immediate top-10/replay artifact
  ergonomics as stronger than ELF's default stress report, while expansion, fusion,
  and rerank remain untested. XY-932 adds a narrow live operator-debug slice where
  ELF beats qmd on trace hydration and candidate-drop visibility, but OpenMemory
  UI/export remains blocked and claude-mem viewer workflows remain blocked until
  Docker-contained hook/viewer evidence exists. XY-925
  now adds fixture-backed first-generation OSS prompt coverage and typed blockers for
  agentmemory durable continuity, memsearch Markdown source-store/debug jobs, and
  claude-mem progressive-disclosure, retrieval-repair, hook, and viewer/operator
  surfaces; those rows still do not create live external real-world suite passes.
  XY-933 adds an ELF live capture/write-policy self-check, but agentmemory capture
  breadth is blocked by mocked/in-memory storage and claude-mem hook/viewer capture
  remains blocked until Docker-contained hook/viewer evidence exists.

## Evidence Classes

This report keeps evidence classes separate. Do not convert fixture passes,
same-corpus smokes, research gates, blocked setup, unsupported shapes, wrong
results, or lifecycle failures into one aggregate leaderboard.

| Evidence class | Meaning |
| --- | --- |
| `fixture_backed` | Checked-in real-world fixtures pass through the benchmark runner. |
| `live_baseline_only` | Docker same-corpus or lifecycle checks ran, but not full real-world jobs. |
| `live_real_world` | A runtime or CLI adapter produced scored real-world job records. |
| `smoke_only` | A tiny setup or output-shape smoke ran. |
| `research_gate` | Source/setup/resource/output-contract evidence exists only as research. |
| `blocked` | A credential, private input, provider, or setup boundary is missing. |
| `incomplete` | Setup reached a partial adapter path but did not reach the behavioral scoring surface. |
| `unsupported` | The project shape is not comparable for the scenario. |
| `not_encoded` | The benchmark does not yet cover the scenario. |
| `wrong_result` | The system ran but produced the wrong memory answer or evidence. |
| `lifecycle_fail` | Update/delete/reload/persistence behavior failed. |

## Source Artifacts

| Command or run | Artifact | Supported claim |
| --- | --- | --- |
| `cargo make real-world-memory` | `2026-06-11-measurement-coverage-audit.md` plus XY-952, XY-953, and XY-954 fixture updates | ELF fixture aggregate covers 60 jobs across 16 suites with 53 pass and 7 blocked production-ops, private-corpus, private/provider scheduler, or OpenViking context-trajectory measurement gates, including 6 passing `core_archival_memory` jobs, 1 passing `memory_summary` source-trace job, 4 passing `proactive_brief` suggestion jobs plus 1 private-corpus blocker, and 4 passing `scheduled_memory` task-readback jobs plus 1 private/provider scheduler blocker. |
| `cargo make real-world-memory-scheduled` | `tmp/real-world-memory/scheduled/report.json` and `2026-06-16-scheduled-memory-task-scoring-report.md` | The scheduled-memory fixture scores weekly project status summary, stale preference/plan audit, stale decision audit, knowledge-page refresh suggestion, and private/provider scheduler blocker scenarios with evidence refs, freshness/currentness markers, action rationale, execution trace/readback, source-mutation guards, and stale/tombstone guards; this is fixture-backed contract evidence, not hosted scheduler, ChatGPT Tasks, Pulse, notification, or provider-backed private-corpus parity. |
| `cargo make real-world-memory-summary` | `tmp/real-world-memory/memory-summary/report.json` | The memory summary fixture scores reviewable top-of-mind, background, stale, superseded, tombstoned, and derived project-profile entries with source refs, freshness metadata, rationale, and unsupported-claim flags; this is fixture-backed contract evidence, not managed-memory parity. |
| `cargo make real-world-memory-proactive-brief` | `tmp/real-world-memory/proactive-brief/report.json` and `2026-06-16-proactive-brief-scoring-report.md` | The proactive brief fixture scores daily project brief, resume-work brief, stale decision audit, stale plan/preference warning, and private-corpus refresh blocker scenarios with evidence refs, freshness/currentness markers, action rationale, and stale/tombstone guards; this is fixture-backed contract evidence, not Pulse or hosted managed-memory parity. |
| `cargo make real-world-memory-core-archival` | `tmp/real-world-memory/core-archival/report.json` | ELF core-block behavior is scored separately from archival note search for attachment, scope, provenance, stale-core detection, archival fallback, and project-decision recovery. |
| `cargo make real-world-memory-live-adapters` | `2026-06-11-measurement-coverage-audit.md` | ELF live service adapter reports 22 pass, 5 wrong_result, 2 blocked, and 11 not_encoded jobs; qmd reports 17 pass, 6 wrong_result, 2 blocked, and 15 not_encoded jobs. |
| `cargo make real-world-memory-live-adapters` | `2026-06-11-capture-write-policy-live-report.md` | ELF live capture/write-policy jobs pass for redaction, exclusions, source ids, evidence binding, and no secret leakage; qmd remains not_encoded, while agentmemory and claude-mem capture breadth are blocked until durable hook/viewer evidence exists. |
| `cargo make real-world-job-operator-ux-live-adapters` | `tmp/real-world-job/operator-ux-live-adapters/summary.json` | The narrow live operator-debug slice scores ELF as pass and qmd as wrong_result: ELF wins trace hydration, candidate-drop visibility, and selected-but-not-narrated evidence; both systems expose replay commands and repair-action guidance. |
| `ELF_BASELINE_PROJECTS=ELF,agentmemory,mem0,memsearch,claude-mem cargo make baseline-live-docker` | `2026-06-11-first-generation-oss-adapter-promotion-report.md` | mem0/OpenMemory and memsearch pass basic local baseline smokes; agentmemory remains lifecycle_fail and claude-mem remains wrong_result. |
| `cargo make real-world-first-generation-oss` | `2026-06-11-first-generation-oss-continuity-source-store-report.md` | First-generation OSS fixture slice reports 6 jobs: 4 pass, 2 blocked, full evidence/source-ref/quote coverage, and manifest scenario outcomes across win, tie, loss, not_tested, blocked, and non_goal without promoting smoke evidence into live suite passes. |
| `cargo make openmemory-ui-export-readback` | `2026-06-11-mem0-openmemory-history-ui-export-report.md` | mem0 local OSS passes preference correction history, entity-scoped personalization, local `get_all` export-style readback, and deletion audit history; OpenMemory export-helper setup emits a separate blocked artifact with `DOCKER_UNAVAILABLE_IN_BASELINE_RUNNER`, and hosted Platform export remains non-goal. |
| `ELF_GRAPHITI_ZEP_SMOKE_START=1 ELF_GRAPHITI_ZEP_SMOKE_RUN=1 cargo make smoke-graphiti-zep-docker-temporal` | `2026-06-11-temporal-history-competitor-gap-report.md` | Graphiti/Zep temporal smoke remains blocked by `provider_api_key_missing`. |
| `cargo make smoke-graphify-docker-graph-report` | `2026-06-11-graph-rag-scored-smoke-adapter-report.md` | graphify reaches tiny Docker graph/report scoring but remains wrong_result. |
| `cargo make real-world-memory-graph-rag` | `tmp/real-world-memory/graph-rag/report.json` | Representative graph/RAG fixtures produce typed non-pass reports: RAGFlow, GraphRAG, and Graphiti/Zep blocked; LightRAG incomplete with comparison blocked; graphify wrong_result; llm-wiki not_tested; gbrain blocked; private/hosted profiles non_goal. |
| `cargo make baseline-production-synthetic`, `cargo make baseline-backfill-docker`, backup/restore, Qdrant rebuild proof | `2026-06-10-production-adoption-refresh.md` | ELF has provider synthetic, stress, backfill, restore, and rebuild evidence; private-corpus proof is blocked by missing operator-owned manifest. |
| `ELF_BASELINE_PROJECTS=ELF,qmd ELF_BASELINE_PROFILE=stress cargo make baseline-live-docker` plus ELF trace-bundle and qmd CLI replay commands | `2026-06-11-elf-qmd-trace-replay-diagnostics-report.md` | Retrieval correctness remains tied, but qmd wins current immediate top-10/replay artifact ergonomics; ELF trace/admin surfaces are useful but not yet hydrated into the default stress artifact. |

## Scenario Matrix

| Scenario | ELF outcome | Evidence classes | Measured claim | Follow-up |
| --- | --- | --- | --- | --- |
| Source-of-truth rebuild and evidence-bound writes | `win` | `fixture_backed`, `live_real_world`, `live_baseline_only` | ELF has the strongest measured source-of-truth and rebuild story: Postgres is authoritative, Qdrant is rebuildable, trust-source jobs pass, and production restore/rebuild proof exists. | None |
| Work resume and coding-agent continuity | `tie` | `fixture_backed`, `live_real_world`, `live_baseline_only`, `blocked`, `not_encoded` | ELF and qmd both pass encoded live `work_resume` jobs. XY-925 selects agentmemory's next durable local path but keeps it blocked until the SDK KV/index and observation log survive a fresh process; claude-mem work_resume remains `not_encoded`, and OpenViking continuity trajectory remains `blocked`. | XY-928 |
| Project decisions and reversals | `tie` | `fixture_backed`, `live_real_world`, `research_gate`, `not_encoded` | ELF and qmd both pass encoded `project_decisions` jobs. The ELF `core_archival_memory` fixture also scores project-decision recovery through core routing plus archival rationale, but Letta-style comparison remains blocked without contained export evidence. | XY-927 |
| Retrieval quality | `tie` | `fixture_backed`, `live_real_world`, `live_baseline_only` | ELF and qmd both pass encoded live retrieval and stress/same-corpus retrieval evidence. | XY-923 |
| Retrieval quality and local debug UX | `loss` | `live_baseline_only`, `research_gate`, `wrong_result`, `not_encoded` | The XY-923 trace/replay report scores qmd stronger on immediate top-10 candidate artifacts and short CLI replay commands. ELF keeps useful service trace/admin replay surfaces, and expansion, fusion, rerank-on, and candidate-drop diagnostics remain untested. | XY-923 |
| Memory evolution and temporal history | `loss` | `fixture_backed`, `live_real_world`, `live_baseline_only`, `wrong_result`, `blocked` | ELF fixture memory evolution passes, but live ELF passes only delete/TTL and reports five wrong_result jobs where current-vs-historical state is not reconciled. The mem0 local OSS preference-correction history scenario is now measured and is also an ELF loss. | XY-905 |
| Consolidation/proposal review | `not_tested` for direct competitors; ELF self-check passes | `fixture_backed`, `live_real_world`, `research_gate`, `not_encoded` | ELF fixture consolidation passes and XY-934 adds live service-backed proposal materialization, lineage, confidence/usefulness, unsupported-claim flags, and apply/defer/discard audit evidence. Managed dreaming and Always-On Memory Agent patterns remain product references, not direct live competitors. | XY-934 |
| Knowledge page compilation | `not_tested` for direct competitors; ELF self-check passes | `fixture_backed`, `live_real_world`, `wrong_result`, `research_gate`, `blocked`, `not_encoded` | ELF fixture knowledge pages pass, and XY-935 adds a Docker-contained ELF service-native rebuild/lint/search command for the checked-in knowledge pack. The XY-929 graph/RAG representative slice still scores graphify as wrong_result and keeps GraphRAG, llm-wiki, and gbrain as blocked or not_tested references, so broad external knowledge-product comparison remains unproven. | XY-935, XY-929 |
| Operator debugging/viewer UX | `win` | `fixture_backed`, `live_real_world`, `blocked`, `not_encoded` | ELF now has a narrow live operator-debug win over qmd on trace hydration, candidate-drop visibility, and selected-but-not-narrated evidence. ELF ties qmd on replay-command availability and repair-action clarity. XY-925 adds claude-mem progressive-disclosure and retrieval-repair prompt coverage, but claude-mem viewer/operator workflows and OpenMemory UI/export remain blocked, so this is not a broad viewer-product superiority claim. | XY-926 |
| Capture/write policy and redaction | `not_tested` | `fixture_backed`, `live_real_world`, `live_baseline_only`, `blocked`, `not_encoded` | ELF live capture/write-policy self-check jobs pass for redaction, exclusions, source ids, evidence binding, and no secret leakage. qmd remains `not_encoded`; agentmemory and claude-mem hook-capture comparisons remain `blocked` until Docker-contained hook observations and write-policy/viewer readback artifacts exist, so no broad capture-hook superiority claim is allowed. | XY-933, XY-925 |
| Production ops, restore, backfill, and rebuild | `win` | `live_baseline_only`, `blocked` | ELF has the strongest measured local production-operation story: provider synthetic, stress, resumable backfill, backup/restore, and Qdrant rebuild evidence. | XY-930 |
| Private corpus and provider boundaries | `blocked` | `blocked` | Private production profile fails closed without an operator-owned manifest; provider-backed production-ops gates require explicit credentials. | XY-930 |
| Personalization and scoped preferences | `tie` | `fixture_backed`, `live_real_world`, `live_baseline_only`, `not_encoded` | ELF and qmd both pass the single encoded live personalization job. mem0 local OSS now passes entity-scoped personalization, so scoped preference behavior is a measured tie; preference correction history remains a separate ELF loss. | XY-927 |
| Context trajectory and hierarchical retrieval | `not_tested` | `fixture_backed`, `live_baseline_only`, `research_gate`, `wrong_result`, `blocked` | OpenViking reaches the pinned Docker local embedding path and now exposes expected/matched/missing evidence ids, but same-corpus evidence is still wrong_result; staged trajectory, hierarchy selection, and recursive expansion are encoded as blocked fixtures, not scored comparisons. | XY-928 |
| Core-vs-archival memory | `blocked` | `fixture_backed`, `research_gate`, `blocked`, `not_encoded` | ELF now has 6 fixture-backed `core_archival_memory` jobs that score core block attachment, scope, provenance, stale-core detection, archival fallback, and project-decision recovery separately from archival note search. Letta remains blocked or not tested until its contained export/readback artifact maps core and archival source ids. | XY-927 |
| Graph/RAG navigation and citations | `not_tested` | `smoke_only`, `research_gate`, `blocked`, `incomplete`, `wrong_result`, `not_encoded` | `cargo make real-world-memory-graph-rag` adds representative citation, graph-summary, temporal-validity, graph-report, stale-source-lint, and unsupported-claim fixtures. The slice is typed non-pass: RAGFlow, GraphRAG, and Graphiti/Zep are blocked; LightRAG is incomplete with comparison blocked; graphify is wrong_result; llm-wiki is not_tested; gbrain is blocked. Broad graph/RAG navigation and citation quality remain not_tested. | XY-929 |

## Follow-Up Queue

| Issue | Priority | State | Gap |
| --- | --- | --- | --- |
| XY-905 | P0 | Backlog | Live temporal reconciliation answer and trace contract. |
| XY-923 | P0 | Backlog | qmd trace-level replay and wrong-result diagnostics. |
| XY-924/XY-931 | P0 | Encoded local OSS history; UI/export setup blocker measured | mem0/OpenMemory local OSS history and SDK export-style readback are measured; OpenMemory UI/export has a blocked export-helper setup probe and still needs a dedicated compose/import path before any product-UX comparison. |
| XY-925 | P1 | Fixture slice encoded; runtime paths still blocked | First-generation OSS prompt coverage and typed blockers are recorded for agentmemory, memsearch, and claude-mem; durable agentmemory hooks and claude-mem viewer/operator runs still need runtime adapters. |
| XY-926/XY-935 | P1 | ELF live knowledge self-check encoded | ELF live knowledge-page scoring is encoded through a dedicated XY-935 rebuild/lint/search command; broader knowledge-page external comparisons and broad operator-debugging remain dependent on contained llm-wiki/gbrain/GraphRAG/OpenMemory/claude-mem runners. Consolidation is split to XY-934. |
| XY-934 | P1 | ELF live self-check encoded | Live consolidation proposal scoring is encoded for ELF with lineage, confidence/usefulness, unsupported-claim flags, and review-action audit; direct competitor runners remain untested or product-reference only. |
| XY-933 | P1 | Live ELF self-check encoded | Capture/write-policy redaction, exclusion, source-id, evidence-binding, and no-leak scoring for ELF; durable agentmemory/claude-mem capture-hook comparison remains blocked. |
| XY-927 | P1 | Fixture encoded; Letta export blocked | ELF core-vs-archival fixture coverage is encoded; a contained Letta export/readback adapter remains future work before win/tie/loss claims. |
| XY-928 | P1 | Encoded blocked fixtures | OpenViking context-trajectory and hierarchy benchmark is encoded but blocked until evidence-bearing same-corpus and staged artifacts exist. |
| XY-929 | P2 | Representative fixture slice encoded; live contracts still blocked or typed non-pass | Graph/RAG adapters now have representative citation/navigation/lint fixtures, but live evidence-linked output contracts are still blocked, incomplete, wrong_result, not_tested, or non_goal. |
| XY-930 | P1 | Backlog | Private-corpus and credentialed production gates after operator inputs exist. |
| XY-906 | Ops | Todo | Decodex registered-project review-config schema drift blocks Decodex loading of ELF. |

## Allowed Claims

- ELF is adoptable for bounded personal production use with caveats.
- ELF has the strongest measured source-of-truth, rebuild, restore, and backfill
  evidence among the tracked systems.
- ELF ties qmd on encoded live retrieval, work-resume, project-decisions, and
  personalization slices.
- ELF fixture-backed `core_archival_memory` coverage passes attachment, scope,
  provenance, stale-core detection, archival fallback, and project-decision recovery
  jobs separately from archival search.
- ELF has a narrow live operator-debug win over qmd for trace hydration,
  candidate-drop visibility, and selected-but-not-narrated evidence, with
  replay-command availability and repair-action clarity tied.
- ELF live capture/write-policy self-checks pass for redaction, exclusions, source
  ids, evidence binding, and no secret leakage.
- ELF has a live temporal reconciliation loss against the benchmark expectation:
  five memory-evolution jobs remain `wrong_result`.
- Most competitor strengths outside qmd retrieval are `not_tested`, `blocked`,
  `incomplete`, `smoke_only`, or `research_gate`.

## Claims Not Allowed

- Do not claim ELF broadly beats qmd.
- Do not claim qmd's trace/replay artifact win is a broad qmd-over-ELF memory-system
  or retrieval-quality win.
- Do not claim ELF beats mem0/OpenMemory on preference history, UI/export, hosted
  behavior, or graph memory. The local OSS correction-history scenario is currently
  an ELF loss, while OpenMemory UI/export is a measured setup blocker and hosted
  behavior plus graph memory remain outside measured local OSS evidence.
- Do not claim ELF broadly beats OpenMemory or claude-mem viewer UX from the narrow
  ELF/qmd operator-debug slice.
- Do not claim ELF broadly beats agentmemory or claude-mem on capture breadth; the
  current comparison is blocked for their hook/viewer capture paths.
- Do not claim ELF beats OpenViking on staged context trajectory.
- Do not claim ELF beats Letta on core-vs-archival memory.
- Do not claim graph/RAG parity from smoke-only or typed non-pass representative
  evidence.
- Do not promote `fixture_backed`, `live_baseline_only`, `smoke_only`,
  `research_gate`, `blocked`, `incomplete`, `wrong_result`, `lifecycle_fail`,
  `unsupported`, or `not_encoded` states into a generic pass/fail score.
