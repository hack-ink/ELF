---
type: Drift Audit
title: "Knowledge Watch/Rebuild Drift Audit"
description: "Drift audit for the changed-source Knowledge Workspace rebuild and reviewable memory-candidate contract."
resource: docs/evidence/2026-06-22-knowledge-watch-rebuild-drift-audit.md
status: active
authority: current_state
owner: docs
last_verified: 2026-06-22
tags:
  - docs
  - drift-audit
  - knowledge-workspace
source_refs: []
code_refs:
  - apps/elf-api/src/routes.rs
  - packages/elf-domain/src/knowledge.rs
  - packages/elf-service/src/knowledge.rs
  - packages/elf-storage/src/knowledge.rs
  - docs/spec/system_knowledge_pages_v1.md
  - docs/spec/system_elf_memory_service_v2.md
related:
  - docs/spec/system_knowledge_pages_v1.md
  - docs/spec/system_consolidation_proposals_v1.md
drift_watch:
  - apps/elf-api/src/routes.rs
  - packages/elf-domain/src/knowledge.rs
  - packages/elf-service/src/knowledge.rs
  - packages/elf-storage/src/knowledge.rs
  - docs/spec/system_knowledge_pages_v1.md
  - docs/spec/system_elf_memory_service_v2.md
---
# Knowledge Watch/Rebuild Drift Audit

Purpose: Anchor the changed-source Knowledge Workspace rebuild contract to the
current service and API surfaces.
Read this when: You need evidence boundaries for the knowledge watch/rebuild API,
section-state output, or reviewable memory-candidate proposal path.
Not this document: Benchmark interpretation, external product comparison, or
operator setup procedure.

## Watched Claims

- `POST /v2/admin/knowledge/pages/rebuild-changed-sources` is the admin entrypoint
  for changed-source rebuilds.
- Changed-source rebuilds select only pages that already cite supplied source refs.
- Rebuild output reports changed, unchanged, stale, and blocked page/section states.
- Rebuild output emits stale-section, changed-claim, missing-citation, and conflict
  classifications.
- Memory candidates from knowledge deltas are queued through consolidation proposals
  and do not mutate source records or memory notes directly.

## Evidence Anchors

- `packages/elf-storage/src/knowledge.rs` owns affected-page lookup by source ref.
- `packages/elf-service/src/knowledge.rs` owns changed-source lint, rebuild,
  section-state output, and memory-candidate proposal payload construction.
- `apps/elf-api/src/routes.rs` exposes the admin route.
- `docs/spec/system_knowledge_pages_v1.md` owns the normative contract.
- `docs/spec/system_consolidation_proposals_v1.md` owns the reviewable memory
  promotion path.

## Reverse Checks

- Run `cargo make check-rust` after code changes to verify the route/service/storage
  surface compiles.
- Run `cargo make check-docs` after docs changes to verify links and task references.
- Run the focused knowledge service tests before claiming the source-update and stale
  page cases are covered.

## Verdict

pass

## Required Updates

- Keep `docs/spec/system_knowledge_pages_v1.md` aligned with any future changes to
  watch/rebuild response fields, output classifications, or proposal routing.
- The repository-native docs gate passed for this lane. The stricter Decodex docs
  profile still reports unrelated P1 closeout report metadata shape issues outside
  this lane.

## Citations

- `apps/elf-api/src/routes.rs`
- `packages/elf-service/src/knowledge.rs`
- `packages/elf-storage/src/knowledge.rs`
- `docs/spec/system_knowledge_pages_v1.md`
- `docs/spec/system_consolidation_proposals_v1.md`
