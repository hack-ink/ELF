---
type: Spec
title: "Context Pack v1 Specification"
description: "Define the read-time Context Pack v1 contract, routing trace, and privacy boundaries."
resource: docs/spec/system_context_pack_v1.md
status: active
authority: normative
owner: spec
last_verified: 2026-07-03
tags:
  - docs
  - spec
  - context-pack
source_refs:
  - https://linear.app/hack-ink/issue/XY-1154/implement-context-pack-v1-automatic-routing-recall-engine-and-recall-debug
code_refs:
  - packages/elf-service/src/context_pack.rs
  - apps/elf-api/src/routes/context_pack.rs
  - apps/elf-mcp/src/app/server/tools/core/memory.rs
related:
  - docs/spec/agent_memory_knowledge_system_v1.md
  - docs/spec/system_recall_debug_panel_v1.md
  - docs/spec/system_elf_memory_service_v2.md
drift_watch:
  - packages/elf-service/src/context_pack.rs
  - apps/elf-api/src/routes/context_pack.rs
  - apps/elf-mcp/src/app/server/tools/core/memory.rs
---
# Context Pack v1 Specification

Purpose: Define the read-time Context Pack v1 contract, routing trace, and privacy boundaries.
Status: normative
Read this when: You are implementing, validating, or reviewing Context Pack routing,
pack assembly, API/MCP exposure, or Recall Debug trace integration.
Not this document: Low-level search ranking, source capture, memory writes, or
operator procedures.
Defines: `elf.context_pack/v1`, `elf.context_pack.routing_trace/v1`, automatic
layer routing, pin/override limits, freshness suppression, and read-time privacy.

## Boundary

Context Packs are ephemeral read-time scoped views. They assemble bounded, cited
context from current readable recall layers and expire with the response. Context
Pack creation must not insert, update, delete, promote, correct, index, or persist
Memory Authority notes, Source Library documents, Knowledge Workspace pages, Work
Journal entries, graph facts, Dreaming proposals, search traces, or Qdrant points.

The public HTTP route is:

- `POST /v2/context-packs`

The MCP tool is:

- `elf_context_pack_build`

Both surfaces are read-only facades over `elf-service`. The HTTP route derives
tenant, project, agent, and read profile from request headers. The MCP tool must not
accept tenant, project, agent, or read-profile override fields; it uses the MCP
server-configured context headers.

## Request

The request body accepts:

- `task` (required): English task text used by automatic routing.
- `title`, `description` (optional): display metadata.
- `trace_id` (optional): Memory Notes trace anchor.
- `query` (optional): shared query fallback for document and knowledge selectors.
- `docs_query` (optional): Source Library selector.
- `knowledge_query` (optional): Knowledge Workspace selector.
- `graph_subject`, `graph_predicate` (optional): graph selector.
- `include_dreaming` (optional): Dreaming proposal selector.
- `limit` (optional): max pack items, clamped to the service maximum.

Request context is never read from the body.

Service-level debug/test/admin callers may use internal route controls for
`enable_layers`, `disable_layers`, and `pin_layers`. Public HTTP and MCP Context Pack
requests must not expose those controls.

## Response

The response schema is `elf.context_pack/v1`.

Required top-level fields:

- `pack_id`: generated UUID for this ephemeral response.
- `schema`: `elf.context_pack/v1`.
- `version`: `1`.
- `title` and `description`.
- `activation_policy`.
- `authority_layers`.
- `typed_selectors`.
- `required_anchors`.
- `read_profile_policy`.
- `freshness_policy`.
- `budget_limits`.
- `ranking_policy`.
- `debug_policy`.
- `routing_trace`.
- `items`.
- `recall_trace`.
- `generated_at` and `expires_at`.

Pack items carry only references and metadata returned by readable current recall
rows:

- `layer`
- `authority_layer`
- `freshness_state`
- `item_ref`
- `source_refs`
- `score`
- `rank`
- `reason_code`
- `pinned_priority`

Context Pack items are not facts, summaries, or durable memory. Callers must treat
them as transport references back to the owning authority layer.

## Automatic Routing

Default routing is automatic:

- `trace_id` selects the Memory Notes layer.
- `docs_query`, or `query`, or non-empty `task` selects Source Library search.
- `knowledge_query`, or `query`, or non-empty `task` selects Knowledge Workspace search.
- `graph_subject` selects graph facts.
- `include_dreaming = true`, or task text containing Dreaming/review/proposal intent,
  selects Dreaming proposals.

Manual enable, disable, and pin controls are override/debug/test/admin aids only and
are not public agent request fields. When a trusted internal/admin caller uses them,
they must be recorded in selectors and routing trace. A disabled layer is suppressed.
An enabled layer still requires its required anchor. A pinned layer may move eligible
items earlier in pack ordering, but pinning cannot bypass scope, read profile, grants,
freshness, deletion, redaction, authority, evidence, or required-anchor checks.

## Routing Trace

The routing trace schema is `elf.context_pack.routing_trace/v1`.

Each entry includes:

- `layer`
- `activation_state`
- `reason_code`
- `policy_reason`
- `manual_override`
- `pinned`
- `privacy`

Useful `activation_state` values include:

- `selected`
- `suppressed`
- `blocked`
- `not_requested`
- `pinned_ineligible`

Pinned but ineligible layers must be represented as suppressed or
`pinned_ineligible`; they must not expose unreadable source counts, refs, or content.

## Recall And Freshness

Context Pack assembly uses the Recall Debug service read model. The pack response
must include the underlying `elf.recall_trace/v1` projection so callers can inspect
selected, dropped, stale, blocked, suppressed, and not-requested context.

Pack item eligibility requires all of the following:

- the row selection state is `selected`, `available`, or `reviewable`;
- the row evidence class is `pass`;
- source refs or source snapshots are present;
- freshness is current, not deleted, deprecated, expired, stale, superseded,
  tombstoned, or historical;
- the underlying layer already applied read-profile, grant, lifecycle, redaction,
  and authority checks.

Rows that fail eligibility may remain visible in `recall_trace` when the underlying
Recall Debug contract allows it, but they must not become Context Pack items.

## Privacy

Context Pack debug output must not leak unreadable/private source existence, counts,
refs, or content. Public memory-note refs must resolve through active, unexpired,
readable `memory_notes` at read time before pack assembly. Deleted, deprecated,
expired, ungranted, or private rows must not be hydrated into pack `items`.

Blocked and not-requested states should remain typed evidence instead of being
collapsed into pass claims.
