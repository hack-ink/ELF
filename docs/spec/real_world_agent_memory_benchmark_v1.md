# Real-World Agent Memory Benchmark v1

Purpose: Define the v1 benchmark contract for evaluating agent memory systems through
real user jobs instead of isolated top-k retrieval queries.
Status: normative
Read this when: You are implementing, validating, reporting, or extending real-world
agent memory benchmark suites.
Not this document: Runner implementation steps, large fixture generation, operator
commands, or production adoption verdicts.
Defines: `real_world_job` schema, suite taxonomy, scoring dimensions, report states,
allowed uncertainty, and external reference mapping.

## Scope

The benchmark unit is `real_world_job`: a replayable user job that combines a corpus,
timeline, user prompt, expected answer, required evidence, negative traps, scoring
rubric, and allowed uncertainty. A job is intended to answer one question: would this
memory system help an agent do real work correctly, with less repetition and fewer
unsupported claims?

This contract is future benchmark authority only. Existing live baseline reports remain
valid evidence for their encoded retrieval and lifecycle checks. A project must not
claim wins under this v1 suite until a runner encodes the relevant suites and publishes
a report against this contract.

## Design Goals

- Evaluate job completion, not only whether one expected chunk appears in top-k.
- Reward evidence-backed answers, stale-fact handling, and recoverable reasoning.
- Penalize confident but unsupported claims even when retrieval looks plausible.
- Preserve typed failure states instead of flattening every result into one leaderboard.
- Keep external project strengths visible as suite references, not as automatic
  superiority claims.

## Why The Current Benchmark Is Incomplete

The June 2026 live baseline is necessary but biased toward service-style retrieval and
encoded lifecycle checks. ELF and qmd leading that matrix proves that those systems can
retrieve expected evidence and pass encoded update/delete/cold-start checks under the
selected Docker profiles. It does not prove that they help an agent resume a lane,
explain a decision, debug a failed retrieval, reconcile stale notes, compile durable
knowledge, or avoid unsupported claims during an end-to-end user job.

This suite fixes that bias by making the job transcript, expected answer, required
evidence, traps, and scoring rubric first-class. A system can pass retrieval and still
fail a real-world job if it repeats completed work, cites obsolete evidence, omits a
blocking caveat, or fabricates a decision that is not in the corpus.

## Real-World Job Schema

A `real_world_job` record MUST include the fields below. JSON is the canonical exchange
shape; YAML fixtures MAY be used only when converted to the same field names before
runner execution.

```json
{
  "schema": "elf.real_world_job/v1",
  "job_id": "trust-sot-restore-001",
  "suite": "trust_source_of_truth",
  "title": "Recover the authoritative restore decision",
  "corpus": {},
  "timeline": [],
  "prompt": {},
  "expected_answer": {},
  "required_evidence": [],
  "negative_traps": [],
  "scoring_rubric": {},
  "allowed_uncertainty": {},
  "operator_debug": {},
  "encoding": {},
  "memory_evolution": {},
  "tags": []
}
```

### Required Top-Level Fields

| Field | Type | Required semantics |
| --- | --- | --- |
| `schema` | string | MUST equal `elf.real_world_job/v1`. |
| `job_id` | string | Stable ASCII identifier unique within a suite. |
| `suite` | string | One suite id from the Suite Taxonomy section. |
| `title` | string | Human-readable job title. |
| `corpus` | object | Documents, memory items, traces, source refs, and adapter setup needed to replay the job. |
| `timeline` | array | Ordered events that establish what happened before the user prompt. |
| `prompt` | object | The user-facing request sent to the evaluated memory system or agent harness. |
| `expected_answer` | object | Required answer content, accepted uncertainty, and forbidden claims. |
| `required_evidence` | array | Evidence ids, source refs, quotes, or trace handles that must support the answer. |
| `negative_traps` | array | Distractors, stale facts, or misleading memories that must not drive the answer. |
| `scoring_rubric` | object | Dimensions, weights, thresholds, and hard-fail rules for this job. |
| `allowed_uncertainty` | object | Explicit uncertainty language and fallback behavior accepted for the job. |
| `operator_debug` | object or null | Optional for most suites; required for `operator_debugging_ux` jobs. Records trace/viewer evidence and operator workflow scoring inputs. |
| `encoding` | object | Optional job-level limitation declaration. Only `not_encoded`, `blocked`, and `incomplete` statuses are allowed here. |
| `memory_evolution` | object or null | Optional for most suites; used by `memory_evolution` jobs to report current evidence, historical evidence, stale traps, conflicts, update rationale, and temporal-validity limitations. |
| `tags` | array | Optional labels such as `private_corpus`, `synthetic`, `adapter_required`, or `no_live_claim`. |

### `corpus`

`corpus` MUST identify all replay inputs without relying on hidden host state.

Required fields:

- `corpus_id`: stable id.
- `profile`: `synthetic`, `private_sanitized`, `generated_public`, or `external_adapter`.
- `items`: array of corpus items.

Each `items[]` entry MUST include:

- `evidence_id`: stable id used by `required_evidence` and `negative_traps`.
- `kind`: `note`, `document`, `trace`, `issue`, `pr`, `runbook`, `decision`, `message`,
  `compiled_page`, or `adapter_state`.
- `text` or `local_ref`: inline sanitized text or a local fixture pointer.
- `source_ref`: object; MAY be `{}` only for generated synthetic fixtures.
- `created_at`: RFC3339 timestamp or `null` when time is intentionally irrelevant.

Optional corpus fields:

- `capture_behaviors`: object used by `capture_integration` jobs and fixture-backed
  suites to classify integration evidence. Supported arrays are `real`,
  `fixture_backed`, `mocked`, `blocked`, `not_encoded`, and `notes`.
  `fixture_backed` means the behavior is represented by checked-in fixture evidence,
  not by a live adapter pass. Reports MUST NOT convert `fixture_backed`, `mocked`,
  `blocked`, or `not_encoded` behavior into a live integration success claim.

Private corpus fixtures MUST use sanitized inline text or local refs excluded from git.
Reports MAY publish evidence ids and score summaries without publishing private text.

### External Adapter Manifest

Real-world reports MAY include an external adapter manifest. When present, the manifest
MUST use this schema id:

```text
elf.real_world_external_adapter_manifest/v1
```

The manifest is the stable adapter-pack contract for comparing external memory projects
against `real_world_job` suites. It records what an adapter actually executed, which
coverage is only fixture-backed or live-baseline-only, and which suites remain blocked,
unsupported, incomplete, or not encoded. It MUST NOT be used to convert retrieval-only
live-baseline evidence into a real-world suite win.

Required manifest fields:

- `manifest_id`: stable ASCII id for the checked-in or generated manifest.
- `docker_isolation`: object describing the default execution boundary.
- `adapters`: array of adapter records.

`docker_isolation` MUST include:

- `default`: boolean; MUST be `true` for repository-supported external adapter runs
  unless a separate issue records why Docker is impossible.
- `compose_file`: Docker Compose file used by the supported runner.
- `runner`: script or command entrypoint used inside the Compose boundary.
- `artifact_dir`: relative artifact directory for logs and reports.
- `host_global_installs_required`: boolean; MUST be `false` for default external
  runs.
- `notes`: optional bounded explanatory strings.

Each `adapters[]` record MUST include:

- `adapter_id`: stable id unique within the manifest.
- `project`: display name such as `qmd`, `agentmemory`, or `mem0/OpenMemory`.
- `adapter_kind`: local execution shape, for example `docker_cli_same_corpus`,
  `docker_sdk_same_corpus`, `offline_fixture_response`, or `research_gate`.
- `evidence_class`: one of `fixture_backed`, `live_baseline_only`,
  `live_real_world`, or `research_gate`.
- `docker_default`: boolean.
- `host_global_installs_required`: boolean.
- `overall_status`: one adapter status from the table below.
- `setup`, `run`, and `result`: evidence objects with `status`, `evidence`, and
  optional `command` and `artifact`.
- `capabilities`: array of capability coverage records with `capability`, `status`,
  and `evidence`.
- `suites`: array of real-world suite coverage records with `suite_id`, `status`, and
  `evidence`.
- `scenarios`: optional array of scenario judgment records with `scenario_id`,
  optional `suite_id`, `status`, `elf_position`, optional `comparison_outcome`,
  `evidence`, and optional `command` and `artifact`. `elf_position` MUST be one of
  `wins`, `ties`, `loses`, or `untested`. `comparison_outcome`, when present, MUST be
  one of `win`, `tie`, `loss`, `not_tested`, `blocked`, or `non_goal`. Reports SHOULD
  derive `comparison_outcome` from `elf_position` when omitted, but SHOULD use the
  explicit field for scenarios where the legacy ELF-relative position is less precise
  than the report outcome. Scenario judgments are report inputs for dimension-level
  comparison; they MUST NOT convert live-baseline-only evidence into real-world suite
  pass claims.
- `evidence`: array of evidence pointers with `kind`, `ref`, and `status`.
- `notes`: optional bounded explanatory strings.
- `follow_up`: optional `title` and `reason`.
- `execution_metadata`: optional object used by expanded adapter packs and research
  gates. When present, it MUST include `sources`, `setup_path`,
  `runtime_boundary`, `resource_expectation`, and `retry_guidance`. It MAY include
  `research_depth`.

`research_gate` evidence class means the adapter record is a checked-in gating record
for future implementation, not a benchmark execution result. It is used when a project
needs D1/D2 research, resource sizing, credentials, Docker runtime proof, or source
mapping before a fair adapter can run. A `research_gate` record MUST NOT be counted as
fixture-backed, live-baseline-only, or live-real-world evidence.

`execution_metadata.sources[]` entries MUST include:

- `label`: short source label.
- `url`: official source, docs, or repository URL.
- `evidence`: bounded description of why the source matters.

`execution_metadata` fields:

- `setup_path`: intended setup path or the setup blocker to resolve.
- `runtime_boundary`: Docker/service/CLI/process boundary expected for safe runs.
- `resource_expectation`: expected resource or credential envelope, including unknowns.
- `retry_guidance`: one or more concrete next checks before claiming pass/fail.
- `research_depth`: optional `D0`, `D1`, or `D2` research state.

Adapter coverage status terms:

| Term | Meaning |
| --- | --- |
| `real` | The adapter capability is exercised through the project's real local API, CLI, storage, or service surface. |
| `mocked` | The adapter uses a mock, in-memory substitute, fixture replay, or other non-durable stand-in for the named capability. |
| `unsupported` | The project or safe Docker profile does not expose the capability. This is not a quality penalty. |
| `blocked` | The check cannot run safely without credentials, manual setup, durable runtime integration, private input, or host integration outside the run scope. |
| `incomplete` | Setup, build, dependency, adapter wiring, parse, or runtime execution did not reach the behavioral check. |
| `wrong_result` | The adapter reached execution but produced the wrong answer, memory, evidence, or action. |
| `lifecycle_fail` | Retrieval may work, but encoded update, delete, expiry, cold-start, persistence, history, or supersession behavior failed. |
| `pass` | The declared adapter check completed and met its encoded expectations. |
| `not_encoded` | The capability, suite, or adapter path is not implemented in the runner, so no pass/fail claim is allowed. |

Reports that load a manifest MUST emit an `external_adapters` section with schema id
`elf.real_world_external_adapter_report/v1`, the manifest id, Docker isolation
metadata, per-adapter records, and summary counters for:

- adapter count, external project count, Docker-default count, host-global-install
  count;
- `fixture_backed`, `live_baseline_only`, `live_real_world`, and `research_gate`
  evidence classes;
- overall adapter statuses;
- capability coverage statuses;
- real-world suite coverage statuses.

For `elf.real_world_external_adapter_report/v1`, `adapter_count` is the number of
adapter records in the loaded manifest. `external_project_count` is the number of
unique non-ELF project names represented by those records, not the number of non-ELF
adapter records. Multiple adapter records for the same external project MUST count as
one external project in this summary.

Adapter-pack issues SHOULD add new projects by appending adapter records to this
manifest shape. They MUST NOT change these status meanings to make a project look
better or worse.

### `timeline`

`timeline` MUST model the user job as prior agent work, not just a bag of documents.

Each event MUST include:

- `event_id`
- `ts`
- `actor`: `user`, `agent`, `tool`, `system`, `operator`, or `external`
- `action`: short verb phrase such as `created_issue`, `made_decision`,
  `ran_command`, `hit_blocker`, `updated_memory`, `deleted_memory`, or
  `published_report`
- `evidence_ids`: one or more ids from `corpus.items[]`
- `summary`: compact English summary

Timeline order is normative. If a later event supersedes an earlier fact, the expected
answer MUST follow the later event unless `allowed_uncertainty` permits a historical
answer.

### `prompt`

`prompt` MUST include:

- `role`: normally `user`.
- `content`: the exact user request.
- `job_mode`: `resume`, `answer`, `debug`, `decide`, `compile`, `personalize`, or
  `operate`.
- `constraints`: array of explicit instructions such as `do_not_run_live_actions`,
  `cite_evidence`, `avoid_repeating_completed_work`, or `state_blockers`.

The evaluated system MAY retrieve memory, inspect its own state, or call adapter tools
only when the runner profile permits those actions.

### `expected_answer`

`expected_answer` MUST define answer correctness at the job level.

Required fields:

- `must_include`: array of claims or actions that must appear.
- `must_not_include`: array of forbidden claims, stale facts, or unsafe actions.
- `evidence_links`: mapping from required claim ids to acceptable evidence ids.
- `answer_type`: `direct_answer`, `work_plan`, `resume_summary`, `debug_report`,
  `decision_record`, `compiled_knowledge`, or `ops_runbook`.

Optional fields:

- `accepted_alternates`: array of alternate phrasings or equivalent evidence ids.
- `requires_caveat`: boolean; when true, omitting the caveat is a scoring failure.
- `requires_refusal`: boolean; when true, the correct answer is to decline or stop
  because the memory system lacks evidence or authority.

### `required_evidence`

Each required evidence entry MUST include:

- `evidence_id`
- `claim_id`
- `requirement`: `cite`, `use`, `avoid`, or `explain`
- `quote` or `selector`: exact quote for inline fixtures, or a stable selector for
  local/private fixtures.

An answer that states a required claim without any acceptable evidence link is an
`unsupported_claim` unless the job's `allowed_uncertainty` explicitly permits an
uncited low-confidence statement.

### Optional `adapter_response.answer.pages`

Knowledge-compilation fixtures MAY include generated page artifacts in
`corpus.adapter_response.answer.pages[]`. These page artifacts are benchmark outputs,
not authoritative source truth. Any checked-in generated page fixture MUST be clearly
marked as a benchmark artifact.

Each page entry MUST include:

- `page_id`: stable page identifier, such as `project:elf-benchmark-suite`.
- `page_type`: `project`, `entity`, `concept`, `issue_timeline`, or another
  fixture-defined type.
- `title`: human-readable page title.
- `path`: optional fixture path for a checked-in benchmark artifact page.
- `sections`: generated page sections.
- `backlinks`: zero or more page, entity, concept, issue, or evidence identifiers.
- `lint_findings`: zero or more stale, unsupported, or contradiction findings.
- `rebuild`: optional rebuild comparison record.

Each `sections[]` entry MUST include:

- `section_id`
- `heading`
- `role`: examples include `current_truth`, `history`, `timeline`, `backlinks`, and
  `summary`.
- `content`: bounded fixture text.
- `evidence_ids`: zero or more ids from `corpus.items[]`.
- `timeline_event_ids`: zero or more ids from `timeline[]`.
- `unsupported_reason`: optional reason why the section is intentionally unsupported.

Every generated page section MUST trace back to at least one `evidence_id` or
`timeline_event_id`, or it MUST include `unsupported_reason`. A section that lacks both
trace evidence and an unsupported flag is an `unsupported_claim`. A section with
`role = "summary"` and `unsupported_reason` is counted as an unsupported summary, but it
is not a hidden unsupported claim because the page explicitly marks the gap.

Each `lint_findings[]` entry SHOULD include:

- `finding_id`
- `finding_type`: for example `stale_claim`, `unsupported_claim`, or
  `contradiction`.
- `severity`
- `text`
- `evidence_ids`
- `trap_id`: optional link to `negative_traps[]`.

Each `rebuild` record SHOULD include:

- `first_hash`
- `second_hash`
- `deterministic`: true when repeat rebuilds produced byte-stable output.
- `allowed_variance`: explanations for accepted non-semantic variance.

Knowledge-compilation reports SHOULD include citation coverage, stale claim detection,
rebuild determinism, page usefulness, backlink counts, unsupported summary count, and
untraced section count. Rebuild results are acceptable only when repeated output is
deterministic enough for regression comparison or every allowed variance is explicitly
reported.

### `negative_traps`

Negative traps MUST be explicit so systems are tested against realistic memory failure
modes.

Trap types:

- `stale_fact`: once true but superseded later in the timeline.
- `near_duplicate`: semantically close but wrong project, user, tenant, or time.
- `decoy_evidence`: shares query terms but does not support the expected claim.
- `unsafe_action`: would perform live, destructive, credentialed, or out-of-scope work.
- `unsupported_prior`: plausible prior decision not present in the corpus.
- `privacy_leak`: private or excluded content that must not appear in the answer.

Each trap MUST include `trap_id`, `type`, `evidence_ids`, and `failure_if_used`.

### `encoding`

`encoding` declares a fixture that is intentionally not scored as a runnable pass
because the benchmark capability is not encoded or cannot run yet.

Allowed `status` values:

- `not_encoded`: the fixture documents a capability gap and must not claim pass.
- `blocked`: required adapter, corpus, or system support is missing.
- `incomplete`: fixture execution cannot reach a complete scored state.

When `status` is present, `reason` MUST be a non-empty explanation. `follow_up` is
optional, but when present it MUST include non-empty `title` and `reason` fields.

### `memory_evolution`

`memory_evolution` is used by jobs that test whether an answer distinguishes current
facts, historical facts, stale facts, conflicts, corrected memories, and missing
temporal validity support.

Fields:

- `current_evidence_ids`: evidence ids that support the current answer.
- `historical_evidence_ids`: evidence ids that are historically true but not current
  answers unless the prompt asks for history.
- `stale_trap_ids`: negative trap ids that represent stale answers.
- `conflicts`: array of conflicts with `conflict_id`, `claim_id`,
  `current_evidence_id`, `historical_evidence_id`, and optional
  `resolved_by_evidence_id`.
- `update_rationale`: optional object with `claim_id`, `evidence_ids`, and
  `available` to show whether the answer can explain why the memory changed.
- `temporal_validity`: optional object with `required`, `encoded`, and optional
  `follow_up`. When `required = true` and `encoded = false`, the job MUST declare
  `encoding.status = "not_encoded"` or `encoding.status = "blocked"`.
  When `encoded = true`, the job is scored normally and must include concrete
  produced evidence for current and historical validity behavior.

### `operator_debug`

`operator_debug` is required when `suite = "operator_debugging_ux"` and optional
elsewhere. It records whether a human operator can identify the root cause through
viewer, trace, or CLI readback without raw SQL.

Required fields:

- `failure_mode`: stable label such as `expected_evidence_dropped`,
  `rerank_promoted_bad_candidate`, `provider_latency_or_failure`,
  `rebuild_changed_results`, or `relation_context_misled_search`.
- `trace_id`: trace handle when available.
- `viewer_url`: read-only viewer path that opens the trace evidence when available.
- `admin_trace_bundle_url`: direct admin trace bundle path when available.
- `root_cause`: concise expected diagnosis.
- `steps_to_root_cause`: number of viewer or CLI steps needed to reach the diagnosis.
- `raw_sql_needed`: must be `false` for a pass under this suite.
- `dropped_candidate_visibility`: whether dropped, retained, or misleading candidates
  are visible through trace/viewer evidence.
- `trace_completeness`: `complete`, `partial`, or `missing`.
- `repair_action_clarity`: `clear`, `partial`, or `missing`.
- `viewer_panels`: viewer panels used, such as `Replay Candidates`, `Stage Details`,
  `Providers And Ranking`, or `Relation Context`.
- `cli_steps`: equivalent CLI or endpoint steps.
- `trace_evidence`: evidence ids used for the diagnosis.
- `ux_gaps`: array of focused follow-up pointers when a needed panel or endpoint is
  absent.

Each `ux_gaps[]` entry MUST include `gap_id`, `severity`, `description`, and
`follow_up_issue`. If a fixture requires a missing panel, the report must encode the
gap instead of hiding it behind a wrong-result score.

### `scoring_rubric`

The rubric MUST be job-specific but use the shared dimensions below.

Required dimensions:

- `answer_correctness`: expected answer content and action selection.
- `evidence_grounding`: correct use of required evidence and source refs.
- `trap_avoidance`: avoidance of stale, decoy, privacy, and unsafe traps.
- `uncertainty_handling`: honest caveats when evidence is missing or ambiguous.
- `workflow_helpfulness`: whether the answer advances the user job without needless
  repetition.

Optional dimensions:

- `lifecycle_behavior`: update, delete, expiry, supersession, or cold-start behavior.
- `debuggability`: trace, timeline, viewer, or explanation quality.
- `latency_resource`: bounded runtime, cost proxy, or resource envelope.
- `personalization_fit`: correct user/project preference application without leakage.

Rubric fields:

- `dimensions`: object keyed by dimension name, each with `weight`, `max_points`, and
  `criteria`.
- `pass_threshold`: total normalized score required for `pass`.
- `hard_fail_rules`: array of rules that force a non-pass status regardless of score.

Hard-fail rules MUST include:

- unsupported high-confidence claim about a required decision or fact;
- unsafe live/destructive action when the prompt forbids it;
- use of a negative trap marked `failure_if_used = true`;
- missing required refusal when the job has `requires_refusal = true`.

### `allowed_uncertainty`

`allowed_uncertainty` MUST distinguish honest uncertainty from failure.

Required fields:

- `can_answer_unknown`: boolean.
- `acceptable_phrases`: array of accepted uncertainty phrases or patterns.
- `fallback_action`: `ask_for_evidence`, `state_blocker`, `cite_partial_evidence`,
  `refuse`, or `continue_with_caveat`.

If `can_answer_unknown = false`, an answer that refuses despite sufficient evidence is
`wrong_result`. If `can_answer_unknown = true`, an answer that invents missing evidence
is `unsupported_claim`.

## Suite Taxonomy

Suite ids are stable public names. Each suite MUST contain at least one
`real_world_job` before a report may claim suite coverage.

| Suite id | Goal | User-job examples | Evidence requirements | Scoring dimensions | Strongest external references |
| --- | --- | --- | --- | --- | --- |
| `trust_source_of_truth` | Verify authoritative storage, provenance, rebuild, and non-authoritative derived index handling. | Restore a note after Qdrant rebuild; identify whether a compiled page is derived; explain why a source ref supports a claim. | Source note/document ids, restore or rebuild trace, source_ref lineage, no hidden index-only evidence. | answer_correctness, evidence_grounding, trap_avoidance, lifecycle_behavior. | ELF, memsearch, OpenViking. |
| `work_resume` | Help an agent resume real work without repeating completed steps or losing blockers. | Resume a retained lane; identify next command after a failed run; summarize what remains blocked. | Timeline events, issue/PR ids, run summaries, latest blocker evidence. | answer_correctness, workflow_helpfulness, uncertainty_handling, trap_avoidance. | agentmemory, claude-mem, OpenViking. |
| `project_decisions` | Recover durable decisions, rationale, reversals, and current policy. | Explain why a design was chosen; distinguish old vs current validation gate; cite decision evidence. | Decision records, superseding events, accepted alternatives, current-policy timestamp. | answer_correctness, evidence_grounding, trap_avoidance, uncertainty_handling. | ELF, gbrain, llm-wiki, Letta. |
| `retrieval` | Measure task-relevant retrieval quality beyond top-k keyword matching. | Answer a task query with expected evidence; find alternate phrasing; avoid near-duplicate project evidence. | Expected evidence ids, allowed alternates, decoy evidence ids, trace ids when available. | answer_correctness, evidence_grounding, trap_avoidance, latency_resource. | qmd, ELF, memsearch, OpenViking. |
| `memory_evolution` | Verify updates, deletes, expiry, supersession, contradiction handling, and history. | Apply a new preference; suppress a deleted memory; explain what superseded an old fact. | Before/after memory versions, ingest decision rows or adapter history, current timeline event. | lifecycle_behavior, answer_correctness, evidence_grounding, trap_avoidance. | mem0, ELF, Graphiti/Zep, Letta. |
| `consolidation` | Test reviewable derived memory formation without hidden source mutation. | Produce a consolidation proposal; identify unsupported claims; discard stale synthesis. | Source inputs, derived proposal id, lineage, review state, conflict markers. | answer_correctness, evidence_grounding, uncertainty_handling, debuggability. | Claude Dreams, Gemini CLI Auto Memory, Always-On Memory Agent, ELF. |
| `knowledge_compilation` | Compile evidence into maintained project/entity/concept pages while preserving provenance. | Build a project status page; answer from compiled truth plus timeline; lint a stale page section. | Page section sources, backlinks, timeline entries, lint evidence. | answer_correctness, evidence_grounding, workflow_helpfulness, trap_avoidance. | llm-wiki, gbrain, graphify, ELF. |
| `operator_debugging_ux` | Show whether a wrong or ambiguous memory result can be debugged without raw store spelunking. | Explain why a result ranked first; inspect a trace; identify which stage dropped expected evidence. | Trace bundle, retrieval trajectory, candidate metrics, viewer or CLI readback. | debuggability, evidence_grounding, workflow_helpfulness, answer_correctness. | claude-mem, qmd, agentmemory, ELF. |
| `capture_integration` | Evaluate how accurately work observations become usable memory across agents and tools. | Capture a session decision; exclude private spans; import external agent observations. | Hook/import logs, write policy audits, excluded spans, resulting note ids. | answer_correctness, evidence_grounding, trap_avoidance, lifecycle_behavior. | agentmemory, claude-mem, memsearch, mem0. |
| `production_ops` | Prove safe operation under backup, restore, backfill, cold start, resource, and credential boundaries. | Resume interrupted import; restore from backup; report missing private manifest as bounded caveat. | Command/report artifacts, resource envelope, checkpoint state, failure guard evidence. | lifecycle_behavior, latency_resource, uncertainty_handling, evidence_grounding. | ELF, qmd, memsearch, LangGraph. |
| `personalization` | Apply user/project preferences correctly without leaking across scopes or overfitting stale preferences. | Remember preferred response style; avoid using another project tenant's note; update a preference. | Scoped memory ids, preference versions, tenant/project/agent context, negative cross-scope traps. | personalization_fit, trap_avoidance, evidence_grounding, answer_correctness. | mem0, Letta, agentmemory, ELF. |

## Report Semantics

Reports MUST preserve typed outcomes at job, suite, and project levels. A report MUST
NOT collapse the results into a single overall leaderboard without the underlying typed
state table.

Outcome terms:

| Term | Meaning |
| --- | --- |
| `pass` | The job or suite is encoded, ran to completion, met the pass threshold, satisfied required evidence, and hit no hard-fail rule. |
| `wrong_result` | The system completed the job but selected the wrong answer, wrong action, wrong current fact, or missed required evidence despite enough available evidence. |
| `lifecycle_fail` | The answer surface may be correct for retrieval, but encoded update, delete, expiry, cold-start, persistence, history, or supersession behavior failed. |
| `incomplete` | The runner could not reach the behavioral check because install, build, dependency, adapter wiring, parse, or runtime setup failed. |
| `blocked` | The check cannot be run safely without credentials, manual setup, private corpus input, durable runtime integration, or host integration outside the run scope. |
| `not_encoded` | The suite, job, adapter path, or scoring dimension is not implemented in the runner, so no pass/fail claim is allowed. |
| `unsupported_claim` | The system produced a substantive claim, decision, evidence citation, or capability claim that is not supported by the job corpus, required evidence, or report metadata. |

`unsupported_claim` is distinct from `wrong_result`: `wrong_result` can be a supported
but incorrect selection, while `unsupported_claim` is an evidentiary failure. When both
apply, reports SHOULD surface `unsupported_claim` because it is higher risk for memory
systems used by agents.

Suite status rules:

- A suite is `pass` only when all encoded required jobs pass.
- A suite is `lifecycle_fail` when at least one lifecycle-scored job proves lifecycle
  behavior wrong and no higher-risk `unsupported_claim` is present.
- A suite is `wrong_result` when at least one required job returns the wrong result and
  no higher-risk `unsupported_claim` is present.
- A suite is `unsupported_claim` when any hard-fail unsupported claim occurs.
- A suite is `incomplete` or `blocked` when required jobs cannot run for those reasons.
- A suite is `not_encoded` when no job in that suite is implemented, or when an
  encoded fixture declares a job-level capability gap that prevents a suite pass claim.

Reports MUST include:

- run id, runner version, corpus profile, job ids, suite ids, project adapter metadata;
- per-job status, normalized score, hard-fail hits, evidence ids used, trap ids used;
- per-job `answer_type`, required caveat/refusal flags, and whether an unknown answer
  is allowed, so current-decision, historical-decision, rationale, and caveat cases are
  distinguishable in generated reports;
- expected evidence recall and irrelevant context ratio at job, suite, and summary
  levels when the runner can derive them from fixture evidence ids;
- trace explainability metadata when an adapter or fixture can identify retrieval
  stages, especially for wrong-result stage attribution;
- per-suite typed status and score distribution;
- unsupported claim list with claim text or a bounded redacted description;
- for encoded knowledge-compilation jobs with page artifacts: citation coverage, stale
  claim detection, rebuild determinism, page usefulness, backlink counts, unsupported
  summary count, and untraced section count;
- explicit `not_encoded` suite list;
- private-corpus redaction policy when private fixtures are used.
- capture/integration coverage classes when any fixture declares `capture_behaviors`,
  preserving the `real`, `fixture_backed`, `mocked`, `blocked`, and `not_encoded`
  distinction.
- external adapter coverage when an external adapter manifest is loaded, preserving
  `fixture_backed`, `live_baseline_only`, `live_real_world`, `research_gate`,
  `real`, `mocked`, `unsupported`, `blocked`, `incomplete`, `wrong_result`,
  `lifecycle_fail`, `pass`, and `not_encoded` distinctions. Scenario summaries MUST
  preserve status counts, legacy `elf_position` counts, and normalized
  `comparison_outcome` counts when scenario judgments are present.

Reports that encode `memory_evolution` jobs SHOULD also include stale-answer counts,
conflict detection counts, update rationale availability, and temporal-validity
`not_encoded` counts. A temporal graph validity job MUST NOT be reported as `pass`
unless the runner can evaluate current-only versus historical relation facts.

Consolidation suite reports MUST also include:

- proposal usefulness score, or `null` when the job has no proposal payloads;
- lineage completeness score over expected source refs;
- review action correctness for `apply`, `discard`, and `defer` outcomes;
- proposal unsupported-claim count for contradiction/staleness reports;
- source mutation count.

For proposal-only consolidation jobs, source mutation count MUST be `0`. If the runner
or adapter cannot execute the consolidation primitive it claims to evaluate, the report
MUST include an executable gap with a precise follow-up issue or issue title. Offline
fixtures MAY still pass when they verify checked-in proposal payloads and clearly avoid
claiming scheduled provider-backed generation.

## Claim Rules

- A project MAY claim a suite pass only for suites with encoded jobs and a published
  report using this contract.
- A project MUST NOT use generated public jobs to claim private production readiness.
- A project MUST NOT treat `blocked`, `incomplete`, or `not_encoded` as evidence of
  weakness or strength; those states only describe benchmark coverage.
- A project MUST NOT claim "best memory system" from this suite. Reports SHOULD describe
  dimension-specific results and typed limitations.
- Existing ELF/qmd-leading live baseline results MAY be cited as retrieval/lifecycle
  evidence, but MUST NOT be reinterpreted as real-world job suite wins.

## Downstream Implementation Contract

Runner implementation issues can cite this spec and choose any subset of suites. The
minimum useful runner increment is:

- one encoded `real_world_job` fixture;
- one adapter path;
- scoring for all required rubric dimensions in that job;
- typed report output using the Report Semantics section.

Implementation issues MUST state which suites remain `not_encoded`.
