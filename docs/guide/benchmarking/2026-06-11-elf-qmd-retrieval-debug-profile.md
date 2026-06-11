# ELF/qmd Retrieval-Debug Profile - June 11, 2026

Goal: Compare the measured retrieval-debug evidence for ELF and qmd without turning
retrieval success into a broader memory-system win claim.
Read this when: You need to decide what ELF should learn from qmd's retrieval and
debug workflow.
Inputs: Fresh local runs of `cargo make real-world-memory-live-adapters` and
`ELF_BASELINE_PROJECTS=ELF,qmd ELF_BASELINE_PROFILE=stress cargo make
baseline-live-docker` on commit `38c586d`.
Outputs: Retrieval pass data, stress-profile data, debug artifact comparison, claim
boundaries, and ELF iteration directions.

## Executive Judgment

ELF and qmd are tied on the measured retrieval correctness surfaces in this report.
Both pass the encoded real-world retrieval suite and both pass the 480-document
generated-public stress baseline.

qmd still remains the better retrieval-debug product reference because its CLI baseline
emits directly inspectable top-10 JSON results with files, line numbers, snippets, and
scores for every query. ELF emits stronger service and production-operation evidence,
including trace ids, backfill checkpoints, Qdrant rebuild proof, resource envelope,
and source-of-truth semantics, but the stress baseline report does not hydrate the full
candidate list behind each ELF trace.

So the correct claim is:

- ELF and qmd are tied on current encoded retrieval correctness.
- ELF is stronger on source-of-truth and production-style service lifecycle evidence.
- qmd is still the simpler local retrieval-debug reference.
- This report does not prove qmd rerank quality, ELF rerank quality, or expansion /
  fusion superiority because the qmd real-world materializer and baseline use
  `--no-rerank`, and no scored expansion/fusion/rerank debug suite exists yet.

## Fresh Runs

| Command | Result | Runtime |
| --- | --- | ---: |
| `cargo make real-world-memory-live-adapters` | pass | 116.76 seconds |
| `ELF_BASELINE_PROJECTS=ELF,qmd ELF_BASELINE_PROFILE=stress cargo make baseline-live-docker` | pass | 149.41 seconds |

The stress baseline used the generated-public profile with 480 documents and 16
queries. The live real-world adapter sweep used the checked-in real-world memory
fixtures.

## Real-World Retrieval Suite

Both adapters pass the same retrieval jobs:

| Adapter | Retrieval jobs | Pass | Expected evidence | Matched evidence | Produced evidence | Mean score |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| ELF live service adapter | `5` | `5` | `6` | `6` | `6` | `1.000` |
| qmd live CLI adapter | `5` | `5` | `6` | `6` | `6` | `1.000` |

The five retrieval jobs are:

| Job | ELF | qmd |
| --- | --- | --- |
| `retrieval-alt-phrasing-001` | pass | pass |
| `retrieval-current-vs-obsolete-001` | pass | pass |
| `retrieval-distractor-heavy-001` | pass | pass |
| `retrieval-minimal-context-001` | pass | pass |
| `retrieval-multi-hop-routing-001` | pass | pass |

Full live sweep context remains a non-pass for both systems:

| Adapter | Jobs | Pass | Wrong result | Blocked | Not encoded | Mean score | Mean latency |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ELF live service adapter | `38` | `18` | `5` | `2` | `13` | `0.525` | `5.823 ms` |
| qmd live CLI adapter | `38` | `17` | `6` | `2` | `13` | `0.486` | `691.163 ms` |

Do not overread the latency row. The ELF adapter is a service-runtime path and the qmd
adapter is a CLI materialization path; the row is useful as observed harness evidence,
not as an apples-to-apples product latency benchmark. The aggregate pass-count
difference comes from the memory-evolution delete/TTL tombstone job; it does not erase
qmd's local retrieval-debug ergonomics advantage.

## Stress Baseline

The stress baseline result:

| Field | Value |
| --- | ---: |
| Profile | `stress` |
| Documents | `480` |
| Queries | `16` |
| Projects | `ELF,qmd` |
| Verdict | `pass` |
| Project statuses | `2/2 pass` |
| Full checks | `13/13 pass` |
| Wrong result | `0` |
| Lifecycle fail | `0` |
| Blocked | `0` |
| Not encoded | `0` |

### ELF Stress Result

| Metric | Value |
| --- | ---: |
| Project elapsed | `81 s` |
| Query pass | `16/16` |
| Mean query latency | `29.808 ms` |
| p95 query latency | `31.298 ms` |
| Backfill source count | `480` |
| Backfill completed count | `480` |
| Resume attempts | `2` |
| Completed before resume | `240` |
| Completed after resume | `480` |
| Duplicate source notes | `0` |
| Qdrant rebuild scope | encoded in the pass criteria |
| Resource envelope elapsed | `71.303 s` |
| RSS | `54,724 KB` |
| Postgres database bytes | `19,338,943` |
| Estimated input tokens | `27,023` |

ELF passed nine checks:

| Check | Status |
| --- | --- |
| `resumable_backfill_no_duplicates` | pass |
| `same_corpus_retrieval` | pass |
| `async_worker_indexing_e2e` | pass |
| `update_replaces_note_text` | pass |
| `delete_suppresses_retrieval` | pass |
| `cold_start_recovery_search` | pass |
| `concurrent_write_search_e2e` | pass |
| `soak_stability_e2e` | pass |
| `resource_envelope` | pass |

Every ELF stress query returned the expected evidence as the top evidence id.

### qmd Stress Result

| Metric | Value |
| --- | ---: |
| qmd commit | `636602409c862db077f38d9006df7f0bdca17ff3` |
| Project elapsed | `66 s` |
| Same-corpus query pass | `16/16` |
| Expected doc top-1 | `16/16` |
| Mean expected-doc rank | `1.000` |
| Mean distractors in top-10 | `7.938` |
| Lifecycle checks | `4/4 pass` |

qmd passed four checks:

| Check | Status | Evidence |
| --- | --- | --- |
| `same_corpus_retrieval` | pass | 16/16 queries matched expected evidence. |
| `update_replaces_note_text` | pass | updated marker `kid-v4` was found and old marker was absent. |
| `delete_suppresses_retrieval` | pass | deleted `deploy-memory.md` no longer matched. |
| `cold_start_recovery_search` | pass | fresh qmd query process retrieved persisted `database-memory.md`. |

The qmd baseline report keeps per-query top-10 JSON results. This is the most concrete
measured qmd debug advantage in this report: an operator can inspect matched files,
scores, line numbers, snippets, and distractor density directly from the artifact.

### Per-Query Stress Observations

| Query | ELF matched top evidence | ELF latency | qmd expected rank | qmd top-10 distractors |
| --- | --- | ---: | ---: | ---: |
| `q-auth` | yes | `30.571 ms` | `1` | `6` |
| `q-auth-alt` | yes | `30.501 ms` | `1` | `7` |
| `q-database` | yes | `30.534 ms` | `1` | `8` |
| `q-database-alt` | yes | `31.281 ms` | `1` | `8` |
| `q-deploy` | yes | `29.958 ms` | `1` | `9` |
| `q-deploy-alt` | yes | `31.298 ms` | `1` | `8` |
| `q-retention` | yes | `30.434 ms` | `1` | `8` |
| `q-retention-alt` | yes | `29.194 ms` | `1` | `9` |
| `q-incident` | yes | `30.839 ms` | `1` | `7` |
| `q-incident-alt` | yes | `28.700 ms` | `1` | `9` |
| `q-billing` | yes | `30.092 ms` | `1` | `7` |
| `q-billing-alt` | yes | `28.855 ms` | `1` | `9` |
| `q-search` | yes | `29.480 ms` | `1` | `8` |
| `q-search-alt` | yes | `28.642 ms` | `1` | `7` |
| `q-recovery` | yes | `28.357 ms` | `1` | `8` |
| `q-recovery-alt` | yes | `28.188 ms` | `1` | `9` |

## Debug Artifact Comparison

| Debug surface | ELF evidence | qmd evidence | Current judgment |
| --- | --- | --- | --- |
| Per-query pass/fail | yes | yes | tied |
| Top expected evidence | yes, top evidence id per query | yes, expected file rank per query | tied on stress profile |
| Candidate list in report | partial: trace id, top snippet, returned count | yes: top-10 file, line, score, snippet | qmd stronger in the checked-in report artifact |
| Trace/replay surface | service trace ids exist | CLI command replay is explicit | different strengths; not directly scored |
| Update/delete/cold-start | yes, service lifecycle checks | yes, collection lifecycle checks | tied on encoded lifecycle correctness |
| Backfill/rebuild/resource envelope | yes | not represented in qmd baseline | ELF stronger |
| Rerank evidence | not scored here | not scored here; qmd path uses `--no-rerank` | non-claim |
| Expansion/fusion evidence | not scored here | structured `lex:` plus `vec:` query is used, but fusion internals are not scored | non-claim |
| Operator-debugging UX suite | live `not_encoded` | live `not_encoded` | non-claim |

## What ELF Should Learn From qmd

1. Put the ranked candidate list in the default benchmark artifact.
   - The qmd artifact makes the top-10 result set immediately visible.
   - ELF has trace ids, but a reader still needs another trace-hydration step to see
     the candidate list and dropped/demoted candidates.

2. Make replay commands short and local.
   - qmd's measured surface is `collection add`, `update`, `embed -f`, and
     `query --json`.
   - ELF should keep service correctness, but benchmark reports should also emit a
     concise replay command for each failed or suspicious query.

3. Score distractor density and candidate-drop behavior.
   - qmd returned the expected doc at rank 1 for every stress query, while still
     returning an average of 7.938 distractor documents in the top 10.
   - ELF should expose equivalent candidate-density metrics from trace candidates so
     the report can distinguish "correct top result" from "clean ranked context."

4. Separate retrieval correctness from retrieval-debug ergonomics.
   - Correctness is currently tied on encoded retrieval jobs.
   - Ergonomics are not tied until ELF produces qmd-like immediate debug artifacts and
     qmd operator-debugging jobs are actually scored.

## Claim Boundaries

Allowed claims:

- ELF and qmd both pass the encoded real-world retrieval suite.
- ELF and qmd both pass the 480-document generated-public stress same-corpus
  retrieval profile.
- qmd provides stronger directly inspectable top-10 query artifacts in the current
  stress baseline report.
- ELF provides stronger service lifecycle, backfill, rebuild, resource, and
  source-of-truth evidence in the same stress baseline.

Not allowed yet:

- ELF beats qmd retrieval overall.
- qmd beats ELF as a memory system overall.
- Either system has a full live real-world suite pass.
- Either system has measured rerank superiority from this report.
- Either system has measured expansion/fusion superiority from this report.
- qmd operator-debugging UX is proven by the live real-world suite; it is still
  `not_encoded`.

## Next Measurement Work

The next report should close the remaining retrieval-debug gaps before making stronger
claims:

1. Hydrate ELF trace candidates into the stress report.
   - Include kept, dropped, demoted, sparse/dense, final rank, and snippet fields.

2. Add qmd query latency and candidate-density aggregates to the project summary.
   - The raw qmd top-10 rows exist, but the summary currently lacks query latency and
     candidate-density counters.

3. Add a rerank-on qmd profile or explicitly keep qmd rerank as unmeasured.
   - Current qmd materialization uses `--no-rerank`.

4. Add a scored operator-debugging retrieval job for both systems.
   - The job should ask why a result was wrong or why a distractor appeared, not only
     whether the top result was correct.

5. Add an expansion/fusion trace profile.
   - Score lex-only, vec-only, hybrid, fusion, and final ranking stages separately.

## Bottom Line

This profile strengthens the evidence base but does not close the competitiveness
goal. Retrieval correctness is currently tied between ELF and qmd on encoded data.
ELF's next useful iteration direction is not "more retrieval" in the abstract; it is
qmd-level immediate retrieval debugging while preserving ELF's stronger
source-of-truth, trace, backfill, and production-operation model.
