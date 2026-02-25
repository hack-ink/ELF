# System: `doc_source_ref/v1` for `docs_put`

Purpose: define a stable `source_ref` envelope for `POST /v2/docs` / `elf_docs_put`.

Identifiers:
- Envelope identifier: `doc_source_ref/v1`
- File: `docs/spec/system_doc_source_ref_v1.md`

Scope:
- Covers `source_ref` carried by docs records ingested through `docs_put`.
- Covers source kinds: `chat`, `search`, `dev`, `knowledge`.
- This schema is for provenance and retrieval correlation, not for note-level evidence
  pointers (`source_ref/v1`).

`source_ref` is optional for `docs_put`; when omitted, the service persists an empty JSON object.

==================================================
1) Top-level shape and required keys
==================================================

When `source_ref` is provided, it MUST be a JSON object with these required keys:

- `schema` (string): exact value `doc_source_ref/v1`.
- `source` (string): one of `chat`, `search`, `dev`, `knowledge`.
- `ref` (object): stable external identifiers and canonical lookup hints.

--------------------------------------------------
`ref` object (required)
--------------------------------------------------

`ref` MUST contain:

- `id` (string): stable source identifier.

`ref` MAY contain:

- `uri` (string): canonical URI/path/URN into the source system.
- `keys` (object): stable key/value pairs used for exact lookup.

--------------------------------------------------
Optional top-level keys
--------------------------------------------------

- `locator` (object): optional source-specific location hints.
  - `page` (integer), `line` (integer), or other numeric position hints.
- `state` (object): optional snapshot fields such as `version` or `last_seen`.
- `meta` (object): optional, source-specific enrichment fields.

==================================================
2) Source-specific recommendation notes
==================================================

For producers, include `ref.id` plus at least one source-specific hint in
`ref.keys` when available:

- `chat`: `thread_id`, `message_id`, `speaker` (optional).
  - `speaker` is opaque metadata and is not enumerated by this spec or by service
    validation. Emit a stable role marker that your producers understand (for example,
    `user` or `assistant`).
- `search`: `query_id`, `result_id`, `provider`.
- `dev`: `project`, `repo`, `branch`, `file`, `commit`.
- `knowledge`: `knowledge_base`, `entry_id`, `section_id`.

==================================================
3) Identifier stability and NLP/LID rules
==================================================

The following fields are machine identifiers and must be stable over time:

- `schema`
- `source`
- `ref.id`
- `ref.uri`
- `ref.keys.*`

Do not apply NLP/LID checks to these identifier/URI/key fields.
They must be byte-stable identifiers, not natural-language content.

==================================================
4) Examples
==================================================

Chat:

```json
{
  "schema": "doc_source_ref/v1",
  "source": "chat",
  "ref": {
    "id": "thread-8f7e2f9a/message-1c3d",
    "uri": "chat://tenant-a/project-b/thread-8f7e2f9a",
    "keys": {
      "thread_id": "thread-8f7e2f9a",
      "message_id": "message-1c3d"
    }
  },
  "meta": {
    "speaker": "agent"
  }
}
```

Search:

```json
{
  "schema": "doc_source_ref/v1",
  "source": "search",
  "ref": {
    "id": "search-result-7b4a",
    "uri": "search://tenant-a/project-b/query/7b4a/result/3",
    "keys": {
      "query_id": "7b4a",
      "result_id": "d9a1"
    }
  }
}
```

Dev:

```json
{
  "schema": "doc_source_ref/v1",
  "source": "dev",
  "ref": {
    "id": "ingest-dev-2026-02-25",
    "keys": {
      "project": "tenant-a/project-b",
      "repo": "core-engine",
      "branch": "main",
      "commit": "9f1f4e6"
    }
  }
}
```

Knowledge:

```json
{
  "schema": "doc_source_ref/v1",
  "source": "knowledge",
  "ref": {
    "id": "kb-entry-2026-02",
    "uri": "docs://kb/architecture/2026/02",
    "keys": {
      "knowledge_base": "architecture",
      "entry_id": "2026-02",
      "section_id": "overview"
    }
  }
}
```
