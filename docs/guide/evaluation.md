# Retrieval Evaluation

Purpose: Provide a repeatable way to measure memory retrieval quality and prevent regressions.

## Tool

Use the `elf-eval` app to run an evaluation against a dataset of queries and expected note IDs.

Example:

```bash
cargo run -p elf-eval -- -c ./elf.toml --dataset ./docs/guide/eval-sample.json
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
- `ranking` (optional): A request-scoped ranking override (for example, `ranking.blend.enabled`,
  `ranking.blend.segments`, or normalization settings).

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
- `queries[].trace_id` (and `queries[].trace_ids` when `runs_per_query > 1`) for trace-based replay.

## Notes

- The evaluation tool uses the configured embedding and rerank providers.
- The dataset should avoid secrets and sensitive data.
- To persist traces for later replay without running `elf-worker`, set `search.explain.write_mode = "inline"`
  in the config used by `elf-eval`.
- To compare ranking policies on a fixed candidate set without re-running Qdrant, use trace compare mode:
  - Run: `cargo run -p elf-eval -- -c ./elf.a.toml --config-b ./elf.b.toml --trace-id <uuid1> <uuid2>`
  - Requirements: `search.explain.capture_candidates = true` when generating traces, and candidates must not be
    expired by `search.explain.candidate_retention_days`.

## CI Trace Regression Gate

CI runs a trace regression gate to catch unintended ranking changes on a fixed candidate set.

What it checks:

- Replays ranking from stored `search_trace_candidates` for each `trace_id` (no Qdrant or external providers).
- Compares the replayed top-k `note_id`s against the baseline `search_trace_items` for the same trace.
- Enforces thresholds from a gate JSON file:
  - `max_positional_churn_at_k` and `max_set_churn_at_k`.
  - `min_retrieval_top_rank_retention` (retention over candidates with `retrieval_rank <= retrieval_retention_rank`).
- Fails if the baseline or replay returns fewer than `top_k` items.

Run locally:

```bash
# Load the CI fixture into a local Postgres database.
psql "postgres://postgres:postgres@127.0.0.1:5432/elf" -v ON_ERROR_STOP=1 -f sql/init.sql
psql "postgres://postgres:postgres@127.0.0.1:5432/elf" -v ON_ERROR_STOP=1 -f .github/fixtures/trace_gate/fixture.sql

# Run the gate (reads Postgres DSN from the config).
cargo run -p elf-eval --bin trace_regression_gate -- \
  -c .github/fixtures/trace_gate/config.toml \
  -g .github/fixtures/trace_gate/gate.json \
  --out tmp/trace-regression-gate.report.json
```

Update baseline:

- Re-record the baseline trace items/candidates with the intended baseline build/config, regenerate the fixture,
  then update the gate JSON (trace IDs and thresholds) used by CI.

Artifacts:

- The gate outputs a JSON report (stdout, or the `--out` file) with per-trace metrics and any breached thresholds.

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
  - `ELF_QDRANT_GRPC_URL` (Qdrant gRPC URL, commonly `http://127.0.0.1:51890` in this repository)
  - `ELF_QDRANT_HTTP_URL` (Qdrant REST URL, commonly `http://127.0.0.1:51889` in this repository)

Operational notes:

- The harness builds once and then starts `elf-worker` and `elf-api` by executing `target/debug/...`.
  If you are running the services manually, prefer `cargo build` plus direct binary execution over
  running multiple `cargo run` processes concurrently, which can lead to Cargo lock contention and
  slow startup.
- If the health check does not become ready, inspect `tmp/elf.harness.api.log` and
  `tmp/elf.harness.worker.log` for the first startup error.
- `psql`, `curl`, `taplo`, and `jaq` (or `jq`) are installed.

## Ranking Stability Harness

To empirically measure rank churn reduction from deterministic ranking terms, use the harness
script:

```bash
ELF_PG_DSN="postgres://postgres:postgres@127.0.0.1:51888/postgres" \
ELF_QDRANT_GRPC_URL="http://127.0.0.1:51890" \
ELF_QDRANT_HTTP_URL="http://127.0.0.1:51889" \
scripts/ranking-stability-harness.sh
```

What it does:

- Creates a dedicated database and Qdrant collection for the run.
- Ingests a synthetic dataset with many near-tied candidates.
- Enables a local noisy rerank model to simulate reranker instability.
- Compares `elf-eval` stability metrics with deterministic ranking disabled vs enabled.

Configuration:

- Control rerank noise with `ELF_HARNESS_NOISE_STD`.
- Control stability sampling with `ELF_HARNESS_RUNS_PER_QUERY`.
- Control ranking cutoffs with `ELF_HARNESS_TOP_K` and `ELF_HARNESS_CANDIDATE_K`.

Configuration:

- Override the database name with `ELF_HARNESS_DB_NAME`.
- Override the run identifier with `ELF_HARNESS_RUN_ID`.
- Override the collection name with `ELF_HARNESS_COLLECTION` (must start with `elf_harness_`).
- Override the API binds with `ELF_HARNESS_HTTP_BIND`, `ELF_HARNESS_ADMIN_BIND`,
  and `ELF_HARNESS_MCP_BIND`.
