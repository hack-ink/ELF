# System: `source_ref` Doc Pointer Resolver (v1)

Purpose: Define a concrete, versioned `source_ref` schema for document pointers so agents can reliably hydrate long-form evidence after a note is retrieved.

Audience: LLM agents and implementers integrating ELF Core + Doc Extension v1.

Scope:
- This spec defines a `source_ref/v1` payload with `resolver = "elf_doc_ext/v1"`.
- It targets Doc Extension v1 (PG source of truth + bounded excerpt hydration).

Non-goals:
- Defining a translation pipeline.
- Defining non-ELF doc backends (S3/Git/threads/etc.). Those should use different `resolver` identifiers.

============================================================
1. Background
============================================================

ELF Core stores `source_ref` as an opaque JSON object and does not interpret it. Extensions and agents may interpret `source_ref` to hydrate supporting evidence on demand.

This spec standardizes one common case:

- A short English note in ELF Core references long-form evidence stored in Doc Extension v1.
- The note’s `source_ref` contains a stable pointer (doc_id + optional chunk_id + optional selector hints).
- When needed, an agent can call `docs_excerpts_get` and obtain a bounded excerpt plus verification signals.

============================================================
2. Identifiers (versioned)
============================================================

Envelope schema identifier:
- `schema = "source_ref/v1"`

Doc pointer resolver identifier (this spec):
- `resolver = "elf_doc_ext/v1"`

============================================================
3. Data model (normative)
============================================================

### 3.1 Top-level object

The `source_ref` object MUST be a JSON object and MUST include:

- `schema` (string): `"source_ref/v1"`
- `resolver` (string): `"elf_doc_ext/v1"`
- `ref` (object): stable document identifiers (see 3.2)

The `source_ref` object MAY include:

- `state` (object): integrity and snapshot fields (see 3.3)
- `locator` (object): excerpt selector hints (see 3.4)
- `hashes` (object): optional integrity checks (see 3.5)
- `hints` (object): optional UX/debug fields (see 3.6)

All keys and string values SHOULD be ASCII-safe and stable over time.

### 3.2 `ref` (required)

`ref` MUST include:

- `doc_id` (string): UUID of the document in Doc Extension v1.

`ref` MAY include:

- `chunk_id` (string): UUID of a specific chunk. Use when the pointer came from `docs_search_l0`.

Notes:
- `doc_id` is the canonical lookup key for hydration.
- `chunk_id` is an optional anchor that can help choose a small search neighborhood.

### 3.3 `state` (optional but recommended)

`state` MAY include:

- `content_hash` (string): blake3 hex of the authoritative document content bytes as stored by Doc Extension v1.
- `chunk_hash` (string): blake3 hex of the authoritative chunk text (when `ref.chunk_id` is present).
- `doc_updated_at` (string): RFC3339 timestamp. Informative for debugging and cache keys.

If provided, these fields allow agents to detect drift and to report stronger provenance.

### 3.4 `locator` (optional)

`locator` carries excerpt selector hints. The canonical selector vocabulary is:

- `quote` (object): `TextQuoteSelector` with:
  - `exact` (string, required)
  - `prefix` (string, optional)
  - `suffix` (string, optional)
- `position` (object): `TextPositionSelector` with:
  - `start` (integer, required)
  - `end` (integer, required)

Rules:
- When both `quote` and `position` are present, agents SHOULD prefer `quote` and treat `position` as a fallback.
- `position` is byte-offset based (UTF-8), and is more brittle under content edits than `quote`.

Optional fields:
- `level` (string): `"L1"` or `"L2"` as a suggested excerpt size tier for hydration. If omitted, agents should choose based on context budget.

### 3.5 `hashes` (optional)

`hashes` MAY include:

- `content_hash` (string): same meaning as `state.content_hash` (duplicated here to support simpler consumers).
- `excerpt_hash` (string): blake3 hex of a previously-hydrated excerpt, when the agent wants to pin a specific excerpt payload.

Notes:
- `excerpt_hash` is only meaningful when the hydration request (selector + level) is stable and replayable.
- Doc Extension v1 returns `content_hash` and `excerpt_hash` along with `verified` and `verification_errors`.

### 3.6 `hints` (optional)

`hints` MAY include:

- `title` (string)
- `uri` (string): canonical location (informative; not required for dereference)
- `mime_type` (string)

These fields are convenience-only and MUST NOT be used as the sole dereference mechanism for this resolver.

============================================================
4. Hydration procedure (informative)
============================================================

Given a note with:

- `source_ref.schema = "source_ref/v1"`
- `source_ref.resolver = "elf_doc_ext/v1"`

An agent typically hydrates evidence by calling:

- `docs_excerpts_get` with:
  - `doc_id` from `ref.doc_id`
  - optional `chunk_id` from `ref.chunk_id`
  - optional selector hints from `locator.quote` and/or `locator.position`
  - `level` from `locator.level` or an agent default

The agent SHOULD:

- Prefer excerpts with `verification.verified = true`.
- Preserve `content_hash` and `excerpt_hash` returned by Doc Extension v1 when storing derived facts or when building audit trails.

============================================================
5. English-only boundary interaction (normative)
============================================================

- ELF Core note fields (`notes[].text`, `notes[].key`, and other natural-language fields) MUST comply with the English-only boundary defined by the ELF Memory Service v2 spec.
- Doc Extension v1 MAY store original long-form evidence; agents should store English facts in ELF notes and keep originals in docs.
- `source_ref` pointers are metadata and MAY contain identifiers/URIs that are not English sentences.

============================================================
6. Examples (informative)
============================================================

### 6.1 Minimal doc pointer (doc_id only)

```json
{
  "schema": "source_ref/v1",
  "resolver": "elf_doc_ext/v1",
  "ref": {
    "doc_id": "6b5b2f08-9a89-4c6c-9b6b-9c0c2f0b1f2d"
  }
}
```

### 6.2 Pointer anchored to a chunk (from docs_search_l0)

```json
{
  "schema": "source_ref/v1",
  "resolver": "elf_doc_ext/v1",
  "ref": {
    "doc_id": "6b5b2f08-9a89-4c6c-9b6b-9c0c2f0b1f2d",
    "chunk_id": "b2e8a8d2-4c10-4a1b-98f8-7a8702fd0cc1"
  },
  "state": {
    "content_hash": "baf7cfd2d5b71f5b0f5d5a08a3c38d7b43cf7a2e5a4f75d5c1b4a9072f6dd3b8",
    "chunk_hash": "bd85b0e07464bde3a7f3a2b2f3c2d5d4c1c9f0d0c1a2b3c4d5e6f7a8b9c0d1e2"
  }
}
```

### 6.3 Pointer with quote + fallback position selector

```json
{
  "schema": "source_ref/v1",
  "resolver": "elf_doc_ext/v1",
  "ref": {
    "doc_id": "6b5b2f08-9a89-4c6c-9b6b-9c0c2f0b1f2d"
  },
  "locator": {
    "level": "L1",
    "quote": {
      "exact": "Deployment steps for service.",
      "prefix": "Fact: ",
      "suffix": "\\n"
    },
    "position": {
      "start": 1234,
      "end": 1262
    }
  }
}
```

