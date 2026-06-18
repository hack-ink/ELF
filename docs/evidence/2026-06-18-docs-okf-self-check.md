---
type: Drift Audit
title: "Docs OKF Self-Check"
description: "Drift audit anchoring the documentation bundle migration to the current OKF and LLM Wiki profile."
resource: docs/evidence/2026-06-18-docs-okf-self-check.md
status: active
authority: current_state
owner: docs
last_verified: 2026-06-18
tags:
  - docs
  - drift-audit
  - okf
source_refs: []
code_refs:
  - Makefile.toml
  - scripts/check-docs.py
related: []
drift_watch:
  - docs/
  - Makefile.toml
  - scripts/check-docs.py
---
# Docs OKF Self-Check

Purpose: Anchor the documentation structure migration against the current
Markdown-only OKF and LLM Wiki profile.
Read this when: You need the evidence boundary for the docs readiness claim.
Not this document: Product behavior validation, benchmark result interpretation, or
runtime proof.

## Watched Claims

- `docs/` is a Markdown-only OKF and LLM Wiki bundle.
- Required root files and lane indexes exist.
- Machine-readable JSON artifacts are outside `docs/`; legacy research JSON artifacts
  were promoted, moved to app fixtures, or moved as active tool state.
- Repository-native docs validation still runs through `cargo make check-docs`.
- Decodex profile validation runs through `decodex docs check`.

## Evidence Anchors

- `docs/policy.md` owns the current docs profile.
- `docs/log.md` records the migration.
- `docs/evidence/2026-06-18-research-artifact-disposition.md` records the legacy
  research JSON disposition.
- `Makefile.toml` defines `check-docs` as the repository-native docs task.
- `scripts/check-docs.py` validates repository Markdown links and cargo-make task
  references.

## Reverse Checks

- Search `docs/` for non-Markdown files before claiming readiness.
- Search docs references for stale legacy JSON paths after artifact moves.
- Run both `decodex docs check` and `cargo make check-docs`.

## Verdict

pass

## Required Updates

- Re-run `decodex docs check` after material docs or research layout changes.
- Record any remaining intentional limitations in the final handoff.

## Citations

- `docs/policy.md`
- `docs/log.md`
- `Makefile.toml`
- `scripts/check-docs.py`
