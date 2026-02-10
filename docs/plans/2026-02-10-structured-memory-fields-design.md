# Structured Memory Fields With Field-Level Embeddings (Issue #17)

## Goal
Improve semantic precision on fact-like queries by adding optional structured fields to notes (summary, facts, concepts), embedding them separately, and merging field matches back into a single note result with explicit explain output.

This change is additive to the existing chunk-first retrieval design and does not require a graph database.

## Data Model
Add a normalized structured-field table and a derived embedding table:

- `memory_note_fields`: One row per note field item (`summary`, `fact`, `concept`) with `item_index` for ordering.
- `note_field_embeddings`: One embedding vector per field row and embedding version. This table is derived and must be rebuildable from Postgres data.

The canonical human-readable note remains `memory_notes.text`.

## Write Semantics
- `add_note` remains deterministic. Structured fields are optional input. When provided:
  - `facts` must be evidence-bound deterministically (either a substring of the note text, or a substring of any `source_ref.evidence[].quote` strings when provided).
- `add_event` extractor output may include `structured`. Evidence binding remains strict:
  - `facts` must be supported by the extracted evidence quotes.
- Structured field changes enqueue an indexing outbox `UPSERT` so the worker regenerates field embeddings.

## Indexing
The worker embeds both chunk texts and structured field texts in the same embedding batch, then writes:
- chunk vectors to `note_chunk_embeddings` and pooled vectors to `note_embeddings` (existing behavior),
- field vectors to `note_field_embeddings` (new behavior).

## Retrieval And Explain
Retrieval remains chunk-first via Qdrant hybrid search. In addition:
- Perform a Postgres vector search over `note_field_embeddings` to retrieve additional note candidates and record which fields matched (`summary`, `facts`, `concepts`).
- For field-only candidates, select a representative chunk via Postgres similarity over chunk embeddings so results remain chunk-shaped.

Explain output includes `matched_fields` entries for matched structured fields.

## Testing And Evaluation
- Unit tests cover structured-field validation and evidence binding for facts.
- Add a small evaluation dataset focused on fact-like queries and run `elf-eval` before/after enabling structured-field retrieval to compare precision and false positives.

