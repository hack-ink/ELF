# Getting Started

Goal: Provide the canonical setup and local run flow for ELF.
Read this when: You are bootstrapping a local ELF environment or resetting a broken one.
Inputs: This repository checkout, Docker Compose for local dependencies, and optional provider credentials.
Depends on: `Makefile.toml`, `docker-compose.yml`, `config/local/elf.docker.toml`, `elf.example.toml`, and the relevant service binaries.
Verification: Configuration is in place and the local ELF stack can start successfully.

## Prerequisites

- Docker Compose for the local dependency stack, or separately managed Postgres with `pgvector` and Qdrant.
- Rust toolchain from `rust-toolchain.toml`.
- Provider endpoints only when you are testing provider-backed embeddings, rerank, query expansion, or `add_event`.

## 1. Start local dependencies

Validate and start the local Postgres and Qdrant services.
The checked-in Compose file is local-development-only:

- Postgres: `127.0.0.1:51888`, database `elf_local`, user `elf_dev`, password `elf_dev_password`.
- Qdrant REST: `127.0.0.1:51889`.
- Qdrant gRPC: `127.0.0.1:51890`.
- Data lives in Docker volumes `elf-postgres-data` and `elf-qdrant-data`.

```sh
docker compose -f docker-compose.yml config >/dev/null
docker compose -f docker-compose.yml up -d postgres qdrant
docker compose -f docker-compose.yml ps
```

## 2. Choose config

For local dependency smoke tests, use the checked-in Docker config directly:

```sh
config/local/elf.docker.toml
```

This config is strict-valid, binds only to loopback, uses the local deterministic embedding and rerank providers, disables LLM query expansion, and contains only placeholder provider keys. Do not use `add_event` with this config until you replace `[providers.llm_extractor]` with a real local or external extractor.

For provider-backed development, copy `elf.example.toml` to `elf.toml`, then set provider and storage values.

```sh
cp elf.example.toml elf.toml
```

Reference:

- Full configuration contract: `docs/spec/system_elf_memory_service_v2.md`.

## 3. Start services

Run each service in its own terminal from the repository root.
`elf-api` and `elf-worker` auto-create the Postgres schema, the Qdrant memory/docs collections, and docs payload indexes during startup.

```sh
cargo run -p elf-api -- -c config/local/elf.docker.toml
```

```sh
cargo run -p elf-worker -- -c config/local/elf.docker.toml
```

Optional MCP server:

```sh
cargo run -p elf-mcp -- -c config/local/elf.docker.toml
```

If you are using `elf.toml` instead, replace `config/local/elf.docker.toml` with `elf.toml`.

## 4. Inspect API contract

After `elf-api` starts, the API process serves:

- `GET /openapi.json` for the generated OpenAPI contract.
- `GET /docs` for the Scalar API reference UI.
- `GET /viewer` on the admin bind for the local read-only search, note, and trace viewer.

Use the host and port from `service.http_bind` in your config.
For example:

```sh
curl -fsS http://127.0.0.1:51892/openapi.json
open http://127.0.0.1:51892/docs
```

Use the host and port from `service.admin_bind` for the viewer.
For the checked-in local config:

```sh
open http://127.0.0.1:51891/viewer
```

## 5. Smoke the local stack

```sh
curl -fsS http://127.0.0.1:51892/health
```

Run a deterministic `add_note` smoke that does not call any LLM provider:

```sh
curl -fsS -X POST http://127.0.0.1:51892/v2/notes/ingest \
  -H 'content-type: application/json' \
  -H 'X-ELF-Tenant-Id: local-tenant' \
  -H 'X-ELF-Project-Id: local-project' \
  -H 'X-ELF-Agent-Id: local-agent' \
  -d '{
    "scope": "agent_private",
    "notes": [
      {
        "type": "fact",
        "key": "local_compose_stack",
        "text": "The local ELF development stack runs Postgres with pgvector and Qdrant through Docker Compose.",
        "importance": 0.7,
        "confidence": 0.9,
        "ttl_days": 14,
        "source_ref": {"schema": "local_smoke/v1", "ref": {"command": "docs/guide/getting_started.md"}}
      }
    ]
  }'
```

## 6. Run retrieval evaluation

Use `elf-eval` with your dataset.

```sh
cargo run -p elf-eval -- -c elf.toml -i path/to/eval.json
```

For dataset format and metric details, see `docs/guide/evaluation.md`.

## 7. Run local checks

With the Compose dependencies running, the context misranking harness can use the same local dependency ports:

```sh
ELF_PG_DSN="postgres://elf_dev:elf_dev_password@127.0.0.1:51888/postgres" \
ELF_QDRANT_GRPC_URL="http://127.0.0.1:51890" \
ELF_QDRANT_HTTP_URL="http://127.0.0.1:51889" \
ELF_HARNESS_VECTOR_DIM=256 \
cargo make e2e
```

## 8. Development workflow

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
- Stop local dependencies with `docker compose -f docker-compose.yml down`.
  Add `-v` only when you intentionally want to delete the local development volumes.

## Related guides

- Evaluation: `docs/guide/evaluation.md`
- Integration testing: `docs/guide/integration-testing.md`
- Test taxonomy: `docs/guide/testing.md`
- Agent setup: `docs/guide/agent-setup.md`
