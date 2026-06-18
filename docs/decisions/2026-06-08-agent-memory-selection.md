---
type: Decision
title: "Agent Memory Selection"
description: "Accepted decision to keep ELF as the evidence-bound memory core while borrowing external memory systems only as adapters, baselines, and derived patterns."
resource: docs/decisions/2026-06-08-agent-memory-selection.md
status: active
authority: normative
owner: decisions
last_verified: 2026-06-18
tags:
  - docs
  - decision
  - memory
  - research-promotion
source_refs: []
code_refs:
  - docs/evidence/external_memory/comparison_external_projects.md
  - docs/evidence/external_memory/research_projects_inventory.md
related: []
drift_watch:
  - docs/evidence/external_memory/comparison_external_projects.md
  - docs/evidence/external_memory/research_projects_inventory.md
  - docs/spec/system_competitive_parity_gate_v1.md
  - docs/spec/system_consolidation_proposals_v1.md
---
# Agent Memory Selection

Purpose: Preserve the accepted June 2026 decision about ELF's relationship to
external agent-memory systems.
Status: normative
Read this when: You are deciding whether ELF should adopt, replace, or integrate with
agentmemory, managed dreaming systems, or adjacent memory projects.
Not this document: A live benchmark result, upstream market survey, or adapter
implementation plan.
Defines: ELF remains the evidence-bound memory core; external systems are optional
capture, benchmark, viewer, and derived-consolidation inputs.

## Decision

Continue ELF as the evidence-bound memory core. Do not replace ELF with agentmemory,
managed dreaming APIs, or another external memory product.

Borrow external systems only where they preserve ELF's source-of-truth boundary:

- optional capture/import adapters
- benchmark baselines
- viewer and operator UX references
- reviewable derived consolidation patterns
- graph, timeline, and knowledge-page presentation patterns

## Rationale

ELF's durable advantage is the explicit evidence contract: deterministic writes,
scoped service semantics, Postgres as the source of truth, rebuildable derived indexes,
and provenance-oriented evaluation. External systems reviewed in June 2026 are useful
but do not replace that contract.

agentmemory is valuable for coding-agent continuity, hooks, MCP/REST packaging, a
viewer, and benchmark UX. That value supports an adapter and benchmark baseline, not a
core replacement.

Dreaming-style systems are valuable because OpenAI, Anthropic, and Google converge on
background memory curation as a product direction. The safe shared pattern is
reviewable derived output over immutable input evidence, not destructive rewriting of
authoritative memory.

## Rejected Options

- Replace ELF with agentmemory.
- Replace ELF's roadmap with managed dreaming APIs.
- Pause ELF core development until the agent-memory market stabilizes.

## Promotion

This decision promotes the accepted conclusion from the retired
`2026-06-08-agent-memory-selection` research run. Settled facts are now owned by this
decision, `docs/evidence/external_memory/comparison_external_projects.md`,
`docs/spec/system_competitive_parity_gate_v1.md`, and
`docs/spec/system_consolidation_proposals_v1.md`.

Remaining unresolved value points are tracked as active research contracts instead of
raw JSON artifacts:

- `docs/research/derived_knowledge_page_followup.md`
- `docs/research/dreaming_product_surface_followup.md`
- `docs/research/graph_rag_adapter_followup.md`

## Drift Watch

Revisit this decision only if an external project provides an ELF-equivalent
evidence-bound deterministic write contract, source-of-truth storage, multi-tenant
service semantics, and lower integration risk, or if a self-hostable managed dreaming
system provides portable, reviewable, evidence-linked memory stores that satisfy ELF's
governance boundary.

## Citations

- `docs/evidence/external_memory/comparison_external_projects.md`
- `docs/evidence/external_memory/research_projects_inventory.md`
- `docs/spec/system_competitive_parity_gate_v1.md`
- `docs/spec/system_consolidation_proposals_v1.md`
