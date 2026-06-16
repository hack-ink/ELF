# Reviewable Memory Summary v1 Specification

Purpose: Define the reviewable memory summary and source-trace contract.
Status: normative
Read this when: You are implementing, validating, or reviewing summary readback for top-of-mind, background, stale, superseded, tombstoned, or derived project-profile memory.
Not this document: Scheduled background jobs, polished viewer UI, live provider generation, or authoritative note mutation.
Defines: `elf.memory_summary/v1` summary artifacts, entries, source traces, freshness markers, and inclusion rationale.

## Core Rule

Memory summaries are derived readback artifacts. They must never replace, rewrite,
delete, deprecate, or silently update authoritative notes, docs, event audits, graph
facts, consolidation proposals, traces, or source pointers.

Postgres remains the source of truth for source memory. A summary may be rebuilt,
discarded, archived, or regenerated without changing the source memory that produced
it. A summary is useful only when an operator can inspect why each entry is current,
background, stale, superseded, tombstoned, or excluded.

## Contract Schema

Canonical schema identifier:

```text
elf.memory_summary/v1
```

Every persisted or benchmarked summary artifact must carry
`contract_schema = "elf.memory_summary/v1"`.

## Summary Artifact

Required fields:

- `summary_id`: stable summary artifact id.
- `contract_schema`: `elf.memory_summary/v1`.
- `generated_at`: RFC3339 timestamp for the readback artifact.
- `tenant_id`, `project_id`, `agent_id`, and `read_profile`: context used to build the
  readback.
- `entries`: non-empty array of summary entries.
- `source_trace`: source selection and exclusion metadata.

The artifact may include provider metadata in future lanes, but v1 summary readback
does not require provider execution and must not hide source selection behind provider
state.

## Entry Categories

`entries[].category` must be one of:

- `top_of_mind`: current high-priority memory that may be attached or shown first.
- `background`: current lower-priority memory that is useful context but not urgent.
- `stale`: non-current memory retained only to explain why it is stale.
- `superseded`: historical memory replaced by newer source evidence.
- `tombstone`: delete, TTL, invalidation, or suppression evidence.
- `derived_project_profile`: derived profile or project-summary entry.

`top_of_mind` entries must have `freshness.status = "current"`. A stale,
superseded, tombstoned, historical, unsupported, or unknown entry must not be surfaced
as top-of-mind.

## Entry Contract

Each summary entry must include:

- `entry_id`: stable id within the summary.
- `category`: one of the categories above.
- `text`: bounded English summary text.
- `source_refs`: source evidence ids or source-ref handles used for the entry.
- `freshness`: validity metadata.
- `rationale`: inclusion, downgrade, or exclusion rationale.
- `unsupported_claim_flags`: reviewer prompts for claims that are not supported well
  enough to include as current derived memory.

`source_refs` must be non-empty for every included or downgraded entry. A
`derived_project_profile` entry may have empty `source_refs` only when
`rationale.decision = "excluded"` and `unsupported_claim_flags` is non-empty. That
shape records a refused derived claim, not a usable memory entry.

## Freshness

`freshness` must include:

- `status`: one of `current`, `background`, `historical`, `stale`, `superseded`,
  `tombstoned`, or `unsupported`.
- `observed_at`: RFC3339 timestamp when the source was observed, or `null` when the
  source is intentionally untimed.
- `valid_from`: RFC3339 timestamp or `null`.
- `valid_to`: RFC3339 timestamp or `null`.
- `last_confirmed_at`: RFC3339 timestamp or `null`.
- `superseded_by`: array of entry ids or source ids that supersede this entry.
- `tombstone_refs`: array of source ids or source-ref handles proving deletion, TTL
  expiry, invalidation, or suppression.

For `category = "superseded"`, `freshness.superseded_by` must be non-empty.
For `category = "tombstone"`, `freshness.tombstone_refs` must be non-empty and
`freshness.status` must be `tombstoned`.

## Rationale

`rationale` must include:

- `decision`: one of `included`, `downgraded`, or `excluded`.
- `reason_code`: stable code for why the entry appears in its category.
- `reason`: reviewer-facing explanation.

Allowed reason-code families:

- `TOP_OF_MIND_*`
- `BACKGROUND_*`
- `DOWNGRADED_STALE_*`
- `SUPERSEDED_*`
- `TOMBSTONE_*`
- `DERIVED_PROFILE_*`
- `EXCLUDED_UNSUPPORTED_*`

The rationale must say why an entry is included, downgraded, or excluded. It is not
enough to say that an entry exists.

## Source Trace

`source_trace` must include:

- `selected_source_refs`: sources used for included or downgraded entries.
- `dropped_source_refs`: candidates not used in the final summary.
- `stale_source_refs`: stale source candidates and their downgrade reason.
- `superseded_source_refs`: superseded sources and the source that superseded them.
- `tombstone_source_refs`: tombstone or TTL invalidation sources.
- `unsupported_claim_flags`: page-level or entry-level unsupported derived claims.

Each source trace item should preserve source status, source `updated_at` or
equivalent freshness timestamp when available, and source snapshot metadata. Empty
trace arrays are allowed only when the category is absent from the summary.

## Readback Rules

Summary readback must:

- Label the artifact as derived and reviewable.
- Return entries with source refs, freshness metadata, and rationale.
- Preserve current-vs-historical truth: current facts may be top-of-mind, while old
  facts must be stale, superseded, tombstoned, or excluded.
- Preserve tombstones and TTL invalidations as suppression evidence instead of
  restating the deleted fact as current.
- Preserve unsupported derived candidates as reviewer prompts, not as current facts.

Summary readback must not:

- Present a stale, superseded, or tombstoned source as current top-of-mind memory.
- Treat a derived profile entry as authoritative source memory.
- Omit source refs from included or downgraded entries.
- Include a derived project-profile entry with neither source refs nor unsupported
  claim flags.
- Claim parity with managed memory or Dreaming products from this local contract alone.

## Benchmark Requirements

The `memory_summary` real-world benchmark suite must fail when:

- stale, superseded, or tombstoned entries appear as current top-of-mind facts;
- included or downgraded entries lack source refs;
- entries lack freshness or rationale metadata;
- derived project-profile entries lack both source refs and unsupported-claim flags;
- unsupported derived claims are silently included as current memory.

Unsupported derived claims may appear only as reviewer prompts. A summary entry with
`unsupported_claim_flags` must not also be included as current memory.

Fixture-backed evidence proves only the contract shape. Live top-of-mind behavior and
scheduled background generation require separate live reports before product-quality
claims are allowed.
