# Search Ranking Explain v2 (Additive Terms, v2-Only)

## Goal
Replace the ad-hoc map-based ranking explain payload with a structured, versioned schema that is stable under iteration and supports reliable evaluation and replay. This change is intentionally breaking. Existing v1 explain payloads and historical trace items are not preserved.

## Non-Goals
- Do not preserve backward compatibility with `search_ranking_explain/v1`.
- Provide a stage graph or non-additive scoring model.
- Expand retrieval or reranking behavior beyond the deterministic terms already tracked in issue work.

## Summary
The ranking explain payload becomes `search_ranking_explain/v2` and is defined as an additive decomposition:

- Invariant: `final_score == sum(terms[].value)`.
- Each term is a named scalar contribution.
- Term inputs are recorded only for persisted traces and evaluation, not in the hot-path search response.

The implementation uses a single scoring path for live search and for trace replay to prevent drift. Tie-breaking rules are explicit so repeated runs are stable when floating-point comparisons are equal.

## Schema
`SearchExplain` remains a two-part object:
- `match`: matched terms and fields.
- `ranking`: ranking breakdown.

`ranking` (v2):
- `schema`: `"search_ranking_explain/v2"`
- `policy_id`: stable policy identifier used for grouping and comparison.
- `final_score`: final score used for sorting.
- `terms`: ordered list of `{ name, value, inputs? }`

In search responses, `inputs` is omitted. In trace persistence and evaluation outputs, `inputs` is included for debugging and tuning.

## Data Persistence
`search_trace_items.explain` stores the v2 explain payload as JSON.

`search_trace_candidates` persists a `candidate_snapshot` JSON object that contains the minimum candidate fields required to replay ranking and compute deterministic terms without re-querying mutable database state. This supports future ranking signals without repeated schema churn.

## Terms
The initial v2 term set mirrors the current additive score components:
- `blend.retrieval`
- `blend.rerank`
- `tie_breaker`
- `context.scope_boost`
- `deterministic.lexical_bonus`
- `deterministic.hit_boost`
- `deterministic.decay_penalty`

Each term may record inputs, for example: weights, normalization kinds, ranks, overlap ratios, and hit statistics.

## Determinism and Tie-Breaks
Sorting is stable and deterministic:
1. `final_score` (descending)
2. `retrieval_rank` (ascending)
3. `note_id` (ascending)
4. `chunk_id` (ascending)

This ensures repeated runs and replay are consistent when scores collide.

## Testing
- Unit tests for additive term bounds and schema stability.
- Trace replay tests ensure the explain schema matches v2 and policy IDs remain stable.
