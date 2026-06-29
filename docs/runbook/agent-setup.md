---
type: Runbook
title: "Agent Setup Runbook"
description: "Help an agent install and run ELF locally with minimal back-and-forth."
resource: docs/runbook/agent-setup.md
status: active
authority: procedural
owner: runbook
last_verified: 2026-06-23
tags:
  - docs
  - runbook
code_refs:
  - Makefile.toml
  - scripts/local-agent-loop.sh
  - config/local/elf.docker.toml
related:
  - docs/runbook/getting_started.md
  - docs/runbook/agent_skills_cookbook.md
  - docs/evidence/2026-06-23-local-agent-loop-drift-audit.md
drift_watch:
  - Makefile.toml
  - scripts/local-agent-loop.sh
  - config/local/elf.docker.toml
  - apps/elf-api/src/routes.rs
  - apps/elf-mcp/src/app/server.rs
---
# Agent Setup Runbook

Goal: Help an agent install and run ELF locally with minimal back-and-forth.
Read this when: You need a practical local setup flow from an existing repository checkout.
Inputs: This repository checkout plus Docker Compose or separately managed Postgres/Qdrant, and optional provider credentials.
Depends on: `Makefile.toml`, `docker-compose.yml`, `config/local/elf.docker.toml`, `elf.example.toml`, and `docs/runbook/getting_started.md`.
Verification: ELF services start, required dependencies are reachable, and the local workflow can continue.

This runbook is written for AI agents helping a human operator install and run ELF locally with minimal back-and-forth.
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

## One-Command Local Agent Loop

Use this path when the operator wants a deterministic install-to-first-value loop
without external provider credentials:

```sh
cargo make local-agent-loop
```

The command runs `scripts/local-agent-loop.sh`, which:

- Starts the checked-in Docker Compose Postgres and Qdrant services.
- Builds and starts `elf-api`, `elf-worker`, and `elf-mcp` with
  `config/local/elf.docker.toml`.
- Imports a Source Library document through `POST /v2/docs`.
- Writes a deterministic source note through `POST /v2/notes/ingest`.
- Creates a reviewable consolidation proposal through
  `POST /v2/admin/consolidation/runs`.
- Applies reviewer approval through
  `POST /v2/admin/consolidation/proposals/{proposal_id}/review`.
- Recalls the approved memory through `POST /v2/searches`.
- Inspects the recall/debug panel through `POST /v2/recall-debug/panel`.
- Supersedes and restores the promoted memory through
  `POST /v2/admin/notes/{note_id}/corrections`.

The script writes request and response artifacts to `tmp/local-agent-loop/`.
It stops the background ELF service processes when the demo exits, but it leaves the
Docker dependency containers running for normal local reuse. Stop dependencies with:

```sh
docker compose -f docker-compose.yml down
```

To run only the lifecycle demo against an already running local API, worker, and admin
API:

```sh
scripts/local-agent-loop.sh demo
```

This local loop is deterministic. It uses `elf_notes_ingest` and a manually supplied
review proposal; it does not call `elf_events_ingest`, query expansion, or a hosted
LLM extractor.

## Minimal Owner Inputs

For the checked-in Docker local stack, no owner inputs are required. Use `docker-compose.yml`
and `config/local/elf.docker.toml` from `docs/runbook/getting_started.md`.

For separately managed dependencies or provider-backed development, ask the owner for:

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
- Docker Compose for the checked-in local dependency stack, or separately running Postgres and Qdrant.
- `psql` available on PATH.
- Running Postgres instance with `pgvector` installed/enabled when not using Compose.
- Running Qdrant instance when not using Compose.

For the repository harness scripts:

- `curl`
- `jq` or `jaq`
- `taplo`

The one-command local agent loop additionally uses:

- `cargo`
- `docker`

## Create The Config

For the checked-in Docker local stack, use the strict-valid local config directly:

```sh
config/local/elf.docker.toml
```

For provider-backed development, copy the template:

```sh
cp elf.example.toml elf.toml
```

Then edit `elf.toml`:

- Set `[storage.postgres].dsn` to your Postgres DSN.
- Set `[storage.qdrant].url` to your Qdrant gRPC base URL.
- Set `[storage.qdrant].collection` to a collection name (for example `mem_notes_v2`).
- Ensure `[chunking].tokenizer_repo` is a non-empty Hugging Face tokenizer repo name (for example `gpt2`).
- Fill all `[providers.*]` blocks. Keys must be non-empty strings.
- Set `security.auth_mode` explicitly:
  - Use `"off"` only for local loopback development.
  - Use `"static_keys"` with non-empty `security.auth_keys` for authenticated access (`Authorization: Bearer <token>`).

## Initialize Storage

For the checked-in Docker local stack, start dependencies and then start `elf-api` or
`elf-worker`; the services auto-create the Postgres schema and Qdrant collections.

```sh
docker compose -f docker-compose.yml up -d postgres qdrant
```

When using separately managed Qdrant and you need to pre-create collections before
service startup, initialize them through the REST endpoint:

```sh
export ELF_QDRANT_HTTP_URL="http://127.0.0.1:6333"
export ELF_QDRANT_COLLECTION="mem_notes_v2"
export ELF_QDRANT_DOCS_COLLECTION="doc_chunks_v1"
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
cargo run -p elf-worker -- -c config/local/elf.docker.toml
cargo run -p elf-api -- -c config/local/elf.docker.toml
```

Optional:

```sh
cargo run -p elf-mcp -- -c config/local/elf.docker.toml
```

Replace `config/local/elf.docker.toml` with `elf.toml` when using a provider-backed config.

## Verify

```sh
curl -fsS http://127.0.0.1:51892/health
```

Adjust the port to match `service.http_bind`.

## Agent Integration Recipes

Use the same local config for each agent client unless the operator has created a
provider-backed `elf.toml`.

### Codex

Run the local stack first:

```sh
cargo make local-agent-loop
```

For an interactive Codex session, register an MCP server that starts ELF MCP from this
repository checkout:

```json
{
  "mcpServers": {
    "elf-local": {
      "command": "cargo",
      "args": ["run", "-p", "elf-mcp", "--", "-c", "config/local/elf.docker.toml"],
      "cwd": "<repo-root>"
    }
  }
}
```

Use the configured MCP tools for the agent-facing loop:

- `elf_docs_put` to store long-form source evidence.
- `elf_notes_ingest` to store a compact deterministic memory candidate.
- `elf_searches_create` to recall approved memory.
- `elf_recall_debug_panel` to inspect selected, dropped, stale, blocked, and
  not-requested context.

Review and correction routes are admin HTTP operations in the current local recipe:

- `POST /v2/admin/consolidation/runs`
- `POST /v2/admin/consolidation/proposals/{proposal_id}/review`
- `POST /v2/admin/notes/{note_id}/corrections`

### Claude, Cursor, And MCP-Style Coding Agents

For clients that accept a Claude/Cursor-style MCP server JSON block, use the same
server command:

```json
{
  "mcpServers": {
    "elf-local": {
      "command": "cargo",
      "args": ["run", "-p", "elf-mcp", "--", "-c", "config/local/elf.docker.toml"],
      "cwd": "<repo-root>"
    }
  }
}
```

If the client does not support `cwd`, run it from the repository root or replace the
config argument with the absolute path to the local config. Keep the MCP server
loopback-only for this runbook.

### Generic MCP Clients

Start the MCP bridge directly:

```sh
cargo run -p elf-mcp -- -c config/local/elf.docker.toml
```

The local MCP config supplies tenant, project, agent, and read-profile headers:

- `tenant_id = "local-tenant"`
- `project_id = "local-project"`
- `agent_id = "local-agent"`
- `read_profile = "private_plus_project"`

Do not let clients override `read_profile` for search, document search, or recall
debug. The MCP adapter strips client-supplied read-profile parameters for those
agent-facing tools.

### CLI Workflow

Use HTTP when you need full admin review or correction operations from a shell:

```sh
export ELF_HTTP=http://127.0.0.1:51892
export ELF_ADMIN=http://127.0.0.1:51891
export ELF_TENANT=local-tenant
export ELF_PROJECT=local-project
export ELF_AGENT=local-agent
export ELF_READ_PROFILE=private_plus_project
```

All local requests use the same context headers:

```sh
-H "X-ELF-Tenant-Id: ${ELF_TENANT}" \
-H "X-ELF-Project-Id: ${ELF_PROJECT}" \
-H "X-ELF-Agent-Id: ${ELF_AGENT}" \
-H "X-ELF-Read-Profile: ${ELF_READ_PROFILE}"
```

For exact request bodies, inspect the artifacts written by:

```sh
cargo make local-agent-loop
```

## Minimal Memory And Knowledge Loop

The first-value loop has six checkpoints:

1. Import source evidence into Doc Extension v1 with `elf_docs_put` or
   `POST /v2/docs`.
2. Propose memory as a reviewable, source-linked candidate. The deterministic local
   path uses `elf_notes_ingest` plus a manual consolidation proposal; a provider-backed
   path may use `elf_events_ingest` after the extractor provider is configured.
3. Approve and apply the proposal through the admin consolidation review route.
4. Recall the approved memory through `elf_searches_create` or `POST /v2/searches`.
5. Inspect recall/debug through `elf_recall_debug_panel` or
   `POST /v2/recall-debug/panel`.
6. Correct and rollback through `POST /v2/admin/notes/{note_id}/corrections` with
   `action = "supersede"` followed by `action = "restore"`.

Source records and reviewable proposals are not mutated by recall or correction. The
correction route changes the memory note lifecycle and writes version history so the
memory can be restored.

## Requirements And Unsupported Paths

Required for the deterministic local loop:

- Rust toolchain from `rust-toolchain.toml`.
- Docker Compose services from `docker-compose.yml`: Postgres with `pgvector` and
  Qdrant.
- `curl`.
- `jq` or `jaq`.
- Loopback binds from `config/local/elf.docker.toml`.

Optional provider-backed paths:

- Replace `config/local/elf.docker.toml` with a complete `elf.toml` when using hosted
  or local model providers.
- Configure `[providers.llm_extractor]` before using `elf_events_ingest`.
- Set `search.expansion.mode` to `always` or `dynamic` only after the LLM provider is
  configured and cost/latency is acceptable.

Unsupported by this local-first runbook:

- Public internet exposure of `elf-api`, `elf-admin`, or `elf-mcp`.
- Hosted managed-memory parity claims.
- Private-corpus or provider-backed quality claims from the checked-in local config.
- Treating local SDK-style exports from other products as OpenMemory UI/export or
  hosted-platform evidence.
- Using `add_event` with `config/local/elf.docker.toml`; the extractor block is a
  placeholder and the deterministic local loop intentionally avoids it.

## Run E2E Harness (Optional)

The context misranking harness creates and drops a dedicated database and Qdrant collection. It requires:

- `ELF_PG_DSN` (a base DSN that typically ends with `/postgres`)
- `ELF_QDRANT_GRPC_URL` (Qdrant gRPC base URL)
- `ELF_QDRANT_HTTP_URL` (Qdrant REST base URL)

Example:

```sh
ELF_PG_DSN="postgres://elf_dev:elf_dev_password@127.0.0.1:51888/postgres" \
ELF_QDRANT_GRPC_URL="http://127.0.0.1:51890" \
ELF_QDRANT_HTTP_URL="http://127.0.0.1:51889" \
cargo make test-e2e
```

## Troubleshooting

- Config parse errors:
  - ELF config has no implicit defaults. Fix missing fields in the TOML (the error message will name the missing field).
- API never becomes healthy:
  - Check the API log and confirm Postgres and Qdrant are reachable.
- Qdrant collection errors:
  - Confirm the REST URL is correct, and rerun `./qdrant/init.sh`.
