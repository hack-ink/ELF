# Trace-Based Ranking Harness: Next Steps

## Context

We have laid the groundwork for trace-based ranking evaluation:

- Traces persist `top_k` items with `final_score` and explain breakdown.
- Optional persistence of the full `candidate_k` set is available via `search_trace_candidates`.
- Trace persistence supports `write_mode = "outbox" | "inline"` for production throughput vs evaluation ergonomics.
- `elf-eval` emits `trace_id` (and `trace_ids` for repeated runs) and supports request-scoped `ranking` overrides.

This document records the next work to deliver a full, reproducible, policy-comparison loop.

## Goal

Provide a fast and reproducible harness that can:

1. Load the exact candidate set from stored traces.
2. Recompute rankings for multiple policy variants on the same candidates.
3. Produce stable metrics and a machine-readable report for diffing and regression gates.

## Non-Goals (For V1)

- No web UI dashboard.
- No ML training (LTR).
- No “live” candidate retrieval re-execution for comparison (the source of truth is the stored candidate set).

## Work Items

1. Add a trace-based compare mode to `elf-eval`.
   - Input options:
     - A list of `trace_id`s.
     - A dataset of queries that includes `trace_id` per query.
   - Output:
     - Stability metrics (top-k overlap, positional churn, set churn).
     - Guardrails (retention of baseline retrieval rank 1–3, if available).
     - Per-trace policy snapshot and per-item score decomposition.

2. Implement a pure “re-rank from candidates” function in `elf-service` (library-only).
   - Inputs: candidate rows (including retrieval rank and rerank score), config snapshot or override.
   - Output: ordered results with the same explain schema (`search_ranking_explain/v2`).
   - Constraints:
     - Must not touch Qdrant, providers, or caches.
     - Must be deterministic for a given input set.

3. Add a stable `policy_id` derived from the policy snapshot.
   - Compute a canonical JSON snapshot of policy parameters.
   - Derive `policy_id` as a short hash (for example, `blend_v1:<hash>`).
   - Store `policy_id` in trace config snapshot and explain outputs to enable automatic grouping.

4. Ensure candidate capture is sufficient for planned ranking signals.
   - Audit what future policies need (diversity, lexical overlap, hit reinforcement, decay).
   - Add only the minimal additional columns required for policy recomputation.
   - Avoid large JSON fields unless they are required for correctness.

5. Define operational defaults for production vs evaluation.
   - Production:
     - `write_mode = "outbox"`.
     - `capture_candidates = false` by default.
   - Evaluation:
     - `write_mode = "inline"` (no worker dependency).
     - `capture_candidates = true` (for policy replay).
   - If production capture is desired, add sampling (for example, 1%) and/or allowlist gates.

## Acceptance Criteria

- Given a fixed list of `trace_id`s, the harness can compare two policy variants and print stability deltas.
- Policy comparisons are reproducible without running Qdrant or external providers.
- The report includes enough detail to explain regressions (policy snapshot and per-term breakdown).

## Risks / Open Questions

- Storage growth if `capture_candidates` is enabled broadly in production.
- Some future signals may require additional inputs that are not currently persisted.
- Inline trace writes increase request latency and should remain evaluation-focused by default.
