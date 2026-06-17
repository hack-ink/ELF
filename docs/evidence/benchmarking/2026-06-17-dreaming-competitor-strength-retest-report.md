# Dreaming Competitor-Strength Retest Report - June 17, 2026

Goal: Close out the XY-955 Dreaming-readiness benchmark program pass with a
baseline-vs-current competitor-strength retest and optimization queue.
Read this when: You need the final stage-ledger closeout after XY-905, XY-934,
XY-952, XY-953, and XY-954, or need to know which remaining losses and blockers are
ready for follow-up issue work.
Inputs:
`apps/elf-eval/fixtures/report_snapshots/2026-06-17-dreaming-competitor-strength-retest-report.json`,
`apps/elf-eval/fixtures/report_snapshots/2026-06-16-dreaming-readiness-stage-ledger.json`,
`apps/elf-eval/fixtures/report_snapshots/2026-06-11-competitor-strength-adoption-report.json`,
the June 16 stage reports, and the fresh `tmp/real-world-memory/` retest outputs.
Outputs: Scenario-level improved/regressed/unchanged/blocked/not-tested judgments,
claim boundaries, and the next optimization queue.

## Executive Judgment

ELF is locally and partially stronger after the Dreaming-readiness stages. It is not
broadly superior to the tracked competitors.

The public/local retest supports these narrow improvements:

- Live ELF `memory_evolution` moved from `pass=1`, `wrong_result=5` in the XY-951
  baseline to `pass=6`, `wrong_result=0` in the XY-905 report and the fresh partial
  ELF live adapter output.
- Live ELF consolidation self-checks now pass for service-backed proposal
  materialization, source lineage, confidence/usefulness, unsupported-claim flags,
  review actions, and zero source mutations.
- Fixture-backed memory summary, proactive brief, and scheduled-memory task scoring
  are now encoded and passing except for their explicit private/provider blockers.

The broader competitor-strength outcome is unchanged:

- qmd debug ergonomics remain a measured ELF loss from the existing trace/replay
  report. The fresh qmd full-suite live report is typed non-pass, but that does not
  retest or erase qmd's top-k/replay artifact advantage.
- mem0/OpenMemory preference-history and export-style local OSS readback remain
  separate measured strengths; OpenMemory UI/export and hosted Platform behavior are
  not proven by this retest.
- Letta core/archive, OpenViking trajectory/hierarchy, Graphiti/Zep temporal graph,
  and broad graph/RAG citation/navigation quality remain blocked, incomplete,
  wrong-result, or not-tested.
- Private-corpus and credentialed provider gates remain tied to XY-930.

No scenario regressed in the checked-in local/public retest evidence. The remaining
work is issue-shaped only for measured losses or typed blockers.

## Commands

| Command | Status | Artifact | Result |
| --- | --- | --- | --- |
| `cargo make real-world-memory` | `pass` | `tmp/real-world-memory/real-world-memory-report.json` | 60 jobs, 53 pass, 0 wrong_result, 7 blocked, evidence/source-ref/quote coverage 1.000. |
| `cargo make real-world-memory-graph-rag` | `pass` | `tmp/real-world-memory/graph-rag/report.json` | 5 jobs, 0 pass, 1 wrong_result, 1 incomplete, 3 blocked. This is typed non-pass graph/RAG evidence. |
| `cargo make real-world-first-generation-oss` | `pass` | `tmp/real-world-memory/first-generation-oss/report.json` | 6 jobs, 4 pass, 2 blocked, evidence coverage 1.000. |
| `cargo make real-world-memory-live-adapters` | `pass` | `tmp/real-world-memory/live-adapters/summary.json` | ELF live: 66 jobs, 40 pass, 0 wrong_result, 7 blocked, 19 not_encoded. qmd live: 66 jobs, 17 pass, 13 wrong_result, 7 blocked, 29 not_encoded. |

The full live-adapter command now has fresh ELF and qmd scored reports. The qmd
full-suite non-pass result is not a regression of qmd debug ergonomics and is not a
broad ELF-over-qmd win.

## Stage Closeout

| Stage | Baseline | Current | Judgment | Boundary |
| --- | --- | --- | --- | --- |
| Current-vs-historical correctness | `pass=1`, `wrong_result=5` | `pass=6`, `wrong_result=0` | `improved` | Encoded ELF live `memory_evolution` only; no Graphiti/Zep, mem0/OpenMemory, Letta, private-corpus, or broad qmd claim. |
| Preference evolution | `wrong_result=1` | `pass=1`, `wrong_result=0` | `improved` | ELF current-vs-historical preference case improved; mem0/OpenMemory history remains separately stronger on the local OSS history surface. |
| Deletion, TTL, and tombstones | `pass=1` | `pass=1` | `unchanged` | Single encoded tombstone/TTL job remains passing; broader update/delete/recreate history is still follow-up work. |
| Reviewable consolidation | `pass=4`, `not_tested=1`, `not_encoded=1` | `pass=4`, `not_tested=0`, `not_encoded=0` | `improved` | ELF live self-check evidence only; direct competitor consolidation runners remain untested or product-reference only. |
| Memory summary/top-of-mind | `pass=8`, `not_tested=1`, `not_encoded=1` | `pass=9`, `not_tested=0`, `not_encoded=0` | `improved` | Fixture-backed `elf.memory_summary/v1` source-trace contract evidence only. |
| Proactive brief readiness | `pass=0`, `not_tested=1`, `not_encoded=1` | `pass=4`, `blocked=1` | `improved` | Fixture-backed proactive brief scoring only; private-corpus refresh stays blocked under XY-930 and Pulse parity is not proven. |
| Scheduled memory task readiness | `pass=0`, `blocked=1` | `pass=4`, `blocked=1` | `improved` | Fixture-backed scheduled task readback only; hosted scheduler, notification, provider-backed private-corpus, and silent-mutation parity are not proven. |
| Final competitor retest status | `pass=22`, `wrong_result=5`, `blocked=2`, `not_tested=11`, `not_encoded=11` | ELF live: `pass=40`, `wrong_result=0`, `blocked=7`, `not_encoded=19`; qmd live: `pass=17`, `wrong_result=13`, `blocked=7`, `not_encoded=29`; graph/RAG typed non-pass; first-generation OSS `pass=4`, `blocked=2` | `unchanged` | ELF live improvement and qmd full-suite non-pass do not remove qmd debug ergonomics, private/provider, OpenViking, Letta, or graph/RAG blockers. |

## Scenario Retest Matrix

| Scenario | Baseline outcome | Current outcome | Status | Follow-up |
| --- | --- | --- | --- | --- |
| qmd debug ergonomics | `loss` | `unchanged` | `pass` for fresh qmd full-suite materialization; debug ergonomics still a measured ELF loss | XY-923 |
| mem0/OpenMemory preference/history/export | ELF loss on correction history, tie on scoped personalization, UI/export blocked | `unchanged` | `blocked` for UI/export and private/provider inputs | XY-930 plus dedicated UI/export runner work |
| Letta core/archive | `blocked` | `unchanged` | `blocked` | Proposed Letta core/archive adapter brief |
| Graphiti/Zep temporal graph validity | `blocked` | `unchanged` | `blocked` | Graph/RAG adapter follow-up with explicit provider setup |
| OpenViking trajectory/hierarchy | `blocked` | `unchanged` | `blocked` | XY-928 |
| GraphRAG/LightRAG/RAGFlow/llm-wiki/gbrain/graphify citation/navigation/knowledge surfaces | `not_tested` | `unchanged` | typed non-pass: blocked, incomplete, wrong_result, not_tested, or non_goal | XY-929 |
| Private/provider production gates | `blocked` | `unchanged` | `blocked` | XY-930 |

## Optimization Queue

| Priority | Issue | Status | Brief |
| --- | --- | --- | --- |
| P0 | XY-923 | Existing | Re-run qmd trace/replay diagnostics with comparable immediate top-k/replay, expansion, fusion, rerank, and candidate-drop artifacts; preserve qmd's debug ergonomics edge unless ELF produces comparable artifacts. |
| P1 | XY-930 | Existing | Run private-corpus and credentialed provider gates only after operator-owned manifest and explicit provider setup exist; otherwise keep typed blockers. |
| P1 | XY-928 | Existing | Materialize OpenViking staged trajectory, hierarchy selection, and recursive expansion evidence before claiming ELF ties or beats those strengths. |
| P1 | Letta core/archive adapter | Proposed | Add a contained Letta core/archive export-readback adapter that emits source ids for core blocks and archival memories. Non-goals: ELF product changes and broad Letta claims. |
| P2 | XY-929 | Existing | Promote Graph/RAG citation, navigation, stale-source lint, and knowledge-surface cases only when adapters emit comparable evidence-linked outputs. |
| P2 | Service-native Dreaming outputs | Proposed | Move fixture-backed memory summary, proactive brief, and scheduled task contracts into service-native readback/materialization with source-ref, freshness, rationale, trace, and no-source-mutation gates. |

## Follow-Up Issue Briefs

These are Decodex-ready follow-up shapes for the remaining measured losses or typed
blockers. Existing Linear issues should be linked rather than duplicated.

| Issue | State | Brief |
| --- | --- | --- |
| XY-923 | Existing | Re-run qmd trace/replay diagnostics with comparable immediate top-k, replay, expansion, fusion, rerank, and candidate-drop artifacts. Non-goal: do not reinterpret qmd full-suite wrong_result counts as a regression of qmd debug ergonomics. Validation: a scored qmd/ELF debug ergonomics artifact with typed outcomes preserved. |
| XY-930 | Existing | Run private-corpus and credentialed provider gates only after operator-owned manifest and explicit provider setup exist. Non-goal: do not infer credentials or promote synthetic/provider smoke evidence into private-corpus pass evidence. Validation: a public-safe report that states whether the private/provider caveats are removed or still blocked. |
| XY-928 | Existing | Materialize OpenViking same-corpus evidence ids and staged trajectory outputs before scoring hierarchy or recursive retrieval. Non-goal: do not claim ELF ties or beats OpenViking from fixture-only blocked rows. Validation: scored context-trajectory reports with typed pass, wrong_result, blocked, or incomplete outcomes. |
| XY-929 | Existing | Promote graph/RAG citation, navigation, stale-source lint, and knowledge-surface cases only when adapters emit comparable evidence-linked outputs. Non-goal: do not convert research gates, tiny smokes, blocked setup, or graphify wrong_result into graph/RAG parity evidence. Validation: representative graph/RAG reports with typed non-pass states preserved. |
| Letta core/archive adapter | Proposed | Create a Docker-contained Letta export/readback adapter over benchmark-owned data and score only mapped core/archive evidence. Non-goal: no ELF product change or broad Letta claim before comparable evidence exists. Validation: a scored artifact containing Letta core block JSON, archival search/readback JSON, source ids, and typed outcomes. |
| Service-native Dreaming outputs | Proposed | Move memory summary, proactive brief, and scheduled task outputs into service-native materialization with source refs, freshness, rationale, trace, and no-source-mutation gates. Non-goal: no polished hosted scheduler, Pulse clone, notification product, or private/provider path in this follow-up. Validation: service-native scored reports that fail stale, tombstoned, unsupported, or untraced current claims. |

## Claim Boundaries

Allowed:

- ELF is locally and partially stronger after the Dreaming stages on encoded temporal
  reconciliation, reviewable consolidation self-checks, fixture-backed memory
  summary, proactive brief, and scheduled-memory task scoring.
- The public/local aggregate fixture retest remains 53 pass, 0 wrong_result, and 7
  typed blocked jobs across 60 jobs.
- The representative graph/RAG slice remains typed non-pass.
- Private/provider gates remain blocked under XY-930.

Not allowed:

- Do not claim broad ELF-over-qmd superiority.
- Do not claim ELF beats managed Dreaming, Pulse, ChatGPT Tasks, mem0/OpenMemory,
  Letta, OpenViking, Graphiti/Zep, or graph/RAG systems from fixture-only, partial
  live, blocked, or smoke-only evidence.
- Do not collapse scenario-level outcomes into a leaderboard.
- Do not treat qmd full-suite wrong_result counts as a regression of qmd debug
  ergonomics.
