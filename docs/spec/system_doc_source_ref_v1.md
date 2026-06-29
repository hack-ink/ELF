---
type: Spec
title: "System: `doc_source_ref/v1` for `docs_put`"
description: "Normative contract for source_ref values accepted by docs_put."
resource: docs/spec/system_doc_source_ref_v1.md
status: active
authority: normative
owner: spec
last_verified: 2026-06-23
tags:
  - docs
  - spec
source_refs: []
code_refs:
  - apps/elf-mcp/src/app/server/tools/docs.rs
  - packages/elf-service/src/docs.rs
  - packages/elf-service/src/knowledge.rs
  - packages/elf-storage/src/docs.rs
related:
  - docs/runbook/privacy_delete_export.md
drift_watch:
  - docs/spec/system_doc_source_ref_v1.md
  - packages/elf-service/src/docs.rs
  - packages/elf-service/src/knowledge.rs
---
# System: `doc_source_ref/v1` for `docs_put`

Purpose: Define a minimal, versioned `source_ref` convention for docs ingested
through `POST /v2/docs` / MCP `elf_docs_put`.
Status: normative
Read this when: You are producing or validating `source_ref` payloads for `docs_put`.
Not this document: Note-level evidence pointers or retrieval-time document pointer resolution.
Defines: `doc_source_ref/v1`.

Identifiers:
- Envelope identifier: `doc_source_ref/v1`
- File: `docs/spec/system_doc_source_ref_v1.md`

Scope:
- Covers `doc_documents.source_ref` for docs ingested via `docs_put`.
- Covers doc types: `knowledge`, `chat`, `search`, `dev`.
- This schema is for provenance and deterministic filtering keys, not for
  note-level evidence pointers (`source_ref/v1`).

`source_ref` is required for `docs_put` and must conform to this spec.
Legacy `{}` or non-`doc_source_ref/v1` shapes are rejected for `docs_put`.

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
- This contract is strict for `docs_put` writes. Backward-compatible fallback
  mappings are not performed.

==================================================
5) Source Library profile
==================================================

`doc_source_ref/v1` also defines a first-class Source Library profile for
saved long-form material. This profile is opt-in: a payload enters the profile
when it provides any Source Library field below. Once it enters the profile,
the required profile keys below MUST be present and valid.

Required Source Library profile keys:

- `source_kind` (string): one of `article`, `social_thread`, `pdf`,
  `text_export`, `repo_file`, `chat_excerpt`, or `web_page`.
- `canonical_uri` (string): stable URL, URN, file URI, repo URI, or source
  identifier that can be used for deduplication and operator inspection.
- `captured_at` (string): timezone-aware RFC3339 timestamp for when ELF
  captured the source.
- `trust_label` (string): one of `trusted`, `user_captured`, `public_web`,
  `third_party`, or `unverified`.

Optional Source Library profile keys:

- `source_created_at` (string): timezone-aware RFC3339 source publication or
  creation time when available.
- `author` (string): author or source display name when available.
- `handle` (string): stable social/repository/source handle when available.
- `source_content_hash` (string): producer-supplied source hash when available.
  ELF also stores and returns its own canonical `content_hash` for the persisted
  document bytes.
- `excerpt_locator` (object): selector hints for the saved source. It MAY
  include:
  - `quote`: object with required `exact` and optional `prefix`/`suffix`.
  - `position`: object with integer `start` and `end` byte offsets, where
    `start < end`.

Compatibility with `doc_type`:

- `source_kind = "social_thread"` and `source_kind = "chat_excerpt"` require
  `doc_type = "chat"`.
- `source_kind = "repo_file"` requires `doc_type = "dev"`.
- Other source kinds may use the normal `knowledge` or `search` document
  classes based on caller workflow.

Boundary:

- Source Library ingest stores a document and document chunks. It MUST NOT
  create or mutate durable Memory Notes unless the caller separately invokes an
  explicit memory-write or reviewed promotion path.

Normalized capture output:

- `docs_put` MUST return `source_capture.schema = "doc_source_capture/v1"`.
- `source_capture.source_record_id` MUST equal the stored `doc_documents.doc_id`.
- `source_capture.origin` MUST be the canonical source origin used for operator
  inspection and deduplication. Source Library `canonical_uri` takes precedence
  over legacy URL, URI, thread, search, or repo-derived origins.
- `source_capture.captured_at` MUST be the Source Library `captured_at` value
  when present. If the Source Library profile is not active, the service may use
  the service capture timestamp.
- `source_capture.content_hash` MUST be the BLAKE3 hex hash of the persisted
  document content after write-policy transforms.
- `source_capture.visibility_scope` MUST be the document scope.
- `source_capture.title` SHOULD be copied from the request title when present.
- `source_capture.source_type` MUST be `source_kind` when present, otherwise the
  normalized `doc_type`.
- `source_capture.source_spans` MUST list stable span references for persisted
  chunks.
- `source_capture.policy_spans` MUST list excluded or redacted spans when
  write-policy hooks remove or transform source content.

Stable source records and spans:

- `doc_documents.doc_id` is the Source Library source record id for captured
  docs. It MUST be deterministic for the same tenant, effective project, agent,
  scope, doc type, source identity, and persisted content hash.
- Persisted chunk ids MUST be deterministic for the same source record id and
  chunk index.
- Captured source span ids MUST be deterministic for the same persisted content
  hash, byte offsets, and span status.
- Captured span offsets are byte offsets into the persisted document content.
- Policy span offsets are byte offsets into the original request content before
  write-policy transforms.

`doc_source_span/v1` fields:

- `schema` (string): exact value `doc_source_span/v1`.
- `span_id` (string UUID): stable span identifier.
- `chunk_id` (string UUID, optional): present for persisted captured chunks.
- `status` (string): `captured`, `excluded`, or `redacted`.
- `reason_code` (string, optional): required for non-captured spans.
- `start_offset` and `end_offset` (integers): byte offsets, with
  `start_offset <= end_offset`.
- `content_hash` (string): BLAKE3 hex hash for the content the offsets address.
- `chunk_hash` (string, optional): BLAKE3 hex hash for captured chunk text.

Typed policy span reasons:

- Excluded spans MUST use `reason_code = "WRITE_POLICY_EXCLUSION"`.
- Redacted spans MUST use `reason_code = "WRITE_POLICY_REDACTION"`.
- Unsupported or policy-removed content MUST be represented through a typed span
  reason or a typed validation error. It MUST NOT disappear silently from Source
  Library audit surfaces.

Persisted normalized `source_ref`:

- The stored `doc_documents.source_ref` MUST retain the caller-provided
  `doc_source_ref/v1` fields and add normalized capture fields:
  `source_record_id`, `origin`, `captured_at`, `content_hash`,
  `visibility_scope`, `source_type`, and `source_spans`.
- When policy spans exist, stored `doc_documents.source_ref` MUST include
  `policy_spans`.
- Normalized capture fields are evidence metadata only. They MUST NOT promote a
  source record into approved Memory Authority.

Delete, export, and private-span boundary:

- Source Library direct reads, L0 search, excerpt hydration, and derived
  Knowledge Workspace search MUST resolve only active source documents and chunks
  readable under the caller's scope context.
- Deleting or deactivating a Source Library document makes its document and chunk
  refs non-recallable. Derived pages may retain stored stale text until rebuild, but
  page search MUST suppress snippets whose source refs no longer resolve to active
  readable document or chunk rows.
- `doc_source_span/v1` entries with `status = "excluded"` or `status = "redacted"`
  are audit evidence for write-policy handling. They MUST NOT be treated as captured
  source evidence for derived page search, memory promotion, graph facts, or export
  payloads that claim to contain current recallable source text.
- Export of Source Library material is an authorized API readback of current
  source rows and payload levels. It MUST NOT bypass scope, document status,
  write-policy spans, or source visibility rules.

==================================================
6) Examples
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

Source Library article:

```json
{
  "schema": "doc_source_ref/v1",
  "doc_type": "knowledge",
  "ts": "2026-06-20T01:10:00Z",
  "source_kind": "article",
  "canonical_uri": "https://example.com/research/agent-memory-os",
  "captured_at": "2026-06-20T01:10:00Z",
  "source_created_at": "2026-06-19T21:00:00Z",
  "trust_label": "public_web",
  "author": "Example Research Group",
  "excerpt_locator": {
    "quote": {
      "exact": "source libraries preserve long-form evidence"
    },
    "position": {
      "start": 0,
      "end": 128
    }
  }
}
```

Source Library social thread:

```json
{
  "schema": "doc_source_ref/v1",
  "doc_type": "chat",
  "ts": "2026-06-20T02:00:00Z",
  "thread_id": "thread-agent-knowledge-os",
  "role": "user",
  "source_kind": "social_thread",
  "canonical_uri": "https://example.com/thread/agent-knowledge-os",
  "captured_at": "2026-06-20T02:00:00Z",
  "source_created_at": "2026-06-20T01:45:00Z",
  "trust_label": "public_web",
  "author": "Example Builder",
  "handle": "example-builder"
}
```
