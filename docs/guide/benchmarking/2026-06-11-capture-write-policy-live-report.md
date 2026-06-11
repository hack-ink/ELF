# Capture/Write-Policy Live Report - June 11, 2026

Goal: Record the XY-933 live capture/write-policy evidence and competitor claim
boundaries.
Read this when: You need to know whether ELF has live evidence for capture redaction,
exclusions, source ids, evidence binding, and no secret leakage.
Inputs: `cargo make real-world-memory`, `cargo make real-world-memory-live-adapters`,
`apps/elf-eval/fixtures/real_world_memory/capture_integration/`, and
`apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`.
Outputs: Scenario-level capture results, live artifacts, and typed blocker reasons for
agentmemory and claude-mem capture breadth.

## Verdict

ELF now has live capture/write-policy self-check evidence. The ELF live service adapter
passes all 4 `capture_integration` jobs with zero redaction leaks and full required
evidence/source-ref/quote coverage.

This is not a broad capture-hook superiority claim. ELF has a live self-check for the
currently encoded capture/write-policy suite, while qmd keeps those jobs typed
`not_encoded`; that makes qmd untested on this surface, not an ELF-over-qmd win.
Against agentmemory and claude-mem capture breadth, the comparison is still blocked
or untested because no durable local adapter evidence exists for their hook/viewer
capture paths.

## Fresh Runs

| Command | Result | Artifact |
| --- | --- | --- |
| `cargo make real-world-memory` | pass | `tmp/real-world-memory/real-world-memory-report.json` |
| `cargo make real-world-memory-live-adapters` | pass | `tmp/real-world-memory/live-adapters/summary.json` |

## ELF Capture Results

| Job | Live status | Evidence coverage | Source-ref coverage | Redaction leaks | Capture evidence |
| --- | --- | ---: | ---: | ---: | --- |
| `capture-redaction-exclusion-001` | `pass` | `2/2` | `2/2` | `0` | Stores public decision and write-policy audit; excludes private text. |
| `capture-source-id-binding-001` | `pass` | `2/2` | `2/2` | `0` | Preserves `capture:issue-comment-42` and `capture:command-log-7`. |
| `capture-write-policy-redaction-001` | `pass` | `2/2` | `2/2` | `0` | Applies one write-policy redaction and preserves `capture:terminal-log-17`. |
| `capture-integration-boundaries-001` | `pass` | `4/4` | `4/4` | `0` | Preserves the no-live boundary for external hooks and viewer flows. |

The ELF materialization artifact records:

- stored evidence ids for captured public items;
- excluded evidence ids for private or trap inputs;
- runtime `source_ref` metadata returned by search, including copied source ids;
- write-policy audit, exclusion, and redaction counts;
- generated answers that contain no redaction trap text.

## Comparison Boundary

| Compared target | Position | Reason |
| --- | --- | --- |
| qmd live real-world adapter | `untested` | ELF executes and passes 4/4 live capture jobs; qmd keeps the same jobs typed `not_encoded`, so this remains an ELF self-check rather than a qmd comparison result. |
| agentmemory capture hooks | `blocked` | The current Docker baseline uses a process-local StateKV Map and in-memory index. No durable local session/capture path stores source ids, exclusions, write-policy audit, or evidence-bound output. |
| claude-mem capture/viewer flows | `untested` | The checked evidence exercises repository storage, lifecycle, progressive disclosure, and same-corpus retrieval only. Hooks, timeline, observations, viewer capture, and automatic capture review are not run against real-world jobs. |

## Claims Allowed

- ELF live capture/write-policy self-checks pass for redaction, exclusions, source ids,
  evidence binding, and no secret leakage.
- qmd remains `not_encoded` for capture/write-policy jobs in the full live sweep.
- agentmemory capture comparison is blocked by mocked/in-memory storage and lack of a
  durable local capture artifact.
- claude-mem capture breadth is untested until a Docker-contained hook/viewer capture
  runner exists.

## Claims Not Allowed

- Do not claim ELF broadly beats agentmemory or claude-mem on capture breadth.
- Do not use host-global hooks as benchmark evidence.
- Do not weaken ELF write-policy, redaction, or evidence-binding constraints for
  benchmark convenience.
- Do not convert fixture-backed or live-baseline-only capture references into a live
  real-world competitor pass.
