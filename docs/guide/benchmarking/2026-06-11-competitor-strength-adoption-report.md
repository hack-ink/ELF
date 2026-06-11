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
`2026-06-11-mem0-openmemory-history-ui-export-report.md`, and
`2026-06-10-production-adoption-refresh.md`.
Depends on: `docs/spec/real_world_agent_memory_benchmark_v1.md` and the current
external adapter manifest.
Outputs: Adoption decision, evidence-class boundaries, scenario matrix, follow-up
optimization queue, and the machine-readable companion file
`docs/research/2026-06-11-competitor-strength-adoption-report.json`.

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
  memory, and graph/RAG navigation remain unproven. XY-928 encodes OpenViking staged
  trajectory, hierarchy selection, and recursive/context expansion as blocked fixtures
  behind same-corpus evidence output and missing staged artifacts. mem0 local OSS preference history
  is measured separately and is an ELF loss on the current correction history
  scenario. The XY-923 follow-up also scores qmd's immediate top-10/replay artifact
  ergonomics as stronger than ELF's default stress report, while expansion, fusion,
  and rerank remain untested. XY-932 adds a narrow live operator-debug slice where
  ELF beats qmd on trace hydration and candidate-drop visibility, but OpenMemory
  UI/export and claude-mem viewer workflows remain blocked or not encoded. XY-933
  adds an ELF live capture/write-policy self-check, but agentmemory capture breadth
  is blocked by mocked/in-memory storage and claude-mem hook/viewer capture remains
  untested.

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
| `unsupported` | The project shape is not comparable for the scenario. |
| `not_encoded` | The benchmark does not yet cover the scenario. |
| `wrong_result` | The system ran but produced the wrong memory answer or evidence. |
| `lifecycle_fail` | Update/delete/reload/persistence behavior failed. |

## Source Artifacts

| Command or run | Artifact | Supported claim |
| --- | --- | --- |
| `cargo make real-world-memory` | `2026-06-11-measurement-coverage-audit.md` | ELF fixture aggregate covers 43 jobs across 12 suites with 38 pass and 5 blocked production-ops or OpenViking context-trajectory measurement gates. |
| `cargo make real-world-memory-live-adapters` | `2026-06-11-measurement-coverage-audit.md` | ELF live service adapter reports 22 pass, 5 wrong_result, 2 blocked, and 11 not_encoded jobs; qmd reports 17 pass, 6 wrong_result, 2 blocked, and 15 not_encoded jobs. |
| `cargo make real-world-memory-live-adapters` | `2026-06-11-capture-write-policy-live-report.md` | ELF live capture/write-policy jobs pass for redaction, exclusions, source ids, evidence binding, and no secret leakage; qmd remains not_encoded, agentmemory is blocked, and claude-mem is untested for capture breadth. |
| `cargo make real-world-job-operator-ux-live-adapters` | `tmp/real-world-job/operator-ux-live-adapters/summary.json` | The narrow live operator-debug slice scores ELF as pass and qmd as wrong_result: ELF wins trace hydration, candidate-drop visibility, and selected-but-not-narrated evidence; both systems expose replay commands and repair-action guidance. |
| `ELF_BASELINE_PROJECTS=ELF,agentmemory,mem0,memsearch,claude-mem cargo make baseline-live-docker` | `2026-06-11-first-generation-oss-adapter-promotion-report.md` | mem0/OpenMemory and memsearch pass basic local baseline smokes; agentmemory remains lifecycle_fail and claude-mem remains wrong_result. |
| `cargo make openmemory-ui-export-readback` | `2026-06-11-mem0-openmemory-history-ui-export-report.md` | mem0 local OSS passes preference correction history, entity-scoped personalization, local `get_all` export-style readback, and deletion audit history; OpenMemory export-helper setup emits a separate blocked artifact with `DOCKER_UNAVAILABLE_IN_BASELINE_RUNNER`, and hosted Platform export remains non-goal. |
| `ELF_GRAPHITI_ZEP_SMOKE_START=1 ELF_GRAPHITI_ZEP_SMOKE_RUN=1 cargo make graphiti-zep-docker-temporal-smoke` | `2026-06-11-temporal-history-competitor-gap-report.md` | Graphiti/Zep temporal smoke remains blocked by `provider_api_key_missing`. |
| `cargo make graphify-docker-graph-report-smoke` | `2026-06-11-graph-rag-scored-smoke-adapter-report.md` | graphify reaches tiny Docker graph/report scoring but remains wrong_result. |
| `cargo make baseline-production-synthetic`, `cargo make baseline-backfill-docker`, backup/restore, Qdrant rebuild proof | `2026-06-10-production-adoption-refresh.md` | ELF has provider synthetic, stress, backfill, restore, and rebuild evidence; private-corpus proof is blocked by missing operator-owned manifest. |
| `ELF_BASELINE_PROJECTS=ELF,qmd ELF_BASELINE_PROFILE=stress cargo make baseline-live-docker` plus ELF trace-bundle and qmd CLI replay commands | `2026-06-11-elf-qmd-trace-replay-diagnostics-report.md` | Retrieval correctness remains tied, but qmd wins current immediate top-10/replay artifact ergonomics; ELF trace/admin surfaces are useful but not yet hydrated into the default stress artifact. |

## Scenario Matrix

| Scenario | ELF outcome | Evidence classes | Measured claim | Follow-up |
| --- | --- | --- | --- | --- |
| Source-of-truth rebuild and evidence-bound writes | `win` | `fixture_backed`, `live_real_world`, `live_baseline_only` | ELF has the strongest measured source-of-truth and rebuild story: Postgres is authoritative, Qdrant is rebuildable, trust-source jobs pass, and production restore/rebuild proof exists. | None |
| Work resume and coding-agent continuity | `tie` | `fixture_backed`, `live_real_world`, `live_baseline_only`, `blocked`, `not_encoded` | ELF and qmd both pass encoded live `work_resume` jobs; agentmemory, claude-mem, and OpenViking continuity strengths remain blocked or not encoded. | XY-925, XY-928 |
| Project decisions and reversals | `tie` | `fixture_backed`, `live_real_world`, `research_gate`, `not_encoded` | ELF and qmd both pass encoded `project_decisions` jobs; Letta-style core/archival decision memory is not tested. | XY-927 |
| Retrieval quality | `tie` | `fixture_backed`, `live_real_world`, `live_baseline_only` | ELF and qmd both pass encoded live retrieval and stress/same-corpus retrieval evidence. | XY-923 |
| Retrieval quality and local debug UX | `loss` | `live_baseline_only`, `research_gate`, `wrong_result`, `not_encoded` | The XY-923 trace/replay report scores qmd stronger on immediate top-10 candidate artifacts and short CLI replay commands. ELF keeps useful service trace/admin replay surfaces, and expansion, fusion, rerank-on, and candidate-drop diagnostics remain untested. | XY-923 |
| Memory evolution and temporal history | `loss` | `fixture_backed`, `live_real_world`, `live_baseline_only`, `wrong_result`, `blocked` | ELF fixture memory evolution passes, but live ELF passes only delete/TTL and reports five wrong_result jobs where current-vs-historical state is not reconciled. The mem0 local OSS preference-correction history scenario is now measured and is also an ELF loss. | XY-905 |
| Consolidation/proposal review | `not_tested` | `fixture_backed`, `not_encoded` | ELF fixture consolidation passes, but live consolidation proposal generation and review-action scoring are not encoded. | XY-926 |
| Knowledge page compilation | `not_tested` | `fixture_backed`, `live_real_world`, `wrong_result`, `research_gate`, `not_encoded` | ELF fixture knowledge pages pass, but live knowledge compilation is not encoded; graphify reaches a tiny scored smoke and remains wrong_result. | XY-926, XY-929 |
| Operator debugging/viewer UX | `win` | `fixture_backed`, `live_real_world`, `blocked`, `not_encoded` | ELF now has a narrow live operator-debug win over qmd on trace hydration, candidate-drop visibility, and selected-but-not-narrated evidence. ELF ties qmd on replay-command availability and repair-action clarity. OpenMemory UI/export remains blocked and claude-mem UI remains not encoded, so this is not a broad viewer-product superiority claim. | XY-926 |
| Capture/write policy and redaction | `not_tested` | `fixture_backed`, `live_real_world`, `live_baseline_only`, `blocked`, `not_encoded` | ELF live capture/write-policy self-check jobs pass for redaction, exclusions, source ids, evidence binding, and no secret leakage. qmd remains `not_encoded`; agentmemory comparison is `blocked`; claude-mem capture breadth is `not_encoded`, so no broad capture-hook superiority claim is allowed. | XY-933, XY-925 |
| Production ops, restore, backfill, and rebuild | `win` | `live_baseline_only`, `blocked` | ELF has the strongest measured local production-operation story: provider synthetic, stress, resumable backfill, backup/restore, and Qdrant rebuild evidence. | XY-930 |
| Private corpus and provider boundaries | `blocked` | `blocked` | Private production profile fails closed without an operator-owned manifest; provider-backed production-ops gates require explicit credentials. | XY-930 |
| Personalization and scoped preferences | `tie` | `fixture_backed`, `live_real_world`, `live_baseline_only`, `not_encoded` | ELF and qmd both pass the single encoded live personalization job. mem0 local OSS now passes entity-scoped personalization, so scoped preference behavior is a measured tie; preference correction history remains a separate ELF loss. | XY-927 |
| Context trajectory and hierarchical retrieval | `not_tested` | `fixture_backed`, `live_baseline_only`, `research_gate`, `wrong_result`, `blocked` | OpenViking reaches the pinned Docker local embedding path and now exposes expected/matched/missing evidence ids, but same-corpus evidence is still wrong_result; staged trajectory, hierarchy selection, and recursive expansion are encoded as blocked fixtures, not scored comparisons. | XY-928 |
| Core-vs-archival memory | `not_tested` | `research_gate`, `not_encoded` | ELF has core block semantics in the service contract, but comparable core-vs-archival jobs and a contained Letta export path are not encoded. | XY-927 |
| Graph/RAG navigation and citations | `not_tested` | `smoke_only`, `research_gate`, `blocked`, `wrong_result`, `not_encoded` | Graph/RAG smokes produce scored or typed non-pass adapter reports where possible, but broad graph/RAG navigation and citation quality are not tested. | XY-929 |

## Follow-Up Queue

| Issue | Priority | State | Gap |
| --- | --- | --- | --- |
| XY-905 | P0 | Backlog | Live temporal reconciliation answer and trace contract. |
| XY-923 | P0 | Backlog | qmd trace-level replay and wrong-result diagnostics. |
| XY-924/XY-931 | P0 | Encoded local OSS history; UI/export setup blocker measured | mem0/OpenMemory local OSS history and SDK export-style readback are measured; OpenMemory UI/export has a blocked export-helper setup probe and still needs a dedicated compose/import path before any product-UX comparison. |
| XY-925 | P1 | Backlog | First-generation OSS continuity and source-store adapters. |
| XY-926 | P1 | Backlog | Live consolidation and knowledge-page suites; broad operator-debugging remains dependent on OpenMemory and claude-mem UI runners. |
| XY-933 | P1 | Live ELF self-check encoded | Capture/write-policy redaction, exclusion, source-id, evidence-binding, and no-leak scoring for ELF; durable agentmemory/claude-mem capture-hook comparison remains blocked or untested. |
| XY-927 | P1 | Backlog | Letta-style core-vs-archival memory comparison. |
| XY-928 | P1 | Encoded blocked fixtures | OpenViking context-trajectory and hierarchy benchmark is encoded but blocked until evidence-bearing same-corpus and staged artifacts exist. |
| XY-929 | P2 | Backlog | Graph/RAG adapters beyond scored smokes. |
| XY-930 | P1 | Backlog | Private-corpus and credentialed production gates after operator inputs exist. |
| XY-906 | Ops | Todo | Decodex registered-project review-config schema drift blocks Decodex loading of ELF. |

## Allowed Claims

- ELF is adoptable for bounded personal production use with caveats.
- ELF has the strongest measured source-of-truth, rebuild, restore, and backfill
  evidence among the tracked systems.
- ELF ties qmd on encoded live retrieval, work-resume, project-decisions, and
  personalization slices.
- ELF has a narrow live operator-debug win over qmd for trace hydration,
  candidate-drop visibility, and selected-but-not-narrated evidence, with
  replay-command availability and repair-action clarity tied.
- ELF live capture/write-policy self-checks pass for redaction, exclusions, source
  ids, evidence binding, and no secret leakage.
- ELF has a live temporal reconciliation loss against the benchmark expectation:
  five memory-evolution jobs remain `wrong_result`.
- Most competitor strengths outside qmd retrieval are `not_tested`, `blocked`,
  `smoke_only`, or `research_gate`.

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
  current comparison is blocked or untested for their hook/viewer capture paths.
- Do not claim ELF beats OpenViking on staged context trajectory.
- Do not claim ELF beats Letta on core-vs-archival memory.
- Do not claim graph/RAG parity from smoke-only evidence.
- Do not promote `fixture_backed`, `live_baseline_only`, `smoke_only`,
  `research_gate`, `blocked`, `wrong_result`, `lifecycle_fail`, `unsupported`, or
  `not_encoded` states into a generic pass/fail score.
