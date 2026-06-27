---
type: Drift Audit
title: "Authority Recovery Drill Drift Audit"
description: "Drift audit for production-ops authority recovery drill benchmark artifacts and reports."
resource: docs/evidence/2026-06-27-authority-recovery-drill-drift-audit.md
status: active
authority: evidence
owner: docs
last_verified: 2026-06-27
tags:
  - docs
  - evidence
  - benchmarking
  - production-ops
source_refs:
  - https://linear.app/hackink/issue/XY-1119
code_refs:
  - apps/elf-eval/src/bin/real_world_job_benchmark.rs
  - apps/elf-eval/fixtures/real_world_memory/production_ops/authority_plane_recovery_drill.json
  - docs/spec/real_world_agent_memory_benchmark_v1.md
  - docs/runbook/benchmarking/real_world_agent_memory_benchmark.md
related:
  - docs/spec/real_world_agent_memory_benchmark_v1.md
  - docs/runbook/benchmarking/real_world_agent_memory_benchmark.md
drift_watch:
  - apps/elf-eval/src/bin/real_world_job_benchmark.rs
  - apps/elf-eval/fixtures/real_world_memory/production_ops/
  - docs/spec/real_world_agent_memory_benchmark_v1.md
---
# Authority Recovery Drill Drift Audit

Purpose: Anchor the production-ops authority recovery drill report contract to the
runner, fixture, and documentation surfaces.
Read this when: You need evidence for backup/PITR, idempotent outbox replay, Qdrant
rebuild completeness, degraded read, migration repair, dead-letter handling, and
RPO/RTO reporting in the real-world memory benchmark.
Not this document: Live production restore proof, private-corpus quality, hosted HA,
or multi-region failover evidence.

## Watched Claims

- `elf.authority_recovery_drill/v1` is a benchmark artifact under
  `adapter_response.answer.recovery_drills[]`.
- The runner validates drill topology, failure injections, backup/PITR restored
  evidence, degraded-read labels, RPO/RTO measurements, matching authority record
  counts for source, journal, memory, knowledge, proposal, trace, and audit planes,
  idempotent outbox replay, Qdrant rebuild completeness, migration repair, and
  dead-letter handling.
- Reports expose those drill counts through
  `operational_evidence.authority_recovery`, including backup/PITR restored and
  record-count preservation counters.
- The checked-in fixture is local synthetic evidence only. It does not prove private
  corpus quality, provider-backed behavior, hosted HA, standby failover, or
  multi-region SLA.

## Evidence Anchors

- `apps/elf-eval/src/bin/real_world_job_benchmark.rs` defines and validates
  `AuthorityRecoveryDrillArtifact` and aggregates
  `OperationalAuthorityRecoveryReport`.
- `apps/elf-eval/fixtures/real_world_memory/production_ops/authority_plane_recovery_drill.json`
  encodes one production-ops job with topology, degraded-read labels, RPO/RTO,
  matching before/after authority record counts, replay, rebuild, migration repair,
  and dead-letter evidence.
- `docs/spec/real_world_agent_memory_benchmark_v1.md` defines the artifact schema and
  production-ops/report semantics.
- `docs/runbook/benchmarking/real_world_agent_memory_benchmark.md` routes operators to
  the production-ops command and describes the authority recovery drill coverage.

## Reverse Checks

- Run `cargo make real-world-memory-production-ops` to parse the fixture and render
  the production-ops report.
- Run `cargo make check-docs` after docs changes.

## Verdict

pass

## Required Updates

- If recovery drill fields change, update the runner structs, fixture, benchmark
  spec, runbook, and this audit together.
- If a live Docker recovery drill is added later, preserve the fixture/local evidence
  boundary and add separate live evidence instead of reclassifying this fixture.

## Citations

- `apps/elf-eval/src/bin/real_world_job_benchmark.rs`
- `apps/elf-eval/fixtures/real_world_memory/production_ops/authority_plane_recovery_drill.json`
- `docs/spec/real_world_agent_memory_benchmark_v1.md`
- `docs/runbook/benchmarking/real_world_agent_memory_benchmark.md`
