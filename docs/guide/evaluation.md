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

## Context Misranking Harness

To measure cross-scope misranking before and after enabling context boosting, use the harness
script:

```bash
cargo make e2e
```

Or run the script directly:

```bash
scripts/context-misranking-harness.sh
```

What it does:

- Creates a dedicated database (default: `elf_e2e`).
- Creates a dedicated Qdrant collection for the run (default: `elf_harness_<run_id>`).
- Starts `elf-worker` and `elf-api` with deterministic local providers:
  - `providers.embedding.provider_id = "local"` (token-hash embedding).
  - `providers.rerank.provider_id = "local"` (token overlap rerank).
- Inserts two notes with identical text in different scopes (`org_shared` and `project_shared`),
  with importance configured to intentionally produce baseline misranking.
- Runs `elf-eval` twice:
  - Baseline: no `[context]`.
  - Context: `context.scope_descriptions` + `context.scope_boost_weight`.
- Prints `recall@1` and the top-ranked note ID for both runs, then deletes the notes.
- Deletes the dedicated database and collection unless `ELF_HARNESS_KEEP_DB=1` or
  `ELF_HARNESS_KEEP_COLLECTION=1` is set.

Prerequisites:

- Postgres is running and reachable.
- Qdrant is running and reachable.
- Environment variables are set:
  - `ELF_PG_DSN` (base DSN, typically ending in `/postgres`)
  - `ELF_QDRANT_URL` (Qdrant gRPC URL, commonly `http://127.0.0.1:51890` in this repository)
  - `ELF_QDRANT_HTTP_URL` (Qdrant REST URL, commonly `http://127.0.0.1:51889` in this repository)
- `psql`, `curl`, `taplo`, and `jaq` (or `jq`) are installed.

Configuration:

- Override the database name with `ELF_HARNESS_DB_NAME`.
- Override the run identifier with `ELF_HARNESS_RUN_ID`.
- Override the collection name with `ELF_HARNESS_COLLECTION` (must start with `elf_harness_`).
- Override the API binds with `ELF_HARNESS_HTTP_BIND`, `ELF_HARNESS_ADMIN_BIND`,
  and `ELF_HARNESS_MCP_BIND`.
