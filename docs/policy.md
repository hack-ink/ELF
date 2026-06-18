---
type: Policy
title: "Documentation OKF Policy"
description: "Canonical Markdown-only OKF and LLM Wiki policy for repository documentation."
resource: docs/policy.md
status: active
authority: normative
owner: docs
last_verified: 2026-06-18
tags:
  - docs
  - okf
  - llm-wiki
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
# Documentation OKF Policy

Purpose: Own the repository documentation shape, lane policy, and validation gates for
the Markdown-only OKF and LLM Wiki bundle.
Status: normative
Read this when: You are creating, moving, splitting, promoting, or validating
repository documentation.
Not this document: Product behavior contracts, operational runbooks for one
subsystem, or raw machine-readable research artifacts.
Defines: OKF concept shape, LLM Wiki lane ownership, docs validation, and research
artifact placement.

## Bundle Contract

- `docs/` is a Markdown-only OKF and LLM Wiki bundle.
- `docs/index.md` is the root router.
- `docs/policy.md` owns documentation shape and validation policy.
- `docs/log.md` records material navigation, promotion, naming, and maintenance changes.
- Every populated directory under `docs/` has an `index.md`.
- Non-index, non-log Markdown files are OKF concepts with YAML frontmatter.
- Machine-readable JSON research state, benchmark reports, cursors, and sample datasets
  live outside `docs/`; docs concepts link to or name those artifacts as evidence.

## Required Lanes

- `docs/spec/`: normative contracts, schemas, invariants, and required behavior.
- `docs/runbook/`: procedural runbooks, migrations, validation flows, and
  operational sequences.
- `docs/reference/`: current structure references and non-procedural orientation.
- `docs/decisions/`: accepted rationale and durable decision records.
- `docs/research/`: latent research contracts and research evidence candidates.
- `docs/evidence/`: public-safe proof, validation evidence, and drift audits.
Historical plan artifacts may live under `docs/reference/plans/` while they remain
useful for repository navigation. They are reference concepts, not a top-level docs
lane.

## Concept Frontmatter

Every OKF concept requires:

- `type`
- `title`
- `description`
- `status`
- `authority`
- `owner`
- `last_verified`

Allowed concept types are `Decision`, `Drift Audit`, `Evidence`, `Policy`,
`Reference`, `Research Contract`, `Runbook`, and `Spec`.

Use `tags`, `source_refs`, `code_refs`, `related`, `promotes_to`, and `drift_watch`
when they improve owner discovery or drift review.

## Research Boundary

Research concepts are latent until explicitly promoted. A research concept may cite a
machine-readable artifact outside `docs/`, but the raw artifact is not the docs owner.
Promote accepted facts into `docs/spec/`, `docs/runbook/`, `docs/reference/`,
`docs/decisions/`, or `docs/evidence/`; retire stale raw artifacts once their settled
content has an owner; then update indexes, links, and `docs/log.md`.

## Validation

- Run `decodex docs check` before claiming the OKF and LLM Wiki bundle is ready.
- Run `cargo make check-docs` for the repository-native Markdown link and task-name
  check.
- When docs claims touch commands, config, code, schemas, generated outputs, or runtime
  behavior, perform a semantic drift audit and record the evidence under
  `docs/evidence/`.
