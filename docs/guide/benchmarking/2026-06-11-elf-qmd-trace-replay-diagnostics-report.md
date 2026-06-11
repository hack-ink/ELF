# ELF/qmd Trace Replay Diagnostics Report - June 11, 2026

Goal: Compare ELF and qmd on trace-level replay and wrong-result diagnostics while
keeping retrieval correctness as a separate guardrail.
Read this when: You need the XY-923 report lane for qmd top-10 replay artifacts,
ELF trace/admin bundle surfaces, and typed wrong-result diagnosis classes.
Inputs: The June 11 ELF/qmd retrieval-debug profile, qmd/OpenViking strength profile,
memory-evolution diagnostic, competitor-strength adoption report, live baseline
runner, ELF trace replay code, and the ELF service trace/admin contract.
Outputs: Scenario-level `win`, `tie`, `loss`, `not_tested`, `blocked`, or
`non_goal` outcomes plus concrete replay commands and artifact paths.

Machine-readable companion:
`docs/research/2026-06-11-elf-qmd-trace-replay-diagnostics-report.json`.

## Executive Judgment

Retrieval correctness is still tied: ELF and qmd both pass the encoded live retrieval
suite and both pass the 480-document generated-public stress baseline.

Trace-level debugging is not tied. In the current checked-in artifacts, qmd is ahead
on immediate local replay ergonomics because the baseline keeps top-10 JSON rows with
files, scores, line numbers, snippets, and distractor visibility, and the replay path
is a short CLI sequence. ELF has a deeper service trace model and admin bundle
surfaces, but the stress report still does not hydrate the equivalent candidate list
by default.

The resulting narrow position:

- Retrieval correctness: `tie`.
- Default per-query candidate artifact: ELF `loss` against qmd.
- Replay command locality: ELF `loss` against qmd.
- ELF trace/admin replay surface: `tie` as an available but different replay surface,
  not a default-artifact win.
- Expansion, dense/sparse contribution, fusion, and candidate-drop diagnostics:
  `not_tested` until comparable stage artifacts are emitted.
- Rerank stage scoring: `non_goal` for the current qmd stress path because it uses
  `--no-rerank`.
- Wrong-result selected-but-not-narrated diagnosis: `tie` on typed non-pass
  classification, not on answer quality.

This is not a broad qmd-over-ELF claim. It is a scored local-debug artifact gap.

## Replay Artifact Manifest

| System | Replay surface | Command | Artifact |
| --- | --- | --- | --- |
| ELF | Stress guardrail with trace ids | `ELF_BASELINE_PROJECTS=ELF,qmd ELF_BASELINE_PROFILE=stress cargo make baseline-live-docker` | `tmp/live-baseline/live-baseline-report.json`; summarized in `docs/research/2026-06-11-elf-qmd-retrieval-debug-profile.json` |
| ELF | Admin trace bundle hydration | `curl -fsS 'http://127.0.0.1:51891/v2/admin/traces/<trace_id>/bundle?mode=full&stage_items_limit=256&candidates_limit=200' -H 'X-ELF-Tenant-Id: <tenant>' -H 'X-ELF-Project-Id: <project>' -H 'X-ELF-Agent-Id: <agent>'` | `elf.trace_bundle/v1` response from the admin service |
| ELF | Trace ranking replay | `cargo run -p elf-eval -- --config-a config/local/elf.docker.toml --config-b config/local/elf.docker.toml --trace-id <trace_id>` | JSON trace compare output over `search_trace_candidates` |
| qmd | Stress guardrail and top-10 rows | `ELF_BASELINE_PROJECTS=qmd ELF_BASELINE_PROFILE=stress cargo make baseline-live-docker` | `tmp/live-baseline/qmd-query.json`; summarized in `docs/research/2026-06-11-elf-qmd-retrieval-debug-profile.json` |
| qmd | Per-query CLI replay | `npx tsx src/cli/qmd.ts query 'lex: <query>\nvec: <query>' -c elfbench --json --no-rerank --min-score 0 -n 10` | JSON top-10 rows with `file`, line/snippet/score fields when qmd returns them |
| qmd | Lifecycle replay | `npx tsx src/cli/qmd.ts update && npx tsx src/cli/qmd.ts embed -f -c elfbench && npx tsx src/cli/qmd.ts query ... --json --no-rerank` | `tmp/live-baseline/qmd-query.json` checks for update, delete, and cold-start recovery |

## Scenario Outcomes

| Scenario | Evidence | Result type | ELF outcome | Diagnostic judgment |
| --- | --- | --- | --- | --- |
| Retrieval correctness guardrail | `live_real_world`, `live_baseline_only` | `pass` | `tie` | Both systems pass encoded retrieval and stress same-corpus checks; this row does not score debugging ergonomics. |
| Default top-10 candidate artifact | `live_baseline_only` | `pass` | `loss` | qmd exposes file, score, line/snippet, and distractor rows directly; ELF records trace ids and top evidence but not the full candidate list in the report. |
| Replay command locality | `live_baseline_only` | `pass` | `loss` | qmd replay is a short local CLI query/update/embed path; ELF replay requires a live service config, persisted traces, headers, and trace ids. |
| Trace/admin replay surface availability | `implementation_reference` | `not_encoded` | `tie` | ELF has admin trace bundles and `elf-eval` trace replay; qmd has direct CLI replay. They are different useful surfaces and are not scored as equivalent quality. |
| Query expansion attribution | `research_gate` | `not_encoded` | `not_tested` | No comparable artifact shows expansion variants or dynamic expansion decisions for both systems. |
| Dense/sparse channel attribution | `research_gate` | `not_encoded` | `not_tested` | ELF uses dense plus BM25 and qmd uses structured `lex:` plus `vec:`, but the scored artifacts do not expose comparable per-channel contribution. |
| Fusion attribution | `research_gate` | `not_encoded` | `not_tested` | No comparable artifact shows fusion inputs, RRF/weighted-fusion contributions, or fusion-stage candidate drops. |
| Rerank attribution | `live_baseline_only` | `non_goal` | `non_goal` | The current qmd stress and materializer paths use `--no-rerank`; no rerank-on comparison is claimed. |
| Candidate-drop diagnostics | `research_gate` | `not_encoded` | `not_tested` | `retrieved_but_dropped` is defined but not observed because current qmd artifacts lack intermediate candidate traces and the ELF stress report does not hydrate candidate bundles. |
| Selected-but-not-narrated wrong results | `live_real_world` | `wrong_result` | `tie` | Both live paths produce memory-evolution wrong results where evidence is present but current-vs-historical or lifecycle narration is missing. |
| Evidence-absent and tombstone diagnosis | `live_real_world` | `wrong_result` | `win` | ELF retrieved all required memory-evolution evidence and passed delete/TTL; qmd missed three required evidence links including the delete tombstone. |

Summary: `1` ELF win, `3` ties, `2` ELF losses, `4` not-tested scenarios, `0`
blocked scenarios, and `1` non-goal scenario. The losses are local-debug artifact
losses only. They do not change the retrieval-correctness tie.

## Stage Scoring Notes

| Stage | Current score | Reason |
| --- | --- | --- |
| Expansion | `not_tested` | The current artifacts do not expose comparable expansion variants or dynamic expansion decisions. |
| Dense retrieval | `not_tested` | The systems have dense/vector surfaces, but no comparable scored dense-only contribution artifact. |
| Sparse retrieval | `not_tested` | qmd `lex:` and ELF BM25 are present in command or service design, but contribution and drops are not scored. |
| Fusion | `not_tested` | Fusion candidates and final fusion deltas are not materialized comparably. |
| Rerank | `non_goal` | qmd uses `--no-rerank` in the current path; rerank superiority is out of scope for this run. |
| Candidate drops | `not_tested` | No current report can prove retrieved-but-dropped evidence for qmd, and ELF candidate bundles are not hydrated into the stress artifact. |
| Selected-but-not-narrated | `tie` | Both systems have typed memory-evolution wrong-result rows where evidence is selected or available but not narrated as lifecycle history. |
| Replay commands | `loss` | qmd's local CLI replay is shorter and directly tied to top-10 JSON output. |

## Typed Non-Pass States

The report preserves the wrong-result classes from the June 11 diagnostics:

| Class | Current coverage |
| --- | --- |
| `evidence_absent` | Observed for qmd on verdict caveat, preference rationale, and delete tombstone misses. |
| `retrieved_but_dropped` | Defined but `not_tested`; current artifacts do not expose enough candidate-stage data. |
| `selected_but_not_narrated` | Observed for both ELF and qmd on supersession and temporal-validity jobs. |
| `contradicted_by_lifecycle_evidence` | Observed when current, historical, supersession, or tombstone evidence makes the answer incomplete. |

These states are typed evidence, not leaderboard shortcuts. A `wrong_result` with
good evidence recall is still a wrong result.

## Claim Boundaries

Allowed:

- ELF and qmd remain tied on encoded retrieval correctness.
- qmd currently wins the default local-debug artifact surface: top-10 rows plus short
  CLI replay.
- ELF has useful service trace/admin replay surfaces, but they are not yet hydrated
  into the default stress report as qmd-like candidate artifacts.
- ELF narrowly wins the memory-evolution evidence-retention slice because qmd misses
  the delete tombstone and two other required evidence links.
- Expansion, dense/sparse contribution, fusion, rerank-on quality, and
  retrieved-but-dropped candidate diagnosis remain unproven.

Not allowed:

- Do not claim qmd beats ELF as a memory system overall.
- Do not claim ELF beats qmd retrieval overall.
- Do not turn qmd top-10 ergonomics into a retrieval-quality win.
- Do not treat ELF trace/admin endpoint availability as proof that the default
  benchmark report has qmd-level candidate visibility.
- Do not score rerank superiority from a qmd `--no-rerank` run.
- Do not collapse `not_tested`, `non_goal`, or `wrong_result` into pass evidence.

## Follow-Up Gate

The next measurement should emit one candidate-replay artifact per suspicious query
with:

1. Expansion variants and whether the original query was included.
2. Dense-only and sparse-only candidate sets.
3. Fusion rank and score contribution.
4. Rerank score, or an explicit rerank-disabled marker.
5. Final selected items.
6. Dropped or demoted expected evidence.
7. A one-command replay line for both ELF and qmd.

Until that exists, the current evidence supports a qmd local-debug artifact win, not a
broad product or retrieval win.
