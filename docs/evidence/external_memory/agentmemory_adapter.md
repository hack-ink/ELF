---
type: Evidence
title: "Agentmemory Fixture Adapter"
description: "Evidence record for the agentmemory fixture adapter boundary."
resource: docs/evidence/external_memory/agentmemory_adapter.md
status: active
authority: current_state
owner: evidence
last_verified: 2026-06-18
tags:
  - docs
  - evidence
  - external_memory
---
# Agentmemory Fixture Adapter

Goal: Convert sanitized agentmemory-style session exports into ELF-owned note/doc
candidates and retrieval baseline records.
Read this when: You need to compare coding-agent memory capture against ELF without
running an agentmemory server or bypassing ELF ingestion.
Inputs: A local JSON fixture with agentmemory-style sessions, observations, memories,
and retrieval cases.
Depends on: `elf-eval`, `docs/decisions/2026-06-08-agent-memory-selection.md`,
`docs/spec/system_elf_memory_service_v2.md`, `docs/spec/system_doc_source_ref_v1.md`,
and `docs/spec/system_source_ref_doc_pointer_v1.md`.
Outputs: A deterministic `elf.agentmemory_adapter/v1` JSON bundle with note candidates,
doc candidates, baseline queries, and ignored-item reasons.

## Boundary

The adapter is an offline comparison/import boundary, not an ingestion path.
It does not call agentmemory, ELF HTTP APIs, providers, Postgres, Qdrant, or any LLM.
It only rewrites a sanitized fixture into records that can later be reviewed, grouped,
and submitted through normal ELF endpoints.

Use this boundary when the question is:

- Which agentmemory memories are plausible ELF note candidates?
- Which raw observations should be retained as document evidence?
- Which retrieval cases can become ELF evaluation datasets after candidate notes are
  ingested through `/v2/notes/ingest`?

Do not use it to claim that ELF reproduces agentmemory benchmark numbers. Fixture
retrieval cases preserve agentmemory result ranks and scores as external baseline
metadata only.

## Command

Run the adapter through `cargo run`:

```sh
cargo run -p elf-eval --bin agentmemory_fixture_adapter -- \
  --fixture apps/elf-eval/fixtures/agentmemory/sample_session.json \
  --out tmp/agentmemory-adapter.json
```

Optional flags:

- `--scope`: ELF write scope attached to emitted note and doc candidates. Defaults to
  `agent_private`.
- `--max-note-chars`: maximum accepted note length before a memory is reported as
  ignored. Defaults to `240`, matching the canonical local config limit.

## Fixture Shape

The fixture is intentionally small and producer-owned. It should use this shape:

```json
{
  "schema": "agentmemory.fixture/v1",
  "fixture_id": "agentmemory-sample-2026-06-08",
  "source": {
    "system": "agentmemory",
    "version": "v0.9.27",
    "export_id": "agentmemory-export-sample",
    "exported_at": "2026-06-08T06:30:00Z"
  },
  "sessions": [
    {
      "session_id": "am-session-2026-06-08",
      "agent": "codex",
      "project": "ELF",
      "started_at": "2026-06-08T05:45:00Z",
      "observations": [],
      "memories": [],
      "retrieval_cases": []
    }
  ]
}
```

The checked-in sample fixture is sanitized and exists only to exercise the mapping.
External exports must be reviewed for secrets and sensitive content before being
committed or shared.

## Mapping

Agentmemory memories become `note_candidates` only when all of these are true:

- `kind` maps directly to one ELF note type: `preference`, `constraint`, `decision`,
  `profile`, `fact`, or `plan`.
- `text` is non-empty and does not exceed `--max-note-chars`.
- `importance` and `confidence`, when present, are finite values in `0.0..=1.0`.

The emitted `notes_ingest_item` is shaped like a single `/v2/notes/ingest` note item.
It includes a `source_ref/v1` envelope with `resolver = "agentmemory_fixture/v1"` and
stable origin fields:

- fixture id
- session id
- memory id
- source observation ids
- source system/version
- export, session, and memory timestamps

The adapter does not infer missing ELF note types, does not truncate text, and does not
rewrite memory text into a canonical note sentence.

Agentmemory observations become `doc_candidates` when they have non-empty text and an
RFC3339 timestamp from the observation, session, or export. The emitted `docs_put`
payload uses:

- `doc_type = "chat"`
- `source_ref.schema = "doc_source_ref/v1"`
- `thread_id = session_id`
- `message_id = observation_id`
- `role` from the observation role, observation kind, or `observation`

This keeps raw session evidence separate from authoritative ELF notes. If operators
later ingest docs and want hydrated note evidence, they should attach normal
`elf_doc_ext/v1` doc pointers after `docs_put` returns concrete `doc_id` values.

Retrieval cases become `baseline_queries` when at least one expected memory id maps to
a note candidate. The baseline record preserves:

- query id and query text
- expected agentmemory memory ids
- deterministic note candidate ids
- expected note keys, when available
- agentmemory result ranks/scores, when present

These records are suitable for building an ELF eval dataset after candidate notes are
ingested through ELF policy. They are not benchmark proof on their own.

## Ignored Items

The adapter reports ignored items instead of repairing them. Current reasons include:

- `empty_text`
- `missing_or_invalid_timestamp`
- `note_text_too_long`
- `unsupported_memory_kind`
- `invalid_importance`
- `invalid_confidence`
- `no_mapped_expected_memories`

Ignored items can still be reviewed manually. Do not force them into ELF notes by
loosening the adapter; either fix the fixture upstream or store long/ambiguous evidence
as docs and use normal ELF extraction/review workflows.

## Comparing Retrieval Quality

Use a two-step comparison:

1. Review the adapter output and ingest selected `notes_ingest_item` records through
   `/v2/notes/ingest`, grouped by scope. ELF write policy, English gate, provenance
   validation, duplicate/update resolution, and indexing still run normally.
2. Convert selected `baseline_queries` into the `elf-eval` dataset format. Prefer
   `expected_keys` when keys were emitted; otherwise resolve ingested note IDs and use
   `expected_note_ids`.

Then run `elf-eval` as usual:

```sh
cargo run -p elf-eval -- -c ./elf.toml --dataset tmp/agentmemory-eval.json
```

For config-to-config comparisons or trace replay, follow `docs/runbook/evaluation.md`.

## Verification

Run the adapter fixture test without network services:

```sh
cargo test -p elf-eval --test agentmemory_fixture_adapter
```

Before review handoff for changes to this boundary, run the repository gate from
`Makefile.toml`.
