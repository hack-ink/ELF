# First-Generation OSS Adapter Promotion Report - June 11, 2026

Goal: Promote first-generation OSS memory baselines into scenario-level adapter
evidence without converting live-baseline-only runs into real-world suite wins.
Read this when: You need the current XY-898 status for agentmemory, mem0/OpenMemory,
memsearch, and claude-mem scenario evidence.
Inputs: Fresh scoped Docker baseline run, updated external adapter manifest, and the
June 11 temporal/history competitor-gap report.
Outputs: Scenario judgments, ELF win/tie/loss/untested positions, and next adapter
gates.

## Scope Boundary

This is benchmark/report evidence only. No ELF retrieval, ranking, memory-quality, or
service behavior optimization is implemented here.

The updated external adapter manifest now includes scenario-level judgments for the
first-generation OSS memory projects. These judgments are intentionally narrower than
suite passes:

- `live_baseline_only` pass evidence proves the encoded Docker same-corpus or
  lifecycle smoke for that project.
- It does not prove `real_world_job` suite parity unless a project adapter actually
  executes real-world prompts and scoring.
- Hosted mem0 Platform behavior, OpenMemory UI, host-global hooks, and
  operator-owned credentials remain out of scope for local OSS evidence.

## Fresh Run

| Command | Result | Runtime | Artifact |
| --- | --- | ---: | --- |
| `ELF_BASELINE_PROJECTS=agentmemory,mem0,memsearch,claude-mem cargo make baseline-live-docker` | fail with typed non-pass projects | 237.29 seconds | `tmp/live-baseline/live-baseline-report.json` |

The aggregate failed because two projects remained typed non-pass, not because setup
collapsed:

| Project | Status | Retrieval | Checks | Scenario meaning |
| --- | --- | --- | ---: | --- |
| agentmemory | `lifecycle_fail` | `retrieval_pass` | `2/4` pass, `1` lifecycle_fail, `1` blocked | Same-corpus retrieval runs, but update supersession and durable cold-start are not proven through the in-memory mock. |
| mem0/OpenMemory | `pass` | `retrieval_pass` | `4/4` pass | Basic local OSS same-corpus, update, delete, and cold-start smoke passes. |
| memsearch | `pass` | `retrieval_pass` | `4/4` pass | Canonical Markdown reindex/update/delete/reload smoke passes. |
| claude-mem | `wrong_result` | `retrieval_wrong_result` | `4/5` pass | Durable repository lifecycle, detail hydration, and reload pass, but same-corpus retrieval misses expected evidence. |

## Scenario Judgments

| Project | Scenario | Status | ELF position | Evidence boundary |
| --- | --- | --- | --- | --- |
| agentmemory | basic same-corpus retrieval | `pass` | `untested` | Baseline retrieval passes through an in-memory mock; no durable continuity claim. |
| agentmemory | durable update/reload lifecycle | `lifecycle_fail` | `wins` | Update supersession fails and cold-start is blocked; ELF has broader encoded local lifecycle proof. |
| agentmemory | work-resume capture continuity | `blocked` | `untested` | Needs a durable local session/capture path before fair scoring. |
| mem0/OpenMemory | basic local lifecycle | `pass` | `ties` | ELF and mem0 both pass the encoded local lifecycle smoke; mem0 is no longer a basic-smoke failure. |
| mem0/OpenMemory | preference/entity history | `not_encoded` | `untested` | History, correction chains, entity scope, and deletion audit are not scored. |
| mem0/OpenMemory | OpenMemory UI/export readback | `not_encoded` | `untested` | Local OSS UI/export readback is not executed; hosted behavior remains out of scope. |
| memsearch | canonical Markdown reindex/reload | `pass` | `ties` | Baseline reindex/update/delete/reload passes over the canonical file store. |
| memsearch | TTL/expiry lifecycle | `unsupported` | `wins` | The encoded CLI path has reindex/delete but no TTL/expiry behavior. |
| memsearch | real-world prompt adapter | `not_encoded` | `untested` | No memsearch real_world_job prompt adapter is encoded. |
| claude-mem | same-corpus retrieval | `wrong_result` | `wins` | The durable repository path runs but misses expected retrieval evidence. |
| claude-mem | repository lifecycle reload | `pass` | `ties` | Update, delete, and cold-start reload pass over Docker-local SQLite. |
| claude-mem | progressive-disclosure detail hydration | `pass` | `untested` | Search-to-detail/source hydration passes, but ELF has no directly comparable claude-mem-style progressive-disclosure scenario here. |
| claude-mem | hook capture viewer workflow | `not_encoded` | `untested` | Hooks, viewer, timeline, and observations are not executed. |

Summary: 13 scenario judgments: 5 `pass`, 1 `wrong_result`, 1 `lifecycle_fail`,
1 `blocked`, 1 `unsupported`, and 4 `not_encoded`. ELF positions are 3 `wins`,
3 `ties`, 0 `loses`, and 7 `untested`.

## Manifest And Report Changes

The external adapter manifest is now
`real-world-memory-project-adapters-2026-06-11` and includes `scenarios[]` records
with:

- `scenario_id`
- optional `suite_id`
- typed scenario `status`
- `elf_position`: `wins`, `ties`, `loses`, or `untested`
- evidence text plus optional command/artifact pointers

`real_world_job_benchmark` now preserves these fields in generated reports and
renders an **Adapter Scenario Judgments** table. This makes the report input capable
of saying whether ELF wins, ties, loses, or remains untested per scenario without
changing the real-world suite status rules.

## Claim Boundaries

Allowed:

- mem0/OpenMemory passes the current basic local OSS lifecycle smoke.
- memsearch passes the current canonical Markdown reindex/reload smoke.
- agentmemory remains non-pass for durable lifecycle because the current adapter uses
  an in-memory mock and cannot prove cold-start recovery.
- claude-mem remains wrong-result for same-corpus retrieval while preserving useful
  passed evidence for repository lifecycle and detail hydration.

Not allowed:

- Do not claim hosted OpenMemory behavior from local OSS evidence.
- Do not claim mem0/OpenMemory history, UI/export, hosted, or graph-memory parity.
- Do not claim memsearch source-of-truth real-world suite parity from baseline smoke.
- Do not claim claude-mem hook/viewer/capture parity from repository-only checks.
- Do not collapse `wrong_result`, `lifecycle_fail`, `blocked`, `unsupported`,
  `not_encoded`, and `incomplete` into one generic failure bucket.

## Next Gates

- agentmemory: select a durable local KV/index/session path before work-resume and
  capture jobs.
- mem0/OpenMemory: encode preference/entity history, deletion audit, UI/export
  readback, and optional graph memory for local OSS only.
- memsearch: encode real-world source-of-truth and retrieval-debug prompt jobs over
  the canonical Markdown store.
- claude-mem: fix or explain same-corpus retrieval misses, then encode hook capture,
  viewer/operator, and progressive-disclosure jobs.
