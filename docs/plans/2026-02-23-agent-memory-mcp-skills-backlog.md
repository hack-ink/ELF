# Agent Memory (MCP + Skills) Backlog
Date: 2026-02-23

## Summary
This document captures backlog issues for making ELF maximally usable as an AI-agent memory system when the primary integration surface is MCP.

The key product gap is long-form memory usability: store compact, evidence-linked facts in ELF while referencing long documents via pointers and hydrating relevant excerpts on demand.

## Goals
- Support long-form memory via doc pointers + on-demand hydration while keeping ELF notes compact.
- Make multi-agent / multi-brain shared memory operable via MCP (not HTTP-only).
- Provide reference “skills” (agent-side workflows) so different agents behave consistently.
- Preserve ELF invariants: explicit scopes, explicit sharing grants, auditability, and rebuildable derived indexes.

## Non-Goals
- Turning ELF into a general-purpose document warehouse (unless explicitly decided later).
- Removing the English-only boundary in the v2 contract (treat non-English as an upstream canonicalization concern for now).
- Shipping a full hosted managed service offering.

## Backlog Issues

### Issue 1: Expose sharing + grants management via MCP
Problem: The HTTP API has publish/unpublish and grant management endpoints, but MCP does not expose them. This prevents “MCP-only” agents from operating shared memory.

Proposed MCP tools:
- `elf_notes_publish`
- `elf_notes_unpublish`
- `elf_space_grants_list`
- `elf_space_grant_upsert`
- `elf_space_grant_revoke`

Acceptance criteria:
- Tools forward to the corresponding HTTP endpoints.
- Tools respect server-side auth and context headers (tenant/project/agent/read_profile).
- Add basic end-to-end tests for MCP tool registration + request forwarding.

### Issue 2: Define a versioned `source_ref` schema for doc pointers
Problem: `source_ref` is required and flexible, but without a standard schema downstream agents cannot reliably hydrate documents.

Proposed `source_ref` shape (v0):
- `kind`: `"doc_pointer"`
- `schema_version`: `"0"`
- `doc_id`: stable identifier
- `uri`: optional canonical location
- `content_hash`: strong hash of canonical bytes (or normalized text)
- `title`: optional
- `mime_type`: optional
- `locator`: optional section/span pointer (e.g. `{ "section": "...", "start": 123, "end": 456 }`)
- `access`: optional hint for how to fetch (e.g. `"s3" | "http" | "local_fs"`)

Acceptance criteria:
- Add a spec/guide page describing the schema and forward/backward compatibility rules.
- Provide at least one reference implementation of encoding/decoding in an agent-side “skill”.

### Issue 3: Add a document hydration component (Doc Store and/or Doc MCP)
Problem: ELF intentionally stores compact notes; long documents need a canonical store that can return excerpts safely and cheaply.

Options:
- A) Separate “doc store” service with its own MCP (`doc-mcp`) and a small set of tools:
  - `doc_put(doc_bytes, metadata) -> {doc_id, content_hash}`
  - `doc_get(doc_id) -> bytes` (or streaming)
  - `doc_excerpt(doc_id, locator | query) -> excerpt(s)`
- B) Extend ELF HTTP API and MCP to include document endpoints (higher coupling).

Acceptance criteria:
- Clear ownership of document durability and access control.
- Deterministic excerpting rules (max bytes, max excerpts, stable locators).
- Integration example showing: ingest long doc -> write ELF pointers -> search -> hydrate excerpts.

### Issue 4: Ship a “skills cookbook” (reference agent workflows)
Problem: Without standardized workflows, different agents will write inconsistent notes, misuse scopes, and fail to hydrate long-form context.

Proposed skills (agent-side workflows, not server responsibilities):
- `doc_ingest`: long doc -> doc store -> extract compact facts -> write notes with `source_ref`.
- `hydrate_context`: interpret `source_ref` -> fetch excerpt(s) -> progressive disclosure injection.
- `memory_write_policy`: decide add_note vs add_event, keys, scope selection, and update vs ignore.
- `share_workflow`: publish/unpublish + grants management (project/org sharing).
- `reflect_consolidate`: periodic consolidation of episodic events into stable profiles/decisions/constraints.

Acceptance criteria:
- A small set of runnable examples (or pseudocode + prompt templates) that only require MCP connectivity.
- Guidance on safe defaults (no secrets, evidence rules, TTL expectations).

### Issue 5: Reflection / consolidation loop (human-like “memory formation”)
Problem: Brain-like memory is not just storage + retrieval; it needs consolidation and conflict resolution over time.

Proposed approach:
- Implement as an operator- or scheduler-driven job (agent-side), not inside ELF core.
- Inputs: recent events, high-hit notes, conflicting keys, nearing TTL items.
- Outputs: a small number of updated stable notes (decisions/constraints/profile) with explicit provenance and keys.

Acceptance criteria:
- A deterministic policy surface for what gets consolidated (thresholds, caps, key strategy).
- Evaluation harness scenario(s) that demonstrate reduced context size with preserved correctness.

### Issue 6: Standardize provenance + observability surfaces
Problem: Auditable memory requires consistent provenance and trace correlation across ingest, retrieval, and hydration.

Proposed work:
- Define a provenance mapping for `source_ref` and note evolution (versioning, updates, deprecations).
- Add OpenTelemetry-compatible tracing around ingest/search flows (at least span + request IDs).

Acceptance criteria:
- Operators can answer: “Where did this memory come from?” and “Why was it retrieved?” with stable identifiers.

### Issue 7: Multi-language strategy (English-only boundary vs product reality)
Problem: The v2 contract is English-only; many real deployments are multi-language.

Proposed approach (near-term):
- Keep ELF contract unchanged.
- Add upstream canonicalization in skills (translate/summarize to English + preserve original text in doc store).

Acceptance criteria:
- Clear guidance and examples for CJK/Chinese user inputs: how to store original, how to store English facts, how to hydrate both.

## Open Questions (To Resolve Before Implementation)
1) Doc store choice: S3/object storage vs Postgres large fields vs dedicated document service.
2) Multi-language requirement: is Chinese-first a product requirement, or is English-only acceptable for v2?
3) Can agents connect to multiple MCP servers (e.g., `elf-mcp` + `doc-mcp`), or must everything be behind `elf-mcp`?

## Research Notes (External References)
- Retrieval-Augmented Generation (RAG): https://arxiv.org/abs/2005.11401
- MemGPT (tiered “virtual context” memory): https://arxiv.org/abs/2310.08560
- Generative Agents (memory stream + reflection loop): https://arxiv.org/abs/2304.03442
- BEIR benchmark (retrieval families + robustness): https://arxiv.org/abs/2104.08663
- Reciprocal Rank Fusion (RRF): https://dl.acm.org/doi/10.1145/1571941.1572114
- Transactional outbox pattern: https://microservices.io/patterns/data/transactional-outbox.html
- W3C PROV-DM provenance model: https://www.w3.org/TR/prov-dm/
- OpenTelemetry tracing spec: https://opentelemetry.io/docs/specs/otel/trace/

