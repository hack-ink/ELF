# Post-Adapter Production Adoption Refresh - June 10, 2026

Goal: Publish the XY-884 post-adapter production adoption refresh after the live
real-world sweep, OpenViking dependency refresh, and RAG/graph research-gate pass.
Read this when: You need the current decision on whether ELF is ready for personal
production use under the latest checked-in benchmark evidence.
Inputs: `2026-06-09-production-adoption-gate-report.md`,
`2026-06-10-real-world-comparison-report.md`,
`2026-06-10-live-real-world-sweep-report.md`,
`docs/research/2026-06-10-xy-882-rag-graph-adapter-feasibility.json`, and
`apps/elf-eval/fixtures/real_world_external_adapters/memory_projects_manifest.json`.
Depends on: `docs/spec/real_world_agent_memory_benchmark_v1.md`,
`docs/guide/benchmarking/live_baseline_benchmark.md`, and
`docs/guide/single_user_production.md`.
Outputs: Current production adoption decision, evidence-class separation, accepted
caveats, and follow-up issue routing.

## Decision

Adopt with bounded caveats.

ELF remains ready for personal production use as a single-user, self-hosted memory
service when operated through the checked-in production runbook, with Postgres treated
as the source of truth, Qdrant treated as rebuildable, backups enabled, and search
trace/viewer surfaces used for retrieval debugging.

The post-adapter evidence does not upgrade the decision to an unconditional production
pass. It also does not downgrade the June 9 adoption gate. The new evidence mainly
sharpens the claim boundary:

- ELF and qmd now have full-suite live real-world sweep records, but both are typed
  non-pass sweeps, not full-suite live passes.
- The OpenViking cold-start dependency boundary is resolved for classification: the
  pinned Docker local embedding path reaches `add_resource` and `find`, while the
  current OpenViking same-corpus result remains `wrong_result` because expected
  evidence terms are missed.
- The RAG/graph D1/D2 research gates produced adapter candidates and typed blockers,
  but no RAG/graph record has become live adapter evidence.
- Private-corpus and credentialed production-ops checks remain operator-owned
  boundaries. No private-corpus pass is claimed.

## Required Input Status

| Required input | Current outcome | Decision impact |
| --- | --- | --- |
| Full live real-world sweep results for ELF/qmd or typed blockers | Available. ELF and qmd each produced 38 `live_real_world` jobs across 11 suites: 18 pass, 5 wrong_result, 1 incomplete, 2 blocked, and 12 not_encoded. | Supports adoption only with caveats; it proves live sweep coverage, not full-suite live parity. |
| Cold-start/OpenViking dependency issue outcome | Available. The production-ops cold-start dependency fixture is pass; OpenViking now reaches the pinned Docker local embedding path and records `wrong_result` instead of setup failure when evidence terms are missed. | Removes setup uncertainty from the adoption decision, but leaves OpenViking context-trajectory quality as a non-blocking gap. |
| RAG/graph D1/D2 research gate outcome | Available. RAGFlow, LightRAG, GraphRAG, Graphiti/Zep, and graphify are adapter candidates; Letta, LangGraph, nanograph, and llm-wiki are research-only; gbrain is blocked. | Follow-up adapter work is concrete, but research gates remain non-live evidence. |
| Current production/private-corpus evidence and caveats | Available. Provider-backed synthetic, stress, backfill, and restore proof passed; private corpus failed closed because no operator-owned manifest was supplied. | Keeps the June 9 decision: personal production adoption is acceptable with bounded private-corpus and credential caveats. |

## Evidence Classes

| Evidence class | Current evidence | Use in this decision | Claim boundary |
| --- | --- | --- | --- |
| Fixture-backed | `cargo make real-world-memory` reports 38 jobs across 11 suites with 36 pass and 2 blocked production-ops operator boundaries. | Shows the real-world benchmark contract is encoded and ELF fixture behavior is strong outside operator-owned gates. | Fixture scoring is not the same as live service execution. |
| Live adapter | `cargo make real-world-memory-live-adapters` produced full-suite ELF and qmd live sweeps with typed non-pass states preserved. | Confirms live adapters can materialize every encoded job record for ELF and qmd. | Not a full-suite live pass, not private-corpus proof, and not broad external superiority. |
| Private corpus | `baseline-production-private` failed closed at the missing manifest guard. | Accepted caveat for personal use when no operator-owned private manifest exists. | No private-corpus retrieval-quality pass is claimed. |
| Credentialed | Provider-backed ELF synthetic, stress, and backfill runs passed with `Qwen3-Embedding-8B`; provider-backed production-ops fixture jobs remain blocked without routed credentials. | Supports production-provider retrieval and backfill evidence while preserving credential boundaries. | No credentialed production-ops pass is claimed for paths that need unavailable operator credentials. |
| Blocked | Production-ops still contains private manifest and provider credential boundaries; gbrain lacks a proven Docker-local brain repo/database path. | These are explicit accepted caveats or research-gate blockers, not hidden failures. | Blocked states must remain typed until the missing operator or setup input exists. |
| Research gate | RAG/graph records contain setup, resource, retry, and evidence-output metadata plus XY-882 verdicts. | Gives concrete follow-up routing for the next adapter pack. | Research-gate records must not be counted as fixture-backed, live-baseline, or live-real-world pass evidence. |

## Production Evidence

The June 9 production adoption gate remains the production baseline:

| Run | Scope | Result |
| --- | --- | --- |
| Production synthetic provider run | 8 documents, 6 queries, `Qwen3-Embedding-8B`, 4096-dimensional embeddings | `8/8` checks, `retrieval_pass`, `pass` in 59 seconds |
| Provider stress run | 480 generated public documents, 16 queries | `9/9` checks, `retrieval_pass`, `pass` in 779 seconds |
| Provider backfill run | 2,000 generated public documents, 16 queries | `9/9` checks, resume 1,000 -> 2,000, zero duplicate source notes, `pass` in 2,804 seconds |
| Single-user restore proof | Docker Compose backup/restore plus Qdrant rebuild | `rebuilt_count=1`, `missing_vector_count=0`, `error_count=0`, restored search result recovered |
| Private production corpus | Operator-owned manifest required | Failed closed before benchmark execution; no private-corpus pass claimed |

This is enough for personal production use when the operator accepts the documented
private-corpus and credential boundaries. It is not enough for a deployment that
requires private-corpus quality proof before launch.

## Live Sweep Evidence

The full live real-world sweep is useful precisely because it does not flatten typed
outcomes into an artificial win.

| Adapter | Jobs | Pass | Wrong result | Incomplete | Blocked | Not encoded | Evidence recall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ELF live real-world service adapter | 38 | 18 | 5 | 1 | 2 | 12 | 41/75 |
| qmd live real-world CLI adapter | 38 | 18 | 5 | 1 | 2 | 12 | 41/75 |

Both adapters pass the targeted `work_resume`, `project_decisions`, and `retrieval`
suites. Both fail or skip the same broader areas that need more adapter behavior:
current-versus-historical conflict evidence, consolidation proposal generation,
derived knowledge pages, full operator trace hydration, capture/write-policy
integration, and credential/private production operations.

The adoption impact is bounded: ELF has enough production and recovery evidence for
single-user use, but not enough full-suite live evidence to claim broad real-world
memory parity.

## RAG And Graph Gates

XY-882 made the RAG/graph research gates decision-ready:

| Project | Verdict | Follow-up |
| --- | --- | --- |
| RAGFlow | `adapter_candidate` | [XY-885](https://linear.app/hack-ink/issue/XY-885/elf-benchmark-adapter-implement-ragflow-docker-evidence-smoke-adapter) |
| LightRAG | `adapter_candidate` | [XY-886](https://linear.app/hack-ink/issue/XY-886/elf-benchmark-adapter-implement-lightrag-docker-context-export-adapter) |
| GraphRAG | `adapter_candidate` | [XY-887](https://linear.app/hack-ink/issue/XY-887/elf-benchmark-adapter-implement-graphrag-cost-bounded-docker-adapter) |
| Graphiti/Zep | `adapter_candidate` | [XY-888](https://linear.app/hack-ink/issue/XY-888/elf-benchmark-adapter-implement-graphitizep-temporal-graph-adapter) |
| graphify | `adapter_candidate` | [XY-889](https://linear.app/hack-ink/issue/XY-889/elf-benchmark-adapter-implement-graphify-docker-graph-report-adapter) |
| Letta | `research_only` | No implementation issue until a contained evidence export path is selected. |
| LangGraph | `research_only` | No implementation issue; keep as checkpoint/replay reference. |
| nanograph | `research_only` | No implementation issue; keep as graph-lite DX reference. |
| llm-wiki | `research_only` | No implementation issue until a contained plugin or instruction harness exists. |
| gbrain | `blocked` | No implementation issue until a Docker-local brain repo and database path is proven. |

These follow-ups are concrete adapter-work routing, not production blockers for ELF
personal use.

## Accepted Caveats And Follow-Ups

| Gap | Classification | Disposition |
| --- | --- | --- |
| Private production corpus quality | Accepted caveat | Rerun `cargo make baseline-production-private` or `cargo make baseline-production-private-addendum` when an operator-owned sanitized manifest is available. |
| Credentialed production-ops proof | Accepted caveat | Keep typed `blocked` until routed provider credentials are supplied for the specific production-ops gate. |
| Full-suite live real-world pass | Accepted caveat | Current live sweep is intentionally non-pass; use it to target future adapter coverage rather than to block personal production use. |
| OpenViking evidence-bearing retrieval output | Accepted caveat | Setup is no longer the primary blocker; future work should improve same-corpus evidence output before treating OpenViking as a strong runnable context-trajectory baseline. |
| RAGFlow, LightRAG, GraphRAG, Graphiti/Zep, and graphify live adapter evidence | Concrete follow-ups | Use XY-885 through XY-889 and require Docker-contained runs with evidence-linked outputs before any live pass claim. |
| Letta, LangGraph, nanograph, and llm-wiki executable adapter coverage | Accepted research-only caveat | Keep as design references until a contained output contract is selected. |
| gbrain contained setup | Concrete blocker | Revisit only after Docker-local repository/database setup proof exists. |

## Current Adoption Statement

ELF is ready to use personally in production with bounded caveats. Use it when the
operator accepts the checked-in single-user production runbook, backup/restore proof,
provider-backed synthetic/stress/backfill evidence, and explicit private-corpus and
credential boundaries.

Do not claim that ELF has passed a private production corpus, credentialed
production-ops gate, full-suite live real-world parity, or RAG/graph adapter parity.
