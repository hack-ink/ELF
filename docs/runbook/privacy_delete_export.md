---
type: Runbook
title: "Privacy, Delete, Export, and Retention Boundaries"
description: "Operate ELF memory and knowledge surfaces without confusing current recall, audit retention, export, and provider boundaries."
resource: docs/runbook/privacy_delete_export.md
status: active
authority: procedural
owner: runbook
last_verified: 2026-06-23
tags:
  - docs
  - runbook
  - privacy
  - retention
source_refs: []
code_refs:
  - apps/elf-api/src/routes.rs
  - apps/elf-mcp/src/server.rs
  - packages/elf-service/src/delete.rs
  - packages/elf-service/src/docs.rs
  - packages/elf-service/src/graph_query.rs
  - packages/elf-service/src/graph_report.rs
  - packages/elf-service/src/knowledge.rs
  - packages/elf-service/src/search.rs
  - packages/elf-storage/src/docs.rs
  - packages/elf-storage/src/knowledge.rs
  - apps/elf-worker/src/worker.rs
related:
  - docs/spec/system_doc_source_ref_v1.md
  - docs/spec/system_elf_memory_service_v2.md
  - docs/spec/system_graph_memory_postgres_v1.md
  - docs/spec/system_knowledge_pages_v1.md
drift_watch:
  - docs/runbook/privacy_delete_export.md
  - apps/elf-api/src/routes.rs
  - apps/elf-mcp/src/server.rs
  - packages/elf-service/src/delete.rs
  - packages/elf-service/src/docs.rs
  - packages/elf-service/src/graph_query.rs
  - packages/elf-service/src/graph_report.rs
  - packages/elf-service/src/knowledge.rs
  - packages/elf-service/src/search.rs
  - packages/elf-storage/src/docs.rs
  - packages/elf-storage/src/knowledge.rs
---
# Privacy, Delete, Export, and Retention Boundaries

Purpose: Operate ELF memory and knowledge surfaces without confusing current recall,
audit retention, export, and provider boundaries.
Read this when: You connect sources, ingest private chats, apply delete/forget,
export readback, or validate that derived projections no longer recall a source.
Not this document: Legal compliance policy, provider-specific data-processing terms,
or schema definitions.
Depends on: `docs/spec/system_elf_memory_service_v2.md`,
`docs/spec/system_doc_source_ref_v1.md`,
`docs/spec/system_knowledge_pages_v1.md`, and
`docs/spec/system_graph_memory_postgres_v1.md`.
Verification: Deleted, expired, unreadable, ignored, rejected, and excluded source
spans are absent from normal recall surfaces and derived search results while audit
surfaces remain clearly labeled.

## Authority Map

- Source Library: authoritative long-form source records in `doc_documents` and
  `doc_chunks`. Qdrant doc vectors are derived and rebuildable.
- Memory Ledger: authoritative approved memory notes in `memory_notes`, note
  versions, ingest decisions, and correction history.
- Knowledge Workspace: derived pages, sections, citations, lint findings, and
  rebuild metadata. Knowledge pages are rebuildable projections, not source truth.
- Graph-lite facts: structured `graph_facts` rows with evidence links to memory
  notes. Graph readback is valid only when evidence is still readable for the caller.
- Recall traces: bounded debug evidence for a search. They explain a historical
  retrieval and are not canonical current recall.
- Benchmark artifacts: checked reports and snapshots. They are public-safe evidence
  records, not private-corpus storage.

## Delete And Forget

Memory-note delete sets a note to `deleted`, writes a version row, and enqueues an
indexing delete. Ordinary search, search relation context, graph query, graph report,
and Knowledge Workspace page search must treat deleted or expired notes as
non-recallable. Provenance and history endpoints may still show deleted, deprecated,
or restored rows as audit evidence until lifecycle purge policy removes them.

Source Library delete uses `DELETE /v2/docs/{doc_id}` or the MCP
`elf_docs_delete` tool. It marks source documents non-active and enqueues
per-chunk doc-index `DELETE` work so the worker removes derived doc vectors. Direct
document reads, L0 search, excerpt hydration, and derived Knowledge Workspace page
search must resolve only active source rows. A stored page may still exist after its
source is deleted, but page search must suppress snippets whose normalized source
refs no longer resolve to current sources readable under the caller's read profile
and shared-scope grants.

Applied consolidation proposals are not a shortcut around source visibility. If a
Knowledge Workspace page cites a proposal, normal page search/export may expose only
bounded proposal metadata. Raw proposal `source_refs`, nested source snapshots,
lineage, diffs, markers, target refs, and proposed payload bodies stay in retained
review/audit surfaces, and nested non-captured spans suppress the page snippet.

Graph facts are not hard-deleted merely because one evidence note is deleted. Graph
read APIs must require at least one active, unexpired, readable evidence note before
returning a fact, and must omit deleted or unreadable evidence note ids. Facts with
only deleted, expired, or private evidence are retained as stored rows but are not
normal recall results.

Forget is stronger than ordinary delete only when the operator also removes or purges
the authoritative source rows and retained artifacts under the applicable lifecycle
policy. ELF does not provide a broad public "erase everything everywhere" endpoint in
this contract; use the explicit authority surface and verify each derived projection.

## Private And Excluded Spans

Connected chat, search, repo, and web sources can include private or irrelevant
material. Use request-level write policy exclusions and redactions before storing a
Source Library document or event-derived note. Source capture records policy spans as
`excluded` or `redacted` with reason codes; only `captured` spans are eligible for
derived page search readback.

Private scope remains caller-bound. `agent_private` notes, docs, graph facts, and
derived source refs are readable only by the owning agent under a read profile that
allows private scope. Project and org shared rows still require the relevant scope to
be present in the read profile plus an owner-or-grant match where shared grants apply.

Do not ingest secrets, tokens, private keys, seed phrases, passwords, bank ids, or
personal addresses. The write gate rejects detected secrets for memory notes, but
operators should treat connected-source capture as a pre-ingest trust boundary rather
than relying on downstream cleanup.

## Export

Export means reading through the public or admin API for a specific authority surface
and read profile. It does not bypass scope, payload level, source visibility, or
write-policy suppression.

Use payload levels deliberately:

- `l0`: compact recall and no source_ref payload.
- `l1`: structured summary without full source_ref payload.
- `l2`: full text and source_ref for callers authorized to inspect evidence.

Benchmark reports and checked evidence under `docs/evidence/` must stay public-safe.
Do not commit private corpora, raw private chat logs, secrets, provider credentials,
or unsanitized source exports. Prefer fixture ids, bounded quotes, redaction markers,
and typed blockers when private/provider evidence cannot be published.

## Provider And Local Storage Boundaries

Postgres is the source of truth for notes, docs, graph facts, derived pages, audit
history, and source refs. Qdrant is a derived retrieval index and can be rebuilt or
dropped without changing source truth.

Embedding, rerank, and LLM extractor providers may retain request data according to
their own terms. ELF can prevent recall from local derived projections after delete,
but it cannot retract bytes already sent to an external provider. For private chats,
regulated content, or operator-owned corpora, use local providers or disable the
provider-backed path until provider retention is acceptable.

Recall traces and LLM cache rows are local audit/debug data with configured retention.
They are not source truth, but they can contain snippets or identifiers. Keep admin
binds local, avoid exposing trace bundles publicly, and purge local artifacts when an
operator requires stronger cleanup than normal current-recall suppression.
Public recall-debug panels must hydrate memory-note source refs only for active,
unexpired, readable notes; deleted, deprecated, expired, or unreadable notes may
remain in retained trace audit data but must not expose stored `source_ref` payloads
through normal agent-facing recall-debug.

## Verification Checklist

- Delete a memory note and verify ordinary search no longer returns it.
- Query graph facts for the deleted note's entity and verify facts without active
  readable evidence are absent from graph query/report and relation context.
- Delete a Source Library document through `DELETE /v2/docs/{doc_id}` or
  `elf_docs_delete` and verify direct doc reads, L0 search, excerpts, doc-vector
  points, and Knowledge Workspace page search no longer surface its spans.
- Verify Knowledge Workspace lint or changed-source rebuild reports stale or missing
  source refs instead of treating stale derived text as current authority.
- Verify exported reports and benchmark artifacts contain only public-safe ids,
  bounded quotes, redactions, or typed blockers.
