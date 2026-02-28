# Observability and Correlation (MCP + Admin API)

Purpose: Provide a practical traceability workflow for agents and operators.

## 1) Request correlation

Every ELF response returns:

- `X-ELF-Request-Id` response header.
- `request_id` field in JSON responses.

In `elf-mcp`, each tool call carries `X-ELF-Request-Id` automatically:

- `X-ELF-Request-Id` is generated per call.
- The same value is available in the tool response body as `request_id` (when JSON).

Correlation workflow:

1. Capture `request_id` from the JSON response (or header if present).
2. Use the same identifier for logs, incident notes, and trace lookups.

## 2) Admin provenance lookup

For a note-level traceability trail:

- MCP tool: `elf_admin_note_provenance_get`
  - `{"note_id": "<uuid>"}`
- Equivalent HTTP endpoint:
  - `GET /v2/admin/notes/{note_id}/provenance`
  - Schema: `elf.note_provenance_bundle/v1`

Returned bundle sections:

- `note`
- `ingest_decisions`
- `note_versions`
- `indexing_outbox`
- `recent_traces`

Use this bundle to answer:

- Why a note exists or changed.
- Whether indexing/outbox is stalled.
- Which recent searches touched the note.

## 3) Worker traceability fields

For background job diagnostics, filter worker logs with these fields:

- `outbox_id` (indexing/doc indexing/trace outbox jobs).
- `note_id` (note indexing jobs).
- `doc_id`, `chunk_id` (doc indexing jobs).
- `trace_id` (search trace outbox jobs).

Recommended loop:

1. Start from a user-facing error `trace_id` or note `note_id`.
2. Query `elf_admin_trace_*` family to inspect trajectory and trace items.
3. Use `elf_admin_note_provenance_get` to connect trace history with ingest and indexing state.

## 4) MCP admin/debug surface map

- `elf_admin_traces_recent_list` -> `GET /v2/admin/traces/recent`
- `elf_admin_trace_get` -> `GET /v2/admin/traces/{trace_id}`
- `elf_admin_trajectory_get` -> `GET /v2/admin/trajectories/{trace_id}`
- `elf_admin_trace_item_get` -> `GET /v2/admin/trace-items/{item_id}`
- `elf_admin_trace_bundle_get` -> `GET /v2/admin/traces/{trace_id}/bundle`
- `elf_admin_note_provenance_get` -> `GET /v2/admin/notes/{note_id}/provenance`
