# Agent Setup Guide

This guide is written for AI agents helping a human operator install and run ELF locally with minimal back-and-forth.
It assumes you have access to this repository checkout.

## What You Are Setting Up

ELF is a Rust workspace that typically runs:

- `elf-api`: HTTP API service.
- `elf-worker`: background worker that indexes notes into Qdrant.
- `elf-mcp` (optional): an MCP server that forwards to `elf-api`.
- `elf-eval` (optional): an evaluation tool for retrieval quality.

ELF requires:

- Postgres with `pgvector` (source of truth).
- Qdrant (derived index; safe to rebuild).

Important: The ELF config has no implicit defaults. All required config fields must be explicitly present in your TOML.

## Minimal Owner Inputs (Ask These)

Ask the owner for:

1. Postgres DSN for the target database (for example `postgres://user:pass@host:5432/elf`).
2. Qdrant endpoints:
   - REST base URL (default Qdrant REST: `http://127.0.0.1:6333`).
   - gRPC base URL (default Qdrant gRPC: `http://127.0.0.1:6334`).
3. Provider choices:
   - Embedding provider config.
   - Rerank provider config.
   - LLM extractor provider config (required by config; only needed at runtime if the operator uses `add_event` or other LLM-backed features).
4. Whether `elf-api` should bind only to loopback, and whether to enable API/admin auth tokens.

If the owner cannot provide provider endpoints/keys yet, you can still run a local-only development setup for embedding and rerank by setting:

- `providers.embedding.provider_id = "local"`
- `providers.rerank.provider_id = "local"`

Then set `search.expansion.mode = "off"` to avoid LLM-backed query expansion. The extractor config must still be present and non-empty, but should not be used in this mode.

## Prerequisites

The machine must have:

- Rust toolchain (pinned by `rust-toolchain.toml`).
- `psql` available on PATH.
- Running Postgres instance with `pgvector` installed/enabled.
- Running Qdrant instance.

For the repository harness scripts:

- `curl`
- `jq` or `jaq`
- `taplo`

## Create The Config

1. Copy the template:

```sh
cp elf.example.toml elf.toml
```

2. Edit `elf.toml`:

- Set `[storage.postgres].dsn` to your Postgres DSN.
- Set `[storage.qdrant].url` to your Qdrant gRPC base URL.
- Set `[storage.qdrant].collection` to a collection name (for example `mem_notes_v2`).
- Ensure `[chunking].tokenizer_repo` is a non-empty Hugging Face tokenizer repo name (for example `gpt2`).
- Fill all `[providers.*]` blocks. Keys must be non-empty strings.
- If binding `elf-api` to a non-loopback address, set `security.api_auth_token` to a non-empty value.

## Initialize Storage

1. Initialize Postgres schema:

```sh
psql "<dsn from elf.toml>" -f sql/init.sql
```

2. Initialize the Qdrant collection (REST):

```sh
export ELF_QDRANT_HTTP_URL="http://127.0.0.1:6333"
export ELF_QDRANT_COLLECTION="mem_notes_v2"
export ELF_QDRANT_VECTOR_DIM="4096"
./qdrant/init.sh
```

Notes:

- Qdrant REST and gRPC ports often differ. The `ELF_QDRANT_HTTP_URL` above must be the REST base URL.
- `storage.qdrant.url` in `elf.toml` must be the gRPC base URL.
- The Qdrant vector dimension must match the embedding dimension configured in `elf.toml`.

## Start Services

Start each in a separate terminal:

```sh
cargo run -p elf-worker -- -c elf.toml
cargo run -p elf-api -- -c elf.toml
```

Optional:

```sh
cargo run -p elf-mcp -- -c elf.toml
```

## Verify

```sh
curl -fsS http://127.0.0.1:51892/health
```

Adjust the port to match `service.http_bind`.

## Run E2E Harness (Optional)

The context misranking harness creates and drops a dedicated database and Qdrant collection. It requires:

- `ELF_PG_DSN` (a base DSN that typically ends with `/postgres`)
- `ELF_QDRANT_URL` (Qdrant gRPC base URL)
- `ELF_QDRANT_HTTP_URL` (Qdrant REST base URL)

Example:

```sh
ELF_PG_DSN="postgres://postgres:postgres@127.0.0.1:51888/postgres" \
ELF_QDRANT_URL="http://127.0.0.1:51890" \
ELF_QDRANT_HTTP_URL="http://127.0.0.1:51889" \
cargo make e2e
```

## Troubleshooting

- Config parse errors:
  - ELF config has no implicit defaults. Fix missing fields in the TOML (the error message will name the missing field).
- API never becomes healthy:
  - Check the API log and confirm Postgres and Qdrant are reachable.
- Qdrant collection errors:
  - Confirm the REST URL is correct, and rerun `./qdrant/init.sh`.
