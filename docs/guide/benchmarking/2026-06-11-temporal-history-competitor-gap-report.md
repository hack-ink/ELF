# Temporal/History Competitor Gap Report - June 11, 2026

Goal: Turn the latest live measurements into a clear competitor-gap report and
future optimization direction for ELF without implementing optimization changes here.
Read this when: You need to decide whether ELF currently wins, ties, loses, or has
no comparable claim against qmd, mem0/OpenMemory, Graphiti/Zep, Letta, and adjacent
agent-memory projects on temporal history, lifecycle, and real-world memory use.
Inputs: Fresh local runs of Graphiti/Zep temporal smoke, ELF+mem0 live baseline,
fixture memory evolution, and ELF/qmd live real-world adapters on commit
`d6d9051`.
Outputs: Evidence-class boundaries, scenario judgments, claim limits, and a
prioritized benchmark-driven optimization plan.

## Executive Judgment

The overall goal is not complete. ELF does not yet have complete, comparable
benchmark wins across all tracked memory projects and all user-important memory
scenarios.

Update after XY-924: mem0/OpenMemory local OSS history and local SDK export-style
readback are now measured in
`2026-06-11-mem0-openmemory-history-ui-export-report.md`. That report records mem0
passes for preference correction history, entity-scoped personalization, deletion
audit history, and local `get_all` readback, while keeping OpenMemory UI/export
blocked and hosted Platform export plus optional graph memory as local-lane
non-goals.

The current evidence supports a narrower judgment:

- ELF remains a strong personal-production foundation because its core source of
  truth, typed evidence, rebuild/backfill/restore story, and fixture benchmark
  coverage are much more disciplined than most competitors.
- ELF now ties or beats mem0 only on the fresh basic local lifecycle smoke shape:
  the combined Docker run passed `12/12` checks across ELF and mem0. This does not
  measure OpenMemory UI, hosted behavior, entity history quality, optional graph
  memory, or real-world temporal jobs.
- ELF narrowly beats qmd on the fresh live memory-evolution slice because ELF passes
  the delete/TTL tombstone job that qmd fails, and ELF retrieves all required
  memory-evolution evidence. This is still not a production-quality temporal memory
  win because ELF fails five current-vs-historical jobs.
- Graphiti/Zep remains the strongest temporal-validity design reference, but the
  local live smoke is typed `blocked` because no explicit provider API key was
  configured. No ELF-over-Graphiti/Zep claim is allowed.
- Letta remains a core-vs-archival memory design reference. There is no contained
  comparable live benchmark here, so no win, tie, or loss claim is allowed.

The highest-value ELF direction is temporal reconciliation and lifecycle readback,
not more generic retrieval. In the failing temporal jobs ELF usually finds the
evidence but does not turn current, historical, superseded, and deleted facts into a
clear answer and trace.

## Fresh Runs

| Command | Result | Runtime | Main artifact |
| --- | --- | ---: | --- |
| `ELF_GRAPHITI_ZEP_SMOKE_START=1 ELF_GRAPHITI_ZEP_SMOKE_RUN=1 cargo make graphiti-zep-docker-temporal-smoke` | typed blocked | 3.5 seconds | `tmp/real-world-memory/graphiti-zep-smoke/summary.json` |
| `ELF_BASELINE_PROJECTS=ELF,mem0 cargo make baseline-live-docker` | pass | 50.14 seconds | `tmp/live-baseline/live-baseline-report.json` |
| `cargo make real-world-memory-evolution` | pass | 59.65 seconds | `tmp/real-world-memory/evolution-report.json` |
| `cargo make real-world-memory-live-adapters` | pass | 166.61 seconds | `tmp/real-world-memory/live-adapters/` |

The Graphiti/Zep command did not use a hosted Zep service or unrecorded credentials.
It recorded a typed blocker: `provider_api_key_missing`.

The ELF+mem0 baseline loaded the repository `.env` from the main checkout so the
container had the configured embedding environment. The report artifact still records
the local smoke embedding mode for this baseline path, so do not cite this run as a
4096-dimensional production-embedding quality test.

## Evidence-Class Boundary

| Evidence class | What it proves | What it does not prove |
| --- | --- | --- |
| Fixture memory-evolution pass | The benchmark contract can score current facts, historical facts, conflicts, update rationales, and history readback. | Live ELF or competitor runtime quality. |
| ELF/qmd live real-world adapters | Comparable live behavior for encoded suites in the checked-in runner. | Full memory-system superiority or unencoded suites. |
| ELF+mem0 live baseline | Basic Docker local same-corpus, update, delete, and reload lifecycle smoke. | OpenMemory UI, hosted behavior, real-world jobs, temporal history quality, or graph memory. |
| Graphiti/Zep typed blocker | The adapter has a Docker-local temporal smoke contract and typed provider boundary. | Live Graphiti/Zep search quality or ELF superiority over Graphiti/Zep. |
| Letta research-only state | Core-vs-archival memory is a relevant product pattern for ELF to borrow. | Comparable live results. |

## Basic Local Lifecycle: ELF And mem0

The fresh `ELF,mem0` live-baseline run passed.

| Project | Status | Checks | Runtime | What passed |
| --- | --- | ---: | ---: | --- |
| ELF | pass | `8/8` | 11 seconds | resumable backfill, same-corpus retrieval, async worker indexing, update, delete, cold-start reload, concurrent writes, resource envelope |
| mem0 | pass | `4/4` | 36 seconds | same-corpus retrieval, update, delete, cold-start reload |

This updates the older mem0 local-baseline picture. For the basic Docker local
lifecycle smoke, mem0 should no longer be described as currently failing.

It remains a limited comparison. ELF's smoke covers more local operational checks,
while mem0's strongest product claims are elsewhere: entity-scoped memory history,
OpenMemory inspection UX, hosted ecosystem behavior, and optional graph memory. Those
are not measured by this run.

## Live Temporal Memory: ELF And qmd

The fixture memory-evolution suite passed `5/5` with mean score `1.000`, expected
evidence `11/11`, conflict detection `5`, and update rationale count `5`.

The fresh live adapters still fail the real temporal-history behavior.

| Adapter | Jobs | Pass | Wrong-result jobs | Mean score | Expected evidence recall | Evidence coverage |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| ELF live service adapter | `38` | `18` | `5` | `0.525` | `41/77` | `48/84` |
| qmd live CLI adapter | `38` | `17` | `6` | `0.486` | `38/77` | `45/84` |

For the `memory_evolution` suite:

| Adapter | Encoded jobs | Job statuses | Score mean | Evidence recall | Diagnosis |
| --- | ---: | --- | ---: | ---: | --- |
| ELF live service adapter | `6` | `1` pass, `5` wrong_result | `0.492` | `1.000` | Finds the evidence, but does not narrate current-vs-historical conflict and lifecycle state. |
| qmd live CLI adapter | `6` | `0` pass, `6` wrong_result | `0.325` | `0.769` | Same lifecycle gap, plus missed evidence including the delete tombstone. |

### Job-Level Pattern

| Job | ELF | qmd | What the result means |
| --- | --- | --- | --- |
| `memory-evolution-benchmark-verdict-001` | wrong_result, `0.40`, evidence `3/3` | wrong_result, `0.15`, evidence `2/3` | ELF found current verdict, caveat, and rationale but did not represent the superseded verdict as historical. |
| `memory-evolution-deploy-method-001` | wrong_result, `0.40`, evidence `2/2` | wrong_result, `0.40`, evidence `2/2` | Both found current runbook and rationale, but neither preserved the old quickstart path as historical. |
| `memory-evolution-issue-state-001` | wrong_result, `0.40`, evidence `2/2` | wrong_result, `0.40`, evidence `2/2` | Both found current done state and rationale, but neither surfaced the earlier blocked state. |
| `memory-evolution-preference-001` | wrong_result, `0.40`, evidence `2/2` | wrong_result, `0.15`, evidence `1/2` | ELF found current preference and rationale, but did not preserve old preference history. |
| `memory-evolution-relation-temporal-001` | wrong_result, `0.35`, evidence `2/2` | wrong_result, `0.35`, evidence `2/2` | Both found current and old owners, but did not emit scored temporal-validity explanation. |
| `memory-evolution-delete-ttl-001` | pass, `1.00`, evidence `2/2` | wrong_result, `0.50`, evidence `1/2` | ELF found tombstone and current plan. qmd missed the tombstone. |

The key ELF failure is not retrieval. The five wrong-result jobs all have evidence
grounding `1.0`, trap avoidance `1.0`, answer correctness `0.0`, and lifecycle
behavior `0.0`. ELF needs to reconcile and explain lifecycle state, not merely return
the right snippets.

## Competitor Strengths And Current ELF Position

| Scenario | Competitor/reference strength | Current evidence | ELF position |
| --- | --- | --- | --- |
| Basic local lifecycle | mem0 update/delete/reload | Fresh Docker baseline: ELF `8/8`, mem0 `4/4`, combined `12/12` | ELF ties or exceeds the encoded smoke surface, but does not beat OpenMemory UI/history/hosted claims. |
| Retrieval/debug | qmd transparent CLI, expansion/fusion/rerank/replay ergonomics | ELF/qmd live adapters pass retrieval suites; previous qmd debug profile exists | ELF is not clearly stronger. qmd remains the debug-UX bar. |
| Current-vs-historical memory | Graphiti/Zep temporal validity; mem0 history surfaces | ELF/qmd live memory-evolution wrong_result; Graphiti/Zep blocked; mem0 real-world history not encoded | ELF has a measured gap. It only narrowly beats qmd's current run. |
| Delete/tombstone lifecycle | ELF production ops and qmd local replay | ELF passes delete/TTL job; qmd misses tombstone | ELF has a narrow measured win over qmd on this job. |
| Entity preference history | mem0/OpenMemory | Only basic mem0 lifecycle smoke passed | Not comparable. Need mem0/OpenMemory history and UI/export benchmark. |
| Core-vs-archival memory | Letta core memory blocks versus archival memory | Research-only, no contained live output | Not comparable. Borrow design only. |
| Context trajectory | OpenViking staged context and hierarchy | Existing adapter remains not encoded or wrong_result for trajectory | Not comparable. Need staged trajectory benchmark. |
| Capture and continuity | agentmemory, claude-mem hooks/viewers | Existing adapters are baseline-only and undermeasured | Not comparable. Need capture/write-policy and work-resume adapters. |
| Knowledge pages and graph/RAG navigation | llm-wiki, gbrain, graphify, RAGFlow, LightRAG, GraphRAG | llm-wiki/gbrain/GraphRAG/RAGFlow/LightRAG remain research-gate or blocked; graphify has a tiny scored `wrong_result` smoke | Not comparable for graph/RAG parity. Need larger Docker-contained evidence-linked adapters. |
| Production operation discipline | ELF backfill, restore, typed gates | Existing production adoption reports plus current benchmark discipline | ELF has the strongest measured local production-operation story, with private/provider gates still typed blocked. |

## What ELF Should Borrow

| Source | Best idea to absorb | Benchmark gate before any claim |
| --- | --- | --- |
| Graphiti/Zep | Validity windows, `valid_at`/`invalid_at`, current/historical/future fact separation, temporal relation provenance | Provider-backed Docker temporal smoke must map current, historical, and rationale facts to scored evidence ids. |
| mem0/OpenMemory | Entity-scoped memory history, user-visible lifecycle inspection, update/delete ergonomics | mem0/OpenMemory adapter must score preference history, correction, deletion, and UI/export readback. |
| Letta | Always-loaded core memory blocks separated from archival search | Add core-vs-archival jobs for attachment scope, provenance, fallback, and stale-core avoidance. |
| qmd | Local replay, candidate inspection, expansion/fusion/rerank debug knobs | ELF trace artifacts must show candidate generation, rerank, dropped evidence, conflict candidates, and replay commands. |
| OpenViking | Staged context trajectory and hierarchy | Encode trajectory jobs after evidence-bearing same-corpus output passes. |
| agentmemory and claude-mem | Capture breadth, continuity hooks, and viewer comfort | Live capture/write-policy benchmark must prove redaction, exclusion, source ids, and no secret leakage. |
| memsearch | User-inspectable canonical files and rebuild clarity | Source-of-truth/reindex benchmark must prove update/delete/reload without making derived vectors authoritative. |
| llm-wiki, gbrain, graphify, GraphRAG | Cited knowledge pages, timelines, graph reports, rebuild/lint loops | Knowledge-page rebuild/lint jobs must catch unsupported claims and stale sections. |

## Optimization Direction

These are future optimization directions, not implemented changes in this report.

### P0 - Temporal Reconciliation Contract

ELF should add an answer and trace contract for current-vs-historical memory:

1. Identify current winner, historical loser, and update rationale for the same claim.
2. Preserve superseded facts as history instead of dropping or silently demoting them.
3. Expose tombstones and TTL invalidations as answerable lifecycle evidence.
4. Emit trace fields for conflict candidates, current selection, historical selection,
   tombstone selection, and rationale selection.
5. Add scorer gates so a retrieved-but-not-narrated conflict remains `wrong_result`.

Target benchmark: ELF live `memory_evolution` should pass all six jobs before any
claim that ELF has solved temporal memory.

### P0 - mem0/OpenMemory History Comparison

The fresh mem0 pass means the next useful comparison is no longer basic update/delete.
It should move to the product behavior users actually care about:

1. preference history across correction events;
2. entity-scoped memory lookup and update;
3. user-visible inspection/export of memory lifecycle;
4. deletion versus historical audit readback;
5. optional graph-memory behavior only if the OSS path is reproducible in Docker.

Target benchmark: mem0/OpenMemory and ELF both run comparable history jobs; claims are
made per scenario, not per project brand.

### P0 - qmd-Level Debugging And Replay

ELF should match qmd's practical debugging strengths:

1. show query expansion, sparse/dense retrieval, fusion, rerank, and final selection;
2. mark candidate-drop reasons;
3. include replay commands that do not require raw SQL;
4. connect wrong-result scores to specific missing stages;
5. keep artifacts local and reproducible.

Target benchmark: every wrong temporal or retrieval answer has a replayable trace that
explains whether evidence was absent, retrieved but dropped, selected but not narrated,
or contradicted by a higher-priority lifecycle fact.

### P1 - Core Memory Blocks

ELF should evaluate Letta-style core memory without weakening ELF's source-of-truth
discipline:

1. scoped read-only core blocks;
2. provenance and source ids on every core assertion;
3. explicit attach/detach rules;
4. stale-core detection when archival evidence supersedes a core statement;
5. fallback to archival search when core memory is insufficient.

Target benchmark: core-vs-archival jobs prove correct attachment, sharing, update
visibility, and stale-core avoidance.

### P1 - Capture, Consolidation, And Knowledge Pages

A good memory system is not only retrieval. ELF should benchmark and later optimize:

1. safe capture/write policy with redaction and exclusion proof;
2. reviewable consolidation proposals with source lineage and unsupported-claim flags;
3. project/entity knowledge pages that rebuild from authoritative notes;
4. timelines for changed decisions, ownership, and production state;
5. operator UX that explains failures without raw database inspection.

Target benchmark: live capture, consolidation, knowledge, and operator-debugging suites
must move from `not_encoded` or fixture-only to comparable live evidence.

### P2 - Graph/RAG And Context-Trajectory Adapters

Graph/RAG and context trajectory should be measured, not assumed:

1. Graphiti/Zep for temporal graph facts;
2. RAGFlow, LightRAG, and GraphRAG for document/chunk/graph evidence handles;
3. graphify for graph-compressed navigation reports;
4. OpenViking for staged context trajectory;
5. llm-wiki and gbrain for maintained knowledge workflows.

Target benchmark: each adapter must emit evidence-linked outputs from Docker-contained
or explicitly typed provider-backed runs before any ELF win/loss claim.

## Claim Boundaries

Allowed:

- ELF+mem0 basic local lifecycle smoke passed in the fresh Docker baseline.
- ELF narrowly outperformed qmd on the fresh memory-evolution slice because ELF passed
  delete/TTL and qmd did not.
- ELF still failed five of six live memory-evolution jobs.
- Graphiti/Zep temporal smoke is typed blocked due missing explicit provider key.
- Letta is a design reference, not a measured comparable competitor in this report.
- The next work should be benchmark/report driven before implementation work is
  claimed successful.

Not allowed:

- Do not claim all goals are complete.
- Do not claim ELF beats all tracked memory projects.
- Do not claim ELF beats mem0/OpenMemory on UI, hosted behavior, entity history, or
  graph memory.
- Do not claim ELF beats Graphiti/Zep on temporal validity.
- Do not claim ELF beats Letta on core-vs-archival memory.
- Do not treat fixture pass, baseline smoke pass, and live real-world pass as the
  same evidence class.

## Next Concrete Report/Issue Directions

1. Open or refine a P0 issue for ELF live temporal reconciliation and trace contract.
2. Open a P0 benchmark issue for mem0/OpenMemory history and UI/export readback.
3. Open a P0 benchmark issue for ELF/qmd trace-level replay and wrong-result
   diagnosis.
4. Open a P1 benchmark issue for Letta-style core-vs-archival memory.
5. Keep Graphiti/Zep provider-backed temporal smoke blocked until explicit provider
   credentials are available, then rerun and compare validity-window behavior.
6. Keep graph/RAG and knowledge-page adapters as P2 until Docker-contained evidence
   mappings are available.

## Bottom Line

ELF is not done competing. The evidence says ELF should keep its strict
source-of-truth and production-operation core, then absorb the best competitor ideas
behind benchmark gates. The immediate product-quality gap is temporal and lifecycle
memory: users need to know what is current, what changed, what was deleted, what is
historical, and why the system believes that answer.
