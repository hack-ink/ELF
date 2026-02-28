# System: `doc_chunking_profiles/v1` for `docs_put`

Purpose: define token-based chunking profiles used by Doc Extension v1 ingestion.

Identifiers:
- Envelope identifier: `doc_chunking_profiles/v1`
- File: `docs/spec/system_doc_chunking_profiles_v1.md`

Scope:
- Applies to `POST /v2/docs` (`docs_put`) chunking behavior in `apps/elf-service/src/docs.rs`.
- Profiles are selected by `doc_type`.

Design goals:
- Deterministic chunking across ingesters when `doc_type` and input text are equal.
- Token-based boundaries to avoid byte-length split artifacts in Unicode/UTF-8 text.
- Small overlap to preserve continuity at boundaries.

==================================================
1) Profile matrix
==================================================

The following profile values are used unless overridden by a future `*_v2` contract:

| `doc_type` | `max_tokens` | `overlap_tokens` |
|------------|--------------|------------------|
| `chat`     | 256          | 32               |
| `search`   | 384          | 64               |
| `dev`      | 768          | 128              |
| `knowledge`| 1024         | 128              |

==================================================
2) Validation rules
==================================================

Each profile must satisfy:
- `max_tokens > 0`
- `overlap_tokens >= 0`
- `overlap_tokens < max_tokens`

==================================================
3) Compatibility rules
==================================================

Forward compatibility:
- Consumers may accept additional profile keys or optional extension metadata.
- Unknown profile metadata is ignored by core chunking behavior.

Backward compatibility:
- This profile set is normative for `doc_chunking_profiles/v1`.
- Clients must not invent alternative `max_tokens`/`overlap_tokens` values for these `doc_type` values without introducing a new version identifier.
