# Search Expansion and Multi-Query Fusion Design

## Overview

This document defines the retrieval architecture for multi-query expansion with configurable dynamic triggering and prefiltering. The goal is to maximize recall and robustness while keeping latency and cost predictable. The design preserves the existing API contract and treats expansion as an internal search policy controlled by configuration.

## Goals

- Improve recall through LLM-based query expansion and multi-query fusion.
- Keep a single public search API and avoid client-side changes.
- Allow a strict superset of existing behavior via configuration.
- Provide deterministic fallbacks when expansion fails.

## Non-goals

- Online learning from user feedback.
- Multilingual retrieval or translation.
- New external dependencies beyond the current provider stack.

## Architecture

The search pipeline becomes: input validation → scope resolution → optional dynamic pre-search → LLM expansion → multi-query hybrid retrieval → RRF fusion → optional prefilter → Postgres revalidation → single rerank → tie-break → top_k. The expansion step generates short English-only query variants and must return JSON with a queries array. The original query is always included unless explicitly disabled.

Dynamic mode performs a baseline hybrid retrieval on the original query, then triggers expansion when candidate_count is below a configured threshold or the top fusion score is below a threshold. Prefiltering is optional and can be disabled by setting max_candidates to 0 or a value greater than or equal to candidate_k, which makes the pipeline equivalent to the non-prefiltered design.

## Configuration

A new search configuration block is introduced:

- `search.expansion.mode`: `off|always|dynamic`
- `search.expansion.max_queries`: integer limit on expanded queries.
- `search.expansion.include_original`: boolean, default true.
- `search.dynamic.min_candidates`: minimum baseline candidates to avoid expansion.
- `search.dynamic.min_top_score`: minimum baseline fusion score to avoid expansion.
- `search.prefilter.max_candidates`: 0 or >= candidate_k disables prefiltering.

## Failure handling

If the LLM expansion call fails or returns invalid JSON, the system must fall back to the original query only. Any CJK output in expanded queries is dropped. If the expanded set becomes empty after filtering, the system must fall back to the original query.

## Testing

- Unit tests for expansion parsing, deduplication, and fallback behavior.
- Service tests for dynamic triggering and prefilter behavior.
- Integration test using the evaluation harness to validate recall and latency against the baseline.

## Rollout

Default configuration should keep behavior stable unless explicitly enabled. Operators can enable `dynamic` mode first, then move to `always` when cost and latency are acceptable.
