# System: Note Provenance Mapping (v1)

Purpose: Define the provenance bundle contract used by admin operations and traceability workflows.

Identifier:
- `elf.note_provenance_bundle/v1`

Status: active.

==================================================
Scope
==================================================

- Defines the response contract for `/v2/admin/notes/{note_id}/provenance`.
- Captures the same note-level artifacts needed for auditability and debugging:
  - source note state
  - ingest decisions
  - note version history
  - indexing outbox state
  - recent traces involving the note
- Does not define any mutation semantics.

==================================================
1) Endpoint contract
==================================================

`GET /v2/admin/notes/{note_id}/provenance`

This admin endpoint returns a single JSON object that **must** use:

```json
{
  "schema": "elf.note_provenance_bundle/v1",
  "note": { ... },
  "ingest_decisions": [...],
  "note_versions": [...],
  "indexing_outbox": [...],
  "recent_traces": [...]
}
```

Headers:
- `X-ELF-Request-Id` is accepted and echoed via response body `request_id` plus response header.
- Standard admin headers from section 14 apply.

`note` fields are a copy of the requested note with:

- core fields (`note_id`, `tenant_id`, `project_id`, `agent_id`, `scope`, `type`, `status`, ...),
- `source_ref` and `embedding_version`,
- `hit_count` / `last_hit_at`.

`ingest_decisions` is joined from `memory_ingest_decisions` by:
- `note_id`, `tenant_id`, `project_id`
and ordered by `ts DESC`.

`note_versions` is joined from `memory_note_versions` by:
- `note_id`, `tenant_id`, `project_id`
and ordered by `ts DESC`.

`indexing_outbox` is joined from `indexing_outbox` by:
- `note_id`, `tenant_id`, `project_id`
and ordered by `updated_at DESC`.

`recent_traces` is joined from:
- `search_traces` and `search_trace_items`
where the trace references the note id, ordered by `created_at DESC, trace_id DESC`.

==================================================
2) Response field shape
==================================================

Core envelope:

- `schema` (string, required): `elf.note_provenance_bundle/v1`.
- `note` (object, required): note snapshot for the requested `note_id`.
- `ingest_decisions` (array, required): ordered ingest audit entries.
- `note_versions` (array, required): ordered historical versions.
- `indexing_outbox` (array, required): active/retry indexing jobs for the note.
- `recent_traces` (array, required): bounded traces involving this note.

No additional top-level keys are required by this contract.

==================================================
3) MCP exposure
==================================================

MCP tool:

- `elf_admin_note_provenance_get` -> `GET /v2/admin/notes/{note_id}/provenance`

Request input:

```json
{
  "note_id": "uuid"
}
```

==================================================
4) Operational guidance
==================================================

- Keep `recent_traces` small (bounded by service defaults) to avoid large admin payloads.
- Use this endpoint for:
  - explainability investigation,
  - evidence lineage checks,
  - outbox lag/metadata checks before manual remediation.

