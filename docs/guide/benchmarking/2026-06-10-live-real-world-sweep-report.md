# Live Real-World Adapter Sweep Report - June 10, 2026

Goal: Publish the XY-880 full-suite live real-world sweep evidence for ELF and qmd.
Read this when: You need the current live_real_world adapter evidence after the
representative XY-868 slice was expanded across the encoded real-world suite corpus.
Inputs: `cargo make real-world-memory-live-adapters`,
`apps/elf-eval/fixtures/real_world_memory/`, and
`apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`.
Depends on: `docs/spec/real_world_agent_memory_benchmark_v1.md`,
`docs/guide/benchmarking/2026-06-10-real-world-comparison-report.md`, and
`docs/guide/benchmarking/live_baseline_benchmark.md`.
Verification: `cargo make real-world-memory-live-adapters` ran on branch
`y/elf-xy-880` and wrote the generated reports under
`tmp/real-world-memory/live-adapters/`.

## Summary

The live adapter command now runs ELF and qmd against the full checked-in
`real_world_memory` fixture corpus, not only the original three-job representative
slice. Each adapter produced 38 live materialized job records across all 11 encoded
suites.

This is a full-suite sweep, not a full-suite live pass. The generated reports preserve
typed non-pass states instead of upgrading unsupported suite capabilities into wins.

| Adapter | Jobs | Pass | Wrong result | Incomplete | Blocked | Not encoded | Mean score | Evidence recall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ELF live real-world service adapter | 38 | 18 | 5 | 1 | 2 | 12 | 0.514 | 41/75 |
| qmd live real-world CLI adapter | 38 | 18 | 5 | 1 | 2 | 12 | 0.512 | 41/75 |

## Suite Results

| Suite | ELF live status | qmd live status | Interpretation |
| --- | --- | --- | --- |
| `trust_source_of_truth` | `pass` | `pass` | Both adapters retrieved the restore/Qdrant rebuild proof evidence. |
| `work_resume` | `pass` | `pass` | Both adapters passed all work-resume continuity jobs. |
| `project_decisions` | `pass` | `pass` | Both adapters passed all project-decision jobs. |
| `retrieval` | `pass` | `pass` | Both adapters passed all retrieval jobs. |
| `memory_evolution` | `wrong_result` | `wrong_result` | Both adapters passed the delete/TTL case but failed current-versus-historical conflict jobs because retrieval-backed answers did not provide the required historical conflict evidence links. |
| `consolidation` | `not_encoded` | `not_encoded` | The live sweep does not generate or review consolidation proposals. |
| `knowledge_compilation` | `not_encoded` | `not_encoded` | The live sweep does not generate derived knowledge pages. |
| `operator_debugging_ux` | `not_encoded` | `not_encoded` | The live sweep does not hydrate full operator trace/viewer diagnostics. |
| `capture_integration` | `not_encoded` | `not_encoded` | The live sweep does not exercise capture integrations or write-policy redaction boundaries. |
| `production_ops` | `incomplete` | `incomplete` | The live sweep does not run backup/restore, private corpus, provider credential, or backfill operations; the existing cold-start dependency remains incomplete and credential/private-manifest jobs remain blocked. |
| `personalization` | `pass` | `pass` | Both adapters retrieved the scoped preference evidence. |

## Claim Boundary

- ELF and qmd still have targeted live pass evidence for the original
  `work_resume`, `retrieval`, and `project_decisions` slice.
- ELF and qmd now also have full-suite live sweep evidence with typed non-pass states.
- Neither adapter has a full-suite live pass.
- This report does not claim private-corpus production proof, provider-backed
  production-ops proof, broad RAG/graph adapter parity, or overall external
  superiority.

## Artifacts

Generated artifacts are intentionally under `tmp/`:

```text
tmp/real-world-memory/live-adapters/elf-materialization.json
tmp/real-world-memory/live-adapters/elf-report.json
tmp/real-world-memory/live-adapters/elf-report.md
tmp/real-world-memory/live-adapters/qmd-materialization.json
tmp/real-world-memory/live-adapters/qmd-report.json
tmp/real-world-memory/live-adapters/qmd-report.md
tmp/real-world-memory/live-adapters/summary.json
```

The checked-in manifest records this evidence in
`apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`.
