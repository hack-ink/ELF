# qmd and OpenViking Strength-Profile Report - June 11, 2026

Goal: Compare ELF against qmd and OpenViking on their actual strengths without
turning broad live-sweep or smoke results into unsupported win claims.
Read this when: You need the XY-899 scenario-level qmd retrieval-debug and
OpenViking context-trajectory benchmark/report outcome.
Inputs: The June 11 retrieval-debug, memory-evolution, and temporal-history reports,
the real-world benchmark spec, the external adapter manifest, and
`scripts/real-world-live-adapters.sh`.
Outputs: Scenario-level win/tie/loss/not-tested judgments, qmd wrong-result
diagnosis taxonomy, OpenViking typed trajectory blockers, and claim boundaries.

Machine-readable companion:
`docs/research/2026-06-11-qmd-openviking-strength-profile-report.json`.

## Executive Judgment

ELF does not have a broad win against either qmd or OpenViking on their strengths.

The measured qmd judgment is narrower:

- Retrieval quality: `tie`. ELF and qmd both pass the encoded live real-world
  retrieval suite and both pass the 480-document stress retrieval baseline.
- Debug/replay ergonomics: `elf_loss`. qmd's current artifacts expose directly
  inspectable top-10 JSON rows with files, line numbers, snippets, scores, and short
  replay commands. ELF has stronger service traces and production-operation evidence,
  but the checked-in stress report does not hydrate an equivalent candidate list.
- Expansion/fusion/rerank controls: `not_tested`. The current qmd materializer and
  stress run use `--no-rerank`; no scored expansion/fusion/rerank profile exists.

The measured OpenViking judgment is split by surface:

- Same-corpus evidence-bearing preconditions: `elf_win`. The pinned Docker local
  embedding path reaches `add_resource`/`find`, but the OpenViking smoke remains
  `wrong_result` because expected evidence terms are missed while ELF passes the
  equivalent retrieval precondition.
- Context trajectory strengths: `not_tested`. The current OpenViking wrong-result
  smoke is not a scored staged-trajectory comparison.
- Staged retrieval, hierarchy selection, and recursive/context expansion remain
  `research_gate` / `not_encoded`; no ELF win, tie, or loss is claimed against those
  strengths.

## qmd Scenario Outcomes

| Scenario | Evidence Class | Result Type | ELF Outcome | What It Means |
| --- | --- | --- | --- | --- |
| Retrieval quality | `live_real_world` | `pass` | `tie` | Both systems pass 5/5 live retrieval jobs with 6/6 expected evidence matched. |
| Local query transparency | `live_baseline_only` | `pass` | `elf_loss` | qmd exposes top-10 files, line numbers, snippets, scores, and distractor density directly in the stress artifact. |
| Expansion/fusion/rerank controls | `research_gate` | `not_encoded` | `not_tested` | No scored profile proves either system's expansion, fusion, or rerank superiority. |
| Stale context isolation | `live_real_world` | `pass` | `tie` | Both systems pass the encoded current-vs-obsolete and distractor-heavy retrieval jobs. |
| Update/delete/cold-start behavior | `live_baseline_only` | `pass` | `tie` | Equivalent update replacement, delete suppression, and cold-start recovery checks pass for both. |
| Operator-debug evidence | `live_real_world` | `not_encoded` | `not_tested` | The live sweep marks operator-debugging UX `not_encoded` for both systems. |
| Local replayability | `live_baseline_only` | `pass` | `elf_loss` | qmd has a shorter checked-in CLI replay path for the current stress profile. |
| Wrong-result diagnosis | `research_gate` | `not_encoded` | `not_tested` | The report classifies qmd memory-evolution failures, but qmd candidate-drop traces are not yet materialized and no pass evidence is claimed. |

Summary: qmd strength-profile outcomes are `0` ELF wins, `3` ties, `2` ELF losses,
and `3` not-tested scenarios. This distinguishes retrieval quality from
debug/replay ergonomics: the retrieval result is tied, but the checked-in debug
artifact ergonomics currently favor qmd.

## qmd Wrong-Result Diagnosis

The report adds a qmd diagnosis taxonomy with four classes:

| Diagnosis Class | Current qmd Coverage |
| --- | --- |
| `evidence_absent` | Observed on the verdict caveat, preference rationale, and delete tombstone misses. |
| `retrieved_but_dropped` | Defined but not observed because current qmd live job artifacts do not expose candidate-stage traces. |
| `selected_but_not_narrated` | Observed on supersession jobs where qmd had evidence but did not narrate current-vs-historical state. |
| `contradicted_by_lifecycle_evidence` | Observed when current, historical, supersession, or tombstone evidence keeps the answer in typed `wrong_result` state. |

The key qmd memory-evolution diagnosis is unchanged from the June 11 diagnostic:
qmd is `0/6` pass on live memory-evolution, misses three required evidence links,
and fails the delete/TTL tombstone job. The new report records that as typed
diagnosis evidence, not as a broad ELF-over-qmd claim.

## OpenViking Scenario Outcomes

| Scenario | Evidence Class | Result Type | ELF Outcome | Typed Blocker |
| --- | --- | --- | --- | --- |
| Docker local embedding setup | `live_baseline_only` | `pass` | `not_tested` | none |
| Same-corpus evidence-bearing retrieval precondition | `live_baseline_only` | `wrong_result` | `elf_win` | `output_missed_expected_terms` |
| Staged retrieval trajectory | `research_gate` | `not_encoded` | `not_tested` | `needs_evidence_bearing_same_corpus_output` |
| Hierarchy selection | `research_gate` | `not_encoded` | `not_tested` | `hierarchy_output_not_scored` |
| Recursive/context expansion | `research_gate` | `not_encoded` | `not_tested` | `recursive_expansion_not_materialized` |
| Missed expected terms evidence | `live_baseline_only` | `wrong_result` | `elf_win` | `retrieval_wrong_result` |

Summary: OpenViking profile outcomes are `2` ELF wins, `0` ties, `0` ELF losses, and
`4` not-tested scenarios. The two wins are only same-corpus evidence-bearing
preconditions and missed-term failure evidence. The current smoke wrong-result is
useful typed failure evidence, but it is not a scored staged-trajectory comparison,
so context-trajectory strengths remain not tested.

## Claim Boundaries

Allowed:

- ELF ties qmd on the current encoded retrieval-correctness surfaces.
- qmd remains stronger than ELF on the currently evidenced local query transparency
  and replay artifact ergonomics.
- qmd expansion/fusion/rerank superiority is untested.
- OpenViking's Docker local embedding setup reaches runtime, but context trajectory
  remains untested because evidence-bearing same-corpus retrieval is not passing.
- ELF currently wins only the equivalent OpenViking same-corpus retrieval
  precondition surfaces, not OpenViking's staged trajectory strengths.

Not allowed:

- Do not claim ELF broadly beats qmd.
- Do not claim qmd's debug ergonomics are equivalent to retrieval quality.
- Do not claim ELF beats OpenViking on staged retrieval, hierarchy, or recursive
  context expansion.
- Do not turn `research_gate`, `not_encoded`, or `unsupported` surfaces into wins or
  losses.

## Validation Hook

The checked-in consistency test reads the machine-readable companion report and
asserts the qmd/OpenViking scenario counts, diagnosis taxonomy, and bottom-line
claim boundaries. This keeps future report edits from silently converting untested
strength surfaces into pass claims.
