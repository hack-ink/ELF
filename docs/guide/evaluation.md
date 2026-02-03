# Retrieval Evaluation

Purpose: Provide a repeatable way to measure memory retrieval quality and prevent regressions.

## Tool

Use the `elf-eval` app to run an evaluation against a dataset of queries and expected note IDs.

Example:

```bash
cargo run -p elf-eval -- --config ./elf.toml --dataset ./docs/guide/eval-sample.json
```

## Dataset format

The dataset is JSON with optional defaults and a list of queries.

```json
{
  "name": "baseline",
  "defaults": {
    "tenant_id": "tenant-1",
    "project_id": "project-1",
    "agent_id": "agent-1",
    "read_profile": "all_scopes",
    "top_k": 12,
    "candidate_k": 60
  },
  "queries": [
    {
      "id": "q-1",
      "query": "where do we store embeddings",
      "expected_note_ids": [
        "11111111-1111-1111-1111-111111111111",
        "22222222-2222-2222-2222-222222222222"
      ]
    }
  ]
}
```

Each query supports these fields:

- `id` (optional): A human-friendly identifier for the query.
- `query` (required): The search query text.
- `expected_note_ids` (required): One or more note IDs expected in the results.
- `tenant_id`, `project_id`, `agent_id`, `read_profile` (optional): Override defaults.
- `top_k`, `candidate_k` (optional): Override defaults.

Resolution order for `top_k` and `candidate_k` is:

1. CLI flags (`--top-k`, `--candidate-k`)
2. Per-query overrides
3. Dataset defaults
4. `elf.toml` values

## Output

The command prints a JSON report containing summary metrics and per-query details:

- `avg_recall_at_k`
- `avg_precision_at_k`
- `mean_rr`
- `mean_ndcg`
- `latency_ms_p50` and `latency_ms_p95`

## Notes

- The evaluation tool uses the configured embedding and rerank providers.
- The dataset should avoid secrets and sensitive data.
