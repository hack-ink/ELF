---
type: Evidence
title: "mem0/OpenMemory History and UI Export Report - June 11, 2026"
description: "Checked-in benchmark evidence record: mem0/OpenMemory History and UI Export Report - June 11, 2026."
resource: docs/evidence/benchmarking/2026-06-11-mem0-openmemory-history-ui-export-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-18
tags:
  - docs
  - evidence
  - benchmarking
---
# mem0/OpenMemory History and UI Export Report - June 11, 2026

Goal: Add scenario-level mem0/OpenMemory history, personalization, deletion-audit,
local SDK export-readback, and bounded OpenMemory export-helper setup evidence without
promoting basic lifecycle smoke into UI or hosted Platform claims.
Read this when: You need the current XY-924 comparison between ELF and
mem0/OpenMemory for entity-scoped history, preference correction, deletion audit,
personalization, OpenMemory inspection/export, hosted Platform export, or optional
graph memory.
Inputs: Fresh scoped mem0 Docker baseline run, refreshed real-world external adapter
manifest, generated real-world memory report, and the June 11 first-generation,
temporal/history, and competitor-strength reports.
Depends on: `docs/spec/real_world_agent_memory_benchmark_v1.md`,
`scripts/live-baseline-benchmark.sh`, and
`apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`.
Outputs: Per-scenario outcomes using `win`, `tie`, `loss`, `not_tested`, `blocked`,
and `non_goal`, plus command and artifact evidence for each measured claim.
Markdown report owner: `docs/evidence/benchmarking/2026-06-11-mem0-openmemory-history-ui-export-report.md`.

## Executive Judgment

The XY-924 objective is now encoded for the reproducible local OSS SDK surface, and
XY-931 adds a separate bounded OpenMemory export-helper setup probe.

mem0/OpenMemory now has fresh local OSS evidence for behavior beyond the basic
lifecycle smoke:

- `preference_correction_history`: `pass`
- `entity_scoped_personalization`: `pass`
- `local_get_all_export_readback`: `pass`
- `delete_history_audit_readback`: `pass`
- `openmemory_ui_export_readback`: `blocked`

The comparison is intentionally narrower than a hosted/OpenMemory product verdict.
The local run measures the mem0 OSS SDK and local FastEmbed/Qdrant/history paths in
Docker. The new product-UX setup probe detects the OpenMemory tree, UI package,
compose file, and export helper, then records a setup blocker: the export helper needs
Docker access to a running OpenMemory product container, while the baseline runner
only has the SDK Qdrant/history artifacts. It does not claim browser/dashboard
readback, hosted mem0 Platform export jobs, or optional graph memory.

## Fresh Evidence

| Command | Result | Runtime | Artifact |
| --- | --- | ---: | --- |
| `cargo make openmemory-ui-export-readback` | `pass` for SDK baseline; OpenMemory export-helper setup probe `blocked` with `DOCKER_UNAVAILABLE_IN_BASELINE_RUNNER` | 35.14 seconds wall; 33 seconds project runtime | `tmp/live-baseline/live-baseline-report.json`, `tmp/live-baseline/mem0-checks.json`, `tmp/live-baseline/mem0-openmemory-ui-export.json`, `tmp/live-baseline/mem0-openmemory-export-attempt.log` |
| `cargo make real-world-memory` | `pass`; refreshed external adapter report published | 7.97 seconds | `tmp/real-world-memory/real-world-memory-report.json`, `tmp/real-world-memory/real-world-memory-report.md` |

Fresh mem0/OpenMemory run id: `live-baseline-20260611122416`.

Generated external adapter summary for all external adapter manifest rows:

- Scenario statuses: `unsupported=2`, `blocked=2`, `wrong_result=1`,
  `lifecycle_fail=1`, `pass=9`, `not_encoded=3`.
- Legacy ELF positions: `wins=2`, `ties=4`, `loses=1`, `untested=11`.
- Normalized comparison outcomes: `win=2`, `tie=4`, `loss=1`,
  `not_tested=8`, `blocked=1`, `non_goal=2`.

mem0/OpenMemory rows in this report contain eight scenarios: `loss=1`,
`tie=3`, `not_tested=1`, `blocked=1`, and `non_goal=2`.

## Scenario Outcomes

| Scenario | mem0/OpenMemory evidence | ELF comparison outcome | Status | Command | Artifact |
| --- | --- | --- | --- | --- | --- |
| Basic local lifecycle | mem0 passes same-corpus retrieval, update, delete, and cold-start reload in the prior first-generation baseline. | `tie` | `pass` | `ELF_BASELINE_PROJECTS=ELF,agentmemory,mem0,memsearch,claude-mem cargo make baseline-live-docker` | `tmp/live-baseline/live-baseline-report.json` |
| Preference correction history | `Memory.history` exposes explicit `ADD` and `UPDATE` preference records; search returns only the current correction. | `loss` | `pass` | mem0: `ELF_BASELINE_PROJECTS=mem0 cargo make baseline-live-docker`; ELF: `cargo make real-world-memory-live-adapters` | mem0: `tmp/live-baseline/mem0-checks.json`; ELF: `tmp/real-world-memory/live-adapters/`, `docs/evidence/benchmarking/2026-06-11-temporal-history-competitor-gap-report.md` |
| Entity-scoped personalization | `search()` with `user_id`, `agent_id`, and `run_id` filters returns the ELF-scoped preference and omits a PubFi-scoped preference. | `tie` | `pass` | mem0: `ELF_BASELINE_PROJECTS=mem0 cargo make baseline-live-docker`; ELF: `cargo make real-world-memory-live-adapters` | mem0: `tmp/live-baseline/mem0-checks.json`; ELF: `tmp/real-world-memory/live-adapters/`, `docs/evidence/benchmarking/2026-06-11-competitor-strength-adoption-report.md` |
| Delete audit readback | `Memory.history` exposes a `DELETE` event and post-delete search suppresses the deleted memory. | `tie` | `pass` | mem0: `ELF_BASELINE_PROJECTS=mem0 cargo make baseline-live-docker`; ELF: `cargo make real-world-memory-live-adapters` | mem0: `tmp/live-baseline/mem0-checks.json`; ELF: `tmp/real-world-memory/live-adapters/`, `docs/evidence/benchmarking/2026-06-11-temporal-history-competitor-gap-report.md` |
| Local SDK export-style readback | `Memory.get_all` returns the current scoped preference and omits the other scope. | `not_tested` | `pass` | `ELF_BASELINE_PROJECTS=mem0 cargo make baseline-live-docker` | `tmp/live-baseline/mem0-checks.json` |
| OpenMemory UI/export readback | The bounded export-helper setup probe finds OpenMemory product files but the export helper cannot run because Docker is unavailable inside the baseline runner. It does not reach browser/dashboard readback or same-corpus product app database validation. | `blocked` | `blocked` | `cargo make openmemory-ui-export-readback` | `tmp/live-baseline/mem0-openmemory-ui-export.json`, `tmp/live-baseline/mem0-openmemory-export-attempt.log` |
| Hosted mem0 Platform export | Hosted Platform export is outside local OSS evidence. | `non_goal` | `unsupported` | Not run; local OSS comparison only. | `apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json` |
| Optional graph memory | Graph memory is not enabled in the default local OSS run. | `non_goal` | `not_encoded` | Not run; opt-in scenario gate. | `apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json` |

## Evidence Details

The fresh mem0 check artifact records eight passing checks:

- `same_corpus_retrieval`
- `update_replaces_note_text`
- `preference_correction_history`
- `entity_scoped_personalization`
- `local_get_all_export_readback`
- `delete_suppresses_retrieval`
- `delete_history_audit_readback`
- `cold_start_recovery_search`

The `preference_correction_history` check verifies all of:

- history is available;
- history contains the original preference;
- history contains the corrected preference;
- history contains explicit `ADD` and `UPDATE` events;
- search contains the corrected preference;
- search omits the old preference.

The `delete_history_audit_readback` check verifies all of:

- history is available;
- history contains a delete event;
- search suppresses the deleted memory.

The local SDK export-style readback check is intentionally named separately from UI
export. It only proves local `get_all` scoped readback through the OSS SDK.

The OpenMemory export-helper setup probe records:

- OpenMemory tree present: `true`;
- UI package present: `true`;
- compose file present: `true`;
- export helper present: `true`;
- sunsetting notice present: `true`;
- SDK `get_all` status: `pass`;
- export attempt command:
  `timeout 30 bash openmemory/backup-scripts/export_openmemory.sh --user-id elf-history-user --container openmemory-openmemory-mcp-1`;
- export attempt exit code: `1`;
- reason code: `DOCKER_UNAVAILABLE_IN_BASELINE_RUNNER`.

The attempt log contains `docker: command not found` before the helper reports that
`openmemory-openmemory-mcp-1` is not running. The concrete next action is to add a
dedicated OpenMemory Docker Compose profile that imports the generated mem0 corpus
into the OpenMemory app database, starts API/UI with explicit local or provider
configuration, then reruns the export helper and validates exported memories.

## Source And Product Boundary

Official mem0 documentation distinguishes the OSS/self-hosted surface from hosted
Platform API paths. The OSS REST page documents CRUD/search/update/delete/reset
operations by `user_id`, `agent_id`, or `run_id`, an OpenAPI explorer at `/docs`, and
memory history endpoints. The export guide distinguishes bulk `get_all()`, semantic
search, structured exports, and Platform UI exports.

This report uses those docs only to set the claim boundary:

- local OSS SDK `history`, `search`, and `get_all` behavior is measurable here;
- OpenMemory browser/dashboard export is not reached here; the current evidence is a
  bounded export-helper setup probe blocked by setup;
- hosted Platform export is a `non_goal` for this local OSS lane;
- optional graph memory remains an opt-in scenario, not a default pass/fail claim.

References:

- Mem0 OSS REST API Server: `https://docs.mem0.ai/open-source/features/rest-api`
- Mem0 Export Stored Memories: `https://docs.mem0.ai/cookbooks/essentials/exporting-memories`

## Claim Boundaries

Allowed:

- mem0/OpenMemory local OSS passes the new encoded history, correction,
  personalization, deletion-audit, and local `get_all` readback checks in run
  `live-baseline-20260611122416`.
- ELF currently has a measured `loss` against mem0 on the preference correction
  history dimension because the June 11 temporal/history report records ELF's live
  memory-evolution preference job as `wrong_result`.
- ELF and mem0 currently `tie` on the encoded entity-scoped personalization and
  delete-audit surfaces.
- OpenMemory UI/export readback is `blocked` by a concrete setup blocker:
  `DOCKER_UNAVAILABLE_IN_BASELINE_RUNNER`; ELF cannot compare against this product-UX
  scenario yet.
- Hosted mem0 Platform export and optional graph memory are `non_goal` for this
  local OSS comparison.

Not allowed:

- Do not reuse the basic lifecycle pass as history, UI, hosted, or graph-memory
  evidence.
- Do not claim OpenMemory UI/export quality from local SDK `get_all`.
- Do not claim hosted mem0 Platform behavior from the local OSS run.
- Do not treat optional graph memory as a default mem0 pass or ELF loss.
- Do not convert `blocked`, `unsupported`, `not_encoded`, or `non_goal` scenarios
  into wins or losses.

## Follow-Up Gate

The next fair UI/export comparison requires extending the bounded runner so it starts
OpenMemory, loads the same local memories into the OpenMemory app database, captures
authenticated inspection/export readback, and publishes a browser/API artifact. That
is separate from the local SDK `get_all` export-style readback added here.
