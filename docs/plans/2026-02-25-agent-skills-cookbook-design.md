# Agent Skills Cookbook (MCP-first) — Design

Status: Proposed
Date: 2026-02-25

## Problem

ELF is used primarily via MCP, but without reference agent-side workflows, different agents:

- Write inconsistent note shapes, keys, scopes, and TTLs.
- Fail to use facts-first + evidence hydration correctly (either storing long text in notes or failing to hydrate supporting evidence).
- Drift on sharing/grants workflows, reducing multi-agent interoperability.

## Goal

Ship a non-normative "skills cookbook" that standardizes how an agent should use ELF via MCP:

- Facts-first memory in Core (short notes).
- Long-form evidence via Doc Extension v1 (store documents; hydrate bounded excerpts on demand).
- Multi-agent sharing through explicit scopes and grants.

This cookbook is a guide/playbook, not a system contract. It must not change ELF Core semantics.

## Core vs Skills contract

### MCP (capability + invariants)

MCP tools must remain a thin forwarding layer to ELF HTTP endpoints and must not contain policy.
All hard guarantees are enforced server-side (elf-api/elf-service), including:

- English-only boundary enforcement.
- ACL/tenancy/scope access.
- Size limits and caps.
- Idempotency and safe retry behavior (where supported).
- Auditability and provenance surfaces exposed by the API.

### Skills (policy + workflow)

Skills define agent-side workflows and policies, such as:

- What to remember vs ignore, and how to normalize content into compact facts.
- When to store long evidence in Doc and attach pointers in note `source_ref`.
- When to hydrate evidence and how to progressively disclose (L0 -> L1 -> L2).
- How to choose scope, keys, TTLs, and how to consolidate/refresh memories over time.

## Deliverable

Add a single guide document:

- `docs/guide/agent_skills_cookbook.md`

It should include:

1. A short "MCP vs Skills" contract and failure modes.
2. Reference workflows:
   - doc_ingest
   - hydrate_context
   - memory_write_policy
   - share_workflow
   - reflect_consolidate
3. Copy-pastable MCP tool-call JSON examples (English-only).

## Non-goals

- No new server features or new endpoints (this is documentation only).
- No changes to normative specs.
- No attempt to ship a general-purpose doc/search platform in Core.

