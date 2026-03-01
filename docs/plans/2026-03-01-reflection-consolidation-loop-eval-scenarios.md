# Reflection & Consolidation Loop: Evaluation Scenarios

## Decision

For issue #79 we define consolidation as an **agent-side policy** and keep **scoring and API behavior as server-side capability**.

The agent decides when to consolidate (`query + merge policy`), while `elf-api`/`elf-worker` only provide:

- append and update semantics,
- duplicate de-duplication rules when configured by service config,
- query retrieval/search behavior,
- and deterministic evaluation primitives for measuring outcomes.

This keeps consolidation policies under LLM-agent control (and easy to evolve) without introducing a separate long-lived service.

## Tradeoff

- **Pros**
  - Faster product iteration: policy thresholds, scoring windows, and trigger conditions can change per-agent workflow without backend deployment.
  - Better portability: consolidation behavior can be reused by different local agents with minimal API changes.
  - Smaller server surface: only stable capabilities and guarantees stay in the shared API.
- **Cons**
- Additional policy logic in clients increases implementation variance across agents.
- Requires explicit evaluation to prevent silent regressions when policies change.

## Evaluation Scenario

### Consolidation stability scenario

Problem: a single logical key has multiple noisy legacy notes. Before consolidation, query results are spread; after deduplication and creation of one canonical note, retrieval should become both more stable and more deterministic.

Harness behavior:

- ingest 3 duplicate notes with key `incident_merge_protocol` and distractor notes,
- run `elf-eval` with dataset query expectation by `expected_keys`,
- perform a consolidation action (delete duplicates, ingest canonical stable note),
- run the same query again.

Success signal:

- baseline and post-consolidation recall remain healthy,
- post-consolidation `retrieved_keys` is focused and stable,
- change in `avg_retrieved_summary_chars` is visible to detect summary-quality drift.

## Why `expected_keys` is required

Consolidation changes note IDs; `expected_note_ids` assertions are brittle under those flows.
`expected_keys` allows intent-based assertions that survive ID churn and still validates semantic coverage through the new `expected_keys` mode in `elf-eval`.
