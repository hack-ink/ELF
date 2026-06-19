---
type: Evidence
title: "OpenMemory UI/Export Product Readback Report - June 19, 2026"
description: "Checked-in benchmark evidence record: OpenMemory UI/Export Product Readback Report - June 19, 2026."
resource: docs/evidence/benchmarking/2026-06-19-openmemory-ui-export-product-readback-report.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-19
tags:
  - docs
  - evidence
  - benchmarking
---
# OpenMemory UI/Export Product Readback Report - June 19, 2026

Goal: Recheck OpenMemory UI/export readback after the earlier setup blocker and
publish a fresh typed product-readback boundary if a local product runner still
cannot validate same-corpus OpenMemory export.
Read this when: You need to know whether XY-987 removed the OpenMemory UI/export
blocker, whether mem0 SDK `get_all` can be used as UI/export evidence, or what setup
work remains before an ELF/OpenMemory product-UX comparison is allowed.
Inputs:
`apps/elf-eval/fixtures/report_snapshots/2026-06-19-openmemory-ui-export-product-readback-report.json`,
`tmp/live-baseline/mem0-openmemory-ui-export.json`,
`tmp/live-baseline/mem0-openmemory-export-attempt.log`,
and `docs/evidence/benchmarking/2026-06-11-mem0-openmemory-history-ui-export-report.md`.
Outputs: A fresh command run, a JSON companion, an attempt-log artifact path, and a
scenario-level improved/unchanged/blocked judgment.

## Executive Judgment

The OpenMemory UI/export product-readback status is unchanged: still blocked.

`cargo make openmemory-ui-export-readback` completed successfully as a benchmark
command and refreshed the mem0 local OSS SDK baseline:

- mem0 SDK checks: 8 pass, 0 fail.
- SDK `get_all` export-style readback: pass.
- OpenMemory UI/export product readback: blocked.
- Reason code: `DOCKER_UNAVAILABLE_IN_BASELINE_RUNNER`.
- Fresh run id: `live-baseline-20260619065543`.

This improves freshness and auditability, not competitive status. The OpenMemory
product tree, UI package, compose file, and export helper are present, but the export
helper requires Docker access to a running OpenMemory product container from inside
the baseline runner. The attempt still fails before browser/dashboard readback or
same-corpus product app database validation is reached.

## Command Evidence

| Command | Result | Runtime | Artifact |
| --- | --- | ---: | --- |
| `cargo make openmemory-ui-export-readback` | command pass; OpenMemory probe `blocked` | 78.02 seconds | `tmp/live-baseline/live-baseline-report.json`, `tmp/live-baseline/mem0-openmemory-ui-export.json`, `tmp/live-baseline/mem0-openmemory-export-attempt.log` |

The probe command was:

`timeout 30 bash openmemory/backup-scripts/export_openmemory.sh --user-id elf-history-user --container openmemory-openmemory-mcp-1`

The attempt log records:

```text
openmemory/backup-scripts/export_openmemory.sh: line 52: docker: command not found
ERROR: Container 'openmemory-openmemory-mcp-1' not found/running. Pass --container <NAME_OR_ID> if different.
```

## Product Surface Readback

| Surface | Status |
| --- | --- |
| OpenMemory tree present | `true` |
| UI package present | `true` |
| Compose file present | `true` |
| Export helper present | `true` |
| Sunsetting notice present | `true` |
| Requires OpenAI API key path | `true` |
| Requires Docker Compose | `true` |
| Export helper requires running container | `true` |
| Product browser/dashboard readback reached | `false` |

## Improvement/Regression Readback

- Improved: there is now a fresh June 19 command run, JSON companion, and attempt log
  for the OpenMemory product-readback blocker.
- Unchanged: OpenMemory UI/export remains blocked before same-corpus product app
  database validation.
- Unchanged: mem0 local OSS SDK history and local `get_all` readback remain separate
  passing evidence. They are not UI/export product evidence.
- No regression: the command still preserves the SDK/product boundary and does not
  convert a setup blocker into an ELF win or loss.

## Claim Boundaries

Allowed:

- mem0 local OSS SDK checks and SDK `get_all` readback pass in the fresh run.
- OpenMemory UI/export product readback remains blocked with a concrete command,
  artifact path, and setup error.
- The June 19 recheck is unchanged versus the June 11 XY-931 setup blocker except
  for freshness and checked-in evidence.

Not allowed:

- Do not claim ELF can compare against OpenMemory UI/export after this run.
- Do not claim OpenMemory product UI/export pass from SDK-only `get_all` evidence.
- Do not claim hosted mem0 Platform behavior.
- Do not use this blocker as an ELF win or OpenMemory loss.

## Next Optimization Direction

The next fair product-readback attempt needs a dedicated OpenMemory Docker Compose
profile that imports the generated mem0 corpus into the OpenMemory app database,
starts API/UI with explicit local or provider configuration, and validates exported
memories against `elf-history-user`.

Required fields before the blocker can move:

- dedicated OpenMemory compose profile,
- same-corpus import into the OpenMemory app database,
- OpenMemory API or UI readback artifact,
- export zip validation against the benchmark-owned user,
- explicit provider or local model configuration,
- separate SDK `get_all` and product export scorers.
