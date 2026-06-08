# Competitive Parity Gate v1 Specification

Purpose: Define the adoption gate ELF must pass before it can be treated as production-eligible memory infrastructure.
Status: normative
Read this when: You are deciding whether ELF is at least as usable as the external memory systems it is being compared against.
Not this document: A market survey, implementation plan, or claim that architecture alone makes ELF better.
Defines: `elf.competitive_parity_gate/v1` dimensions, Docker isolation rules, baseline families, hard thresholds, and report schema.

Related inputs:

- `docs/research/2026-06-08-agent-memory-selection.json`
- `docs/guide/research/comparison_external_projects.md`
- `docs/guide/research/agentmemory_adapter.md`
- `docs/spec/system_elf_memory_service_v2.md`
- `docs/spec/system_consolidation_proposals_v1.md`

## Core Rule

ELF is adoption-eligible only when current test evidence shows that it meets or
exceeds the selected baseline projects in user-visible value. A design advantage,
unchecked capability table, or speculative architecture claim is not sufficient.

The gate must fail closed. If ELF cannot run the comparison, preserve evidence,
retrieve expected memory, expose inspection surfaces, or cleanly isolate state, the
gate result is `fail`.

## Contract Schema

Canonical schema identifier:

```text
elf.competitive_parity_gate/v1
```

Every parity report must carry:

```json
{
  "schema": "elf.competitive_parity_gate.report/v1",
  "gate_schema": "elf.competitive_parity_gate/v1"
}
```

## Docker Isolation

Competitive parity runs must use Docker Compose as the execution boundary.

Required properties:

- The host may invoke `docker compose`, but benchmark code, service processes,
  Postgres, Qdrant, Cargo builds, and test commands must run inside containers.
- The parity compose file must not publish service ports to the host by default.
- Postgres, Qdrant, Cargo registry, Cargo git cache, and Rust target output must use
  Docker-managed volumes.
- The only allowed host artifact is the parity report directory, normally
  `tmp/parity/`.
- A parity runner must refuse to run on the host unless an explicit
  `ELF_PARITY_ALLOW_HOST=1` override is supplied for debugging.
- Cleanup must be possible with `docker compose -f docker-compose.parity.yml down -v
  --remove-orphans`.

## Baseline Families

The gate tracks baseline families separately so evidence can grow without changing
the core contract:

- `agentmemory_fixture`: sanitized offline agentmemory-style session exports mapped
  through the ELF-owned fixture adapter.
- `agentmemory_live_container`: future containerized agentmemory service comparisons
  against the same private evaluation cases.
- `claude_mem_fixture`: future fixture import and retrieval comparison for
  progressive-disclosure Claude memory workflows.
- `mem0_openmemory_fixture`: future local OpenMemory-style workflow comparison.
- `qmd_memsearch_fixture`: future local retrieval-quality comparison against
  CLI/MCP-first hybrid retrieval systems.

External projects are baselines and product references. They must not become hidden
runtime dependencies of ELF core memory semantics unless a separate design spec
explicitly adopts that dependency.

## Gate Dimensions

Each completed gate report must evaluate these dimensions:

| Dimension | Meaning | First hard threshold |
| --------- | ------- | -------------------- |
| `docker_isolation` | The full run used container services and container-local build state. | `pass` |
| `adapter_coverage` | Baseline fixture records are mapped into candidate ELF notes, docs, queries, and ignored reasons. | agentmemory sample emits 2 note candidates, 2 doc candidates, 1 baseline query, and 1 ignored item |
| `provenance_integrity` | Candidate writes keep source-system, session, and item references. | agentmemory note candidate provenance completeness is `1.0` |
| `unsafe_rejection` | Unsupported or unsafe external memory items are rejected explicitly. | at least one ignored item with reason `unsupported_memory_kind` |
| `retrieval_quality` | ELF returns the expected memory for parity queries after normal ingestion/indexing. | consolidation harness after-run recall is not below baseline recall |
| `context_efficiency` | Retrieval/consolidation does not require more context to preserve recall. | consolidation harness after-run context chars are not above baseline |
| `source_safety` | Consolidation output remains derived and reviewable; authoritative source records are not destructively rewritten. | consolidation proposal/source immutability contract remains satisfied |
| `operator_inspectability` | A local operator can inspect memory state without write authority. | admin `GET /viewer` returns 200 during the Docker service run |
| `cleanup` | Test state can be removed without host database or vector-store residue. | documented compose cleanup command exists and succeeds when run |

These are minimum thresholds. Passing them only proves that the checked-in gate is
alive. Personal production use requires the same gate shape to pass against a larger
private fixture pack and at least one live containerized baseline.

## First Gate Scope

The first checked-in executable gate covers:

- Docker-only execution through `docker-compose.parity.yml`.
- Offline `agentmemory_fixture` adapter validation using the sanitized sample fixture.
- Service-backed ELF consolidation/retrieval validation using Postgres and Qdrant
  containers.
- Admin viewer availability during the service-backed run.
- A machine-readable report under `tmp/parity/competitive-parity-report.json`.

The first gate does not claim broad market superiority. It establishes a hard,
repeatable lower bound that must stay green before broader baselines are meaningful.

## Report Schema

Parity reports must be JSON objects with at least:

- `schema`: `elf.competitive_parity_gate.report/v1`
- `gate_schema`: `elf.competitive_parity_gate/v1`
- `gate_id`: stable or timestamped run identifier
- `verdict`: `pass` or `fail`
- `docker_only`: boolean
- `baselines`: object keyed by baseline family
- `dimensions`: object keyed by gate dimension
- `thresholds`: object describing the hard thresholds used by the run
- `artifacts`: object with relative paths to preserved run evidence

Reports may include extra metrics, but extra fields must not weaken the hard
thresholds in this spec.

## Adoption Decision

Treat ELF as `not_adoptable_for_production` while any of these are true:

- The Docker parity gate fails.
- The gate only passes the checked-in toy fixture and has not passed a private
  personal fixture pack.
- At least one selected external baseline outperforms ELF on retrieval quality,
  migration fidelity, operator inspectability, or failure recovery without a
  documented compensating ELF advantage.
- Evidence cannot be reproduced from the report artifacts.

Treat ELF as `personal_production_candidate` only after the Docker gate passes on
both the checked-in fixture and a private personal fixture pack, and after at least
one live external baseline comparison is no worse than ELF on the selected
acceptance metrics.
