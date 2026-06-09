# Real-World Job Benchmark Report

Goal: Publish a Markdown summary for one generated real_world_job benchmark report.
Read this when: You need a durable smoke report for real-world agent memory job fixtures.
Inputs: `tmp/real-world-job/real-world-job-operator-ux-report.json`.
Depends on: `apps/elf-eval/fixtures/real_world_job/`, `docs/spec/real_world_agent_memory_benchmark_v1.md`, and `Makefile.toml`.
Verification: Compare this Markdown summary with the source JSON before committing.

## Summary

- Run ID: `real-world-job-operator-ux`
- Generated at: `2026-06-09T14:52:05.906877Z`
- Runner version: `0.2.0-9b60dee3de54705a71a683d9a36b48d94ce8e752-aarch64-apple-darwin`
- Corpus profile: `synthetic`
- Adapter: `fixture_operator_ux` (offline_fixture_response)
- Jobs: `5`
- Encoded suites: `1`
- Not-encoded suites: `10`
- Status summary: `4` pass, `0` wrong_result, `0` lifecycle_fail, `0` incomplete, `0` blocked, `1` unsupported_claim
- Unsupported claim count: `1`
- Wrong-result count: `3`
- Mean score: `0.800`
- Mean latency: `3.100 ms`
- Cost: `0.000 USD`
- Operator-debug jobs: `5`
- Raw SQL needed: `0`
- Trace-incomplete debug jobs: `0`
- Operator UX gaps: `0`
- Private corpus redaction: `no_private_corpus`

## Suites

| Suite | Status | Jobs | Score | Unsupported Claims | Wrong Results | Reason |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| trust_source_of_truth | `not_encoded` | 0 | `-` | 0 | 0 | No checked-in real_world_job fixture is encoded for this suite. |
| work_resume | `not_encoded` | 0 | `-` | 0 | 0 | No checked-in real_world_job fixture is encoded for this suite. |
| project_decisions | `not_encoded` | 0 | `-` | 0 | 0 | No checked-in real_world_job fixture is encoded for this suite. |
| retrieval | `not_encoded` | 0 | `-` | 0 | 0 | No checked-in real_world_job fixture is encoded for this suite. |
| memory_evolution | `not_encoded` | 0 | `-` | 0 | 0 | No checked-in real_world_job fixture is encoded for this suite. |
| consolidation | `not_encoded` | 0 | `-` | 0 | 0 | No checked-in real_world_job fixture is encoded for this suite. |
| knowledge_compilation | `not_encoded` | 0 | `-` | 0 | 0 | No checked-in real_world_job fixture is encoded for this suite. |
| operator_debugging_ux | `unsupported_claim` | 5 | `0.800` | 1 | 3 | At least one encoded job produced an unsupported claim. |
| capture_integration | `not_encoded` | 0 | `-` | 0 | 0 | No checked-in real_world_job fixture is encoded for this suite. |
| production_ops | `not_encoded` | 0 | `-` | 0 | 0 | No checked-in real_world_job fixture is encoded for this suite. |
| personalization | `not_encoded` | 0 | `-` | 0 | 0 | No checked-in real_world_job fixture is encoded for this suite. |

## Jobs

| Suite | Job | Status | Score | Expected Evidence | Produced Evidence | Unsupported Claims | Wrong Results | Latency | Cost |
| --- | --- | --- | ---: | --- | --- | ---: | ---: | ---: | --- |
| operator_debugging_ux | operator-debug-dropped-evidence-001 | `unsupported_claim` | `0.000` | `trace-dropped-expected` | `trace-dropped-decoy` | 1 | 3 | `2.400 ms` | `0.000 USD` |
| operator_debugging_ux | operator-debug-provider-latency-001 | `pass` | `1.000` | `trace-provider-timeout` | `trace-provider-timeout` | 0 | 0 | `4.800 ms` | `0.000 USD` |
| operator_debugging_ux | operator-debug-rebuild-changed-results-001 | `pass` | `1.000` | `trace-before-rebuild, trace-after-rebuild` | `trace-after-rebuild, trace-before-rebuild` | 0 | 0 | `3.300 ms` | `0.000 USD` |
| operator_debugging_ux | operator-debug-relation-context-mislead-001 | `pass` | `1.000` | `trace-relation-context` | `trace-relation-context` | 0 | 0 | `2.900 ms` | `0.000 USD` |
| operator_debugging_ux | operator-debug-rerank-bad-candidate-001 | `pass` | `1.000` | `trace-rerank-promotion` | `trace-rerank-promotion` | 0 | 0 | `2.100 ms` | `0.000 USD` |

## Operator Debugging UX

| Job | Failure Mode | Trace Evidence | Steps | Raw SQL | Dropped Candidate Visibility | Trace Completeness | Repair Clarity | UX Gaps |
| --- | --- | --- | ---: | --- | --- | --- | --- | --- |
| operator-debug-dropped-evidence-001 | expected_evidence_dropped | `11111111-1111-4111-8111-111111111111`<br>[viewer](/viewer?trace_id=11111111-1111-4111-8111-111111111111)<br>[bundle](/v2/admin/traces/11111111-1111-4111-8111-111111111111/bundle?mode=full&stage_items_limit=128&candidates_limit=200) | 4 | `false` | visible in Retrieval Funnel and Replay Candidates | `complete` | `clear` | `none` |
| operator-debug-provider-latency-001 | provider_latency_or_failure | `33333333-3333-4333-8333-333333333333`<br>[viewer](/viewer?trace_id=33333333-3333-4333-8333-333333333333)<br>[bundle](/v2/admin/traces/33333333-3333-4333-8333-333333333333/bundle?mode=full&stage_items_limit=128&candidates_limit=200) | 3 | `false` | visible as low recall counts rather than a post-recall drop | `complete` | `clear` | `none` |
| operator-debug-rebuild-changed-results-001 | rebuild_changed_results | `44444444-4444-4444-8444-444444444444`<br>[viewer](/viewer?trace_id=44444444-4444-4444-8444-444444444444)<br>[bundle](/v2/admin/traces/44444444-4444-4444-8444-444444444444/bundle?mode=full&stage_items_limit=128&candidates_limit=200) | 5 | `false` | visible by comparing before and after trace candidates | `complete` | `clear` | `none` |
| operator-debug-relation-context-mislead-001 | relation_context_misled_search | `55555555-5555-4555-8555-555555555555`<br>[viewer](/viewer?trace_id=55555555-5555-4555-8555-555555555555)<br>[bundle](/v2/admin/traces/55555555-5555-4555-8555-555555555555/bundle?mode=full&stage_items_limit=128&candidates_limit=200) | 4 | `false` | not dropped; misleading context is visible on selected result | `complete` | `clear` | `none` |
| operator-debug-rerank-bad-candidate-001 | rerank_promoted_bad_candidate | `22222222-2222-4222-8222-222222222222`<br>[viewer](/viewer?trace_id=22222222-2222-4222-8222-222222222222)<br>[bundle](/v2/admin/traces/22222222-2222-4222-8222-222222222222/bundle?mode=full&stage_items_limit=128&candidates_limit=200) | 3 | `false` | not dropped; visible with lower final rank in Replay Candidates | `complete` | `clear` | `none` |

### Operator Debug Details

#### `operator-debug-dropped-evidence-001`

- Root cause: The expected candidate survived recall but was removed by the read-profile scope filter before final selection.
- Viewer panels: `Trace, Retrieval Funnel, Replay Candidates, Stage Details`
- CLI steps: `open viewer trace link -> compare recall before and after filter -> inspect replay candidates -> repair read profile or grant`
- Trace evidence: `trace-dropped-expected`

#### `operator-debug-provider-latency-001`

- Root cause: Provider latency forced fallback behavior, shrinking expanded-query recall.
- Viewer panels: `Providers And Ranking, Stage Summary, Stage Details`
- CLI steps: `open trace bundle -> inspect provider metadata -> compare expanded queries -> raise timeout or repair provider health`
- Trace evidence: `trace-provider-timeout`

#### `operator-debug-rebuild-changed-results-001`

- Root cause: Rebuild removed stale derived-index state and restored source-of-truth-backed ranking.
- Viewer panels: `Trace, Replay Candidates, Selected Final Results`
- CLI steps: `open before trace -> open after trace -> compare replay candidates -> confirm active note selected -> keep Qdrant rebuild as repair`
- Trace evidence: `trace-before-rebuild, trace-after-rebuild`

#### `operator-debug-relation-context-mislead-001`

- Root cause: A deprecated graph relation remained visible in relation_context and conflicted with the selected note text.
- Viewer panels: `Selected Final Results, Relation Context, Stage Details`
- CLI steps: `open trace link -> inspect selected result relation count -> open Relation Context -> invalidate stale relation fact`
- Trace evidence: `trace-relation-context`

#### `operator-debug-rerank-bad-candidate-001`

- Root cause: The correct item was in the candidate set, but rerank.score elevated a cross-project decoy.
- Viewer panels: `Selected Final Results, Replay Candidates, Providers And Ranking`
- CLI steps: `open trace bundle -> compare retrieval rank with final rank -> inspect rerank score -> tighten scope or rerank inputs`
- Trace evidence: `trace-rerank-promotion`

## Unsupported Claims

| Suite | Job | Claim | Evidence | Reason |
| --- | --- | --- | --- | --- |
| operator_debugging_ux | operator-debug-dropped-evidence-001 | No expected evidence was dropped. | `trace-dropped-decoy` | claim_id is not present in expected_answer.evidence_links |

## Result Semantics

This report uses `docs/spec/real_world_agent_memory_benchmark_v1.md` status terms.
It is a real-world job fixture report, not a Docker live-baseline report.
Existing live-baseline reports remain valid for their encoded retrieval and lifecycle checks and are not reinterpreted as real-world suite wins.

- `pass`: encoded jobs met their pass threshold with required evidence and no hard-fail rule.
- `wrong_result`: a job completed but missed required answer or evidence expectations.
- `unsupported_claim`: a job produced a substantive claim not supported by the fixture evidence links.
- `not_encoded`: a suite has no checked-in real_world_job fixture, so no pass/fail claim is allowed.

## Not-Encoded Suites

- `trust_source_of_truth`
- `work_resume`
- `project_decisions`
- `retrieval`
- `memory_evolution`
- `consolidation`
- `knowledge_compilation`
- `capture_integration`
- `production_ops`
- `personalization`
