# Getting Started

Purpose: Provide the canonical setup and local run flow for ELF.

## Prerequisites

- Postgres with `pgvector`.
- Qdrant (REST + gRPC endpoints).
- Provider endpoints for embeddings, rerank, and extraction.

## 1. Prepare config

Copy `elf.example.toml` to `elf.toml`, then set provider and storage values.

```sh
cp elf.example.toml elf.toml
```

Reference:

- Full configuration contract: `docs/spec/system_elf_memory_service_v2.md`.

## 2. Initialize storage

Initialize Postgres schema and Qdrant collection once.

```sh
psql "<dsn from elf.toml>" -f sql/init.sql

# Qdrant REST endpoint (default: 6333). In this repository's local setup, it is often mapped to 51889.
# ELF uses the gRPC endpoint at runtime (default: 6334, often mapped to 51890).
export ELF_QDRANT_HTTP_URL="http://127.0.0.1:51889"
export ELF_QDRANT_COLLECTION="mem_notes_v2"
export ELF_QDRANT_DOCS_COLLECTION="doc_chunks_v1"
export ELF_QDRANT_VECTOR_DIM="4096"
./qdrant/init.sh
```

## 3. Start services

Run each service in its own terminal.

```sh
cargo run -p elf-worker -- -c elf.toml
cargo run -p elf-api -- -c elf.toml
cargo run -p elf-mcp -- -c elf.toml
```

## 4. Run retrieval evaluation

Use `elf-eval` with your dataset.

```sh
cargo run -p elf-eval -- -c elf.toml -i path/to/eval.json
```

For dataset format and metric details, see `docs/guide/evaluation.md`.

## 5. Development workflow

Use `cargo make` tasks from repository root.

```sh
cargo make fmt
cargo make lint
cargo make test
cargo make test-integration
cargo make e2e
```

Notes:

- `cargo make test-integration` runs ignored tests that require external Postgres and Qdrant.
  Set `ELF_PG_DSN` and `ELF_QDRANT_GRPC_URL`.
- `cargo make e2e` runs the context misranking harness.
  Set `ELF_PG_DSN`, `ELF_QDRANT_GRPC_URL`, and `ELF_QDRANT_HTTP_URL`.

## Related guides

- Evaluation: `docs/guide/evaluation.md`
- Integration testing: `docs/guide/integration-testing.md`
- Test taxonomy: `docs/guide/testing.md`
- Agent setup: `docs/guide/agent-setup.md`
