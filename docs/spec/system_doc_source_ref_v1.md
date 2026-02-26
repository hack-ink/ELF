# System: `doc_source_ref/v1` for `docs_put`

Purpose: define a minimal, versioned `source_ref` convention for docs ingested
through `POST /v2/docs` / MCP `elf_docs_put`.

Identifiers:
- Envelope identifier: `doc_source_ref/v1`
- File: `docs/spec/system_doc_source_ref_v1.md`

Scope:
- Covers `doc_documents.source_ref` for docs ingested via `docs_put`.
- Covers doc types: `knowledge`, `chat`, `search`, `dev`.
- This schema is for provenance and deterministic filtering keys, not for
  note-level evidence pointers (`source_ref/v1`).

`source_ref` is optional for `docs_put`. When omitted, the service persists an
JSON empty object (`{}`).

Design goals:
- Deterministic and replayable: two independent ingesters SHOULD emit identical
  keys for the same source event.
- Flat keys: fields SHOULD be top-level to support stable projection into vector
  payloads and filter indexes.
- Minimal requirements: the service MAY accept additional keys, but downstream
  filtering MUST rely only on keys enumerated by this spec.

==================================================
1) Top-level shape and required keys
==================================================

When `source_ref` is provided, it MUST be a JSON object with these required keys:

- `schema` (string): exact value `doc_source_ref/v1`.
- `doc_type` (string): one of `knowledge`, `chat`, `search`, `dev`.
- `ts` (string): RFC3339 timestamp for event time (not ingest time).

==================================================
2) Per-type required keys (minimal)
==================================================

All required fields are top-level.

--------------------------------------------------
2.1) `doc_type="chat"`
--------------------------------------------------

Required:
- `thread_id` (string): stable thread identifier.
- `role` (string): stable role marker (producer-defined). Examples: `user`, `assistant`, `tool`.

Optional (examples):
- `message_id` (string)

--------------------------------------------------
2.2) `doc_type="search"`
--------------------------------------------------

Required:
- `query` (string): literal query string.
- `url` (string): canonical URL for the selected result.
- `domain` (string): canonical domain for the URL, used as a stable filter key.

Optional (examples):
- `provider` (string)

--------------------------------------------------
2.3) `doc_type="dev"`
--------------------------------------------------

Required:
- `repo` (string): repository identifier (producer-defined; SHOULD be stable and human-readable).
- Exactly one of:
  - `commit_sha` (string)
  - `pr_number` (integer)
  - `issue_number` (integer)

Optional (examples):
- `path` (string): file path within the repo.

--------------------------------------------------
2.4) `doc_type="knowledge"`
--------------------------------------------------

Required:
- No additional required keys beyond section (1).

Optional:
- `uri` (string): canonical URI/path/URN for the knowledge source.

==================================================
3) Identifier stability and parsing rules
==================================================

The following fields are machine identifiers and MUST be byte-stable when
re-ingesting the same event:

- `schema`
- `doc_type`
- `thread_id`
- `domain`
- `repo`
- `commit_sha` / `pr_number` / `issue_number`

Timestamp rules:
- `ts` MUST be a timezone-aware RFC3339 datetime string.
- `ts` is the source event time. Do not use ingest time unless the source does
  not provide event time.

==================================================
4) Compatibility rules
==================================================

Forward compatibility:
- Producers MAY include additional keys.
- Consumers MUST ignore unknown keys.

Backward compatibility:
- Persisted docs MAY contain `{}` (no `source_ref`).
- Persisted docs MAY contain older producer-specific shapes. Consumers MUST
  treat such docs as "unfilterable by `doc_source_ref/v1` keys" unless a best-effort
  mapping is explicitly implemented.

==================================================
5) Examples
==================================================

Chat:

```json
{
  "schema": "doc_source_ref/v1",
  "doc_type": "chat",
  "ts": "2026-02-25T19:05:15Z",
  "thread_id": "thread-8f7e2f9a",
  "role": "assistant",
  "message_id": "message-1c3d"
}
```

Search:

```json
{
  "schema": "doc_source_ref/v1",
  "doc_type": "search",
  "ts": "2026-02-25T19:05:15Z",
  "query": "qdrant payload index keyword vs text",
  "url": "https://qdrant.tech/documentation/concepts/payload/",
  "domain": "qdrant.tech",
  "provider": "web"
}
```

Dev (commit):

```json
{
  "schema": "doc_source_ref/v1",
  "doc_type": "dev",
  "ts": "2026-02-25T19:05:15Z",
  "repo": "hack-ink/ELF",
  "commit_sha": "9f1f4e6d0a5b7c2e11c93b5a2c8a3f5e5a1b2c3d",
  "path": "packages/elf-service/src/docs.rs"
}
```

Dev (PR):

```json
{
  "schema": "doc_source_ref/v1",
  "doc_type": "dev",
  "ts": "2026-02-25T19:05:15Z",
  "repo": "hack-ink/ELF",
  "pr_number": 123
}
```

Knowledge:

```json
{
  "schema": "doc_source_ref/v1",
  "doc_type": "knowledge",
  "ts": "2026-02-25T19:05:15Z",
  "uri": "docs://kb/architecture/2026/02/overview"
}
```
