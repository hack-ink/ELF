---
type: Research Contract
title: "Derived Knowledge Page Follow-Up"
description: "Research contract for llm-wiki, gbrain, and OKF-style derived knowledge page patterns that are valuable but not fully implemented."
resource: docs/research/derived_knowledge_page_followup.md
status: active
authority: current_state
owner: research
last_verified: 2026-06-18
tags:
  - docs
  - research
  - llm-wiki
  - okf
source_refs: []
code_refs:
  - docs/evidence/external_memory/comparison_external_projects.md
  - docs/evidence/external_memory/research_projects_inventory.md
  - docs/spec/system_knowledge_pages_v1.md
related: []
drift_watch:
  - docs/spec/system_knowledge_pages_v1.md
  - docs/evidence/external_memory/research_projects_inventory.md
---
# Derived Knowledge Page Follow-Up

Purpose: Preserve the valuable but not fully implemented llm-wiki and gbrain research
thread as a new OKF research contract.
Read this when: You are designing evidence-to-knowledge pages, lint loops, wiki
navigation, or current-truth timeline views.
Not this document: The normative knowledge-page storage contract or a claim that
ELF already ships llm-wiki/gbrain parity.

## Question

How should ELF turn source evidence into rebuildable, cited, lintable project,
entity, concept, issue, and decision pages without letting derived pages replace
authoritative memory?

## Scope

In scope:

- llm-wiki query/save/compile/lint/audit workflows.
- gbrain compiled-truth, timeline, backlink, and primary-home routing patterns.
- OKF and LLM Wiki navigation rules for durable repository docs.
- Citation, stale-source, unsupported-claim, and rebuild checks.

Out of scope:

- Treating generated wiki pages as source-of-truth memory.
- Host-global plugin installs as benchmark proof.
- Broad product parity claims against llm-wiki or gbrain.

## Evidence

- `docs/spec/system_knowledge_pages_v1.md` already owns the normative derived
  knowledge page storage, rebuild, citation, and lint contract.
- `docs/evidence/external_memory/comparison_external_projects.md` records llm-wiki and gbrain
  as reference projects for derived knowledge pages and operational knowledge brain
  presentation.
- `docs/evidence/external_memory/research_projects_inventory.md` records llm-wiki as
  `research_only` and gbrain as `blocked` for adapter purposes.

## Options

- Extend `elf.knowledge_page/v1` with additional LLM Wiki navigation and lint
  evidence.
- Keep llm-wiki/gbrain as research references until ELF has a contained harness that
  produces source-cited pages.
- Drop the thread and rely only on the existing storage spec.

## Judgment

Continue research. The value is real because the current spec defines storage and
lint contracts, but the product-level workflow still needs stronger evidence around
page navigation, source repair, unsupported-claim review, and current-truth/timeline
presentation.

## Challenge

The main risk is duplicating source memory into a polished wiki and then treating the
wiki as authoritative. The mitigation is to keep pages derived, rebuildable, and
explicitly linted against source refs.

## Decision

Not decision-ready for parity claims. Use this contract to route follow-up research
into either `docs/spec/system_knowledge_pages_v1.md` changes or concrete benchmark
evidence.

## Promotion

Promote accepted storage, rebuild, citation, and lint requirements to
`docs/spec/system_knowledge_pages_v1.md`. Promote comparative or upstream movement
only to `docs/evidence/external_memory/comparison_external_projects.md` or
`docs/evidence/external_memory/research_projects_inventory.md`.

## Drift Impact

Watch for upstream llm-wiki/gbrain changes that add contained execution, structured
citation output, unsupported-claim lint, or current-truth timeline maintenance that
ELF can reproduce without host-global state.

## Citations

- `docs/spec/system_knowledge_pages_v1.md`
- `docs/evidence/external_memory/comparison_external_projects.md`
- `docs/evidence/external_memory/research_projects_inventory.md`
