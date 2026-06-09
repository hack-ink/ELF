# Single-User Production Runbook

Goal: Operate one local ELF instance with Docker Compose managed Postgres and Qdrant,
plus ELF API, worker, and optional MCP processes.
Read this when: You are running ELF as a personal production memory service or proving backup,
restore, migration, and Qdrant rebuild behavior.
Preconditions: Docker Compose, this repository checkout, a Rust toolchain for building ELF
binaries, and provider credentials for production embeddings/rerank/extraction.
Depends on: `docker-compose.yml`, `elf.example.toml`, `docs/spec/system_elf_memory_service_v2.md`,
`docs/guide/getting_started.md`, and `docs/guide/integration-testing.md`.
Verification: Health succeeds, a note can be ingested and found, Postgres backup restores notes,
and Qdrant search state can be rebuilt from Postgres.

## Operating Boundary

This runbook is the minimum single-user production path. It does not describe hosted,
cloud-managed, or public internet deployment.

Postgres is the only source of truth for notes, chunks, embeddings, audit history, and outbox
state. Qdrant is derived state. Back up Postgres, not Qdrant. If Qdrant is lost, recreate its
collections and run the admin rebuild from Postgres.

The checked-in `docker-compose.yml` owns only the stateful services:

- `postgres`: Postgres with pgvector.
- `qdrant`: Qdrant REST and gRPC.

`elf-api`, `elf-worker`, and `elf-mcp` run as local ELF binaries from the checked-out release.
Keep their binds on loopback. The API refuses `http_bind` outside loopback when
`security.bind_localhost_only = true`, refuses `security.auth_mode = "off"` on non-loopback HTTP
binds, and always requires `admin_bind` to be loopback. The MCP server also refuses non-loopback
binds when auth is off.

## 1. Create Local Secrets

Create `.env` for Docker Compose only. Docker Compose loads it automatically; ELF itself does not
read provider credentials or required config fields from environment variables.

```sh
cat > .env <<'EOF'
ELF_COMPOSE_PROJECT=elf-prod
ELF_POSTGRES_DB=elf_prod
ELF_POSTGRES_USER=elf_prod
ELF_POSTGRES_PASSWORD=replace-with-a-long-random-password
ELF_POSTGRES_PORT=51888
ELF_POSTGRES_VOLUME=elf-prod-postgres-data
ELF_QDRANT_REST_PORT=51889
ELF_QDRANT_GRPC_PORT=51890
ELF_QDRANT_VOLUME=elf-prod-qdrant-data
ELF_QDRANT_COLLECTION=mem_notes_v2
ELF_QDRANT_DOCS_COLLECTION=doc_chunks_v1
ELF_QDRANT_VECTOR_DIM=4096
EOF
chmod 600 .env
```

For shell commands below, load the same variables into your shell:

```sh
set -a
. ./.env
set +a
```

Create an untracked production config:

```sh
cp elf.example.toml elf.production.toml
chmod 600 elf.production.toml
```

Edit `elf.production.toml`:

- Set `storage.postgres.dsn` to
  `postgres://elf_prod:<ELF_POSTGRES_PASSWORD>@127.0.0.1:51888/elf_prod`, using the real password.
- Set `storage.qdrant.url` to `http://127.0.0.1:51890`.
- Set `storage.qdrant.collection`, `storage.qdrant.docs_collection`, and
  `storage.qdrant.vector_dim` to match `.env`.
- Fill every `[providers.*]` block with real provider endpoints, models, dimensions, and keys.
- Keep `providers.embedding.dimensions` equal to `storage.qdrant.vector_dim`.
- Keep `chunking.enabled = true` and set `chunking.tokenizer_repo` to a non-empty tokenizer.
- Prefer `security.auth_mode = "static_keys"` with non-empty `security.auth_keys`.
- If you run `elf-mcp`, keep `[mcp]` present and ensure exactly one static key matches its
  tenant, project, agent, and read profile.

Do not commit `.env`, `elf.production.toml`, backups, provider keys, bearer tokens, or database
dumps. `.env*`, root ELF config files, and `backups/` are ignored for this reason.

## 2. Start Postgres And Qdrant

Validate the Compose file and start storage:

```sh
docker compose -f docker-compose.yml config >/dev/null
docker compose -f docker-compose.yml up -d postgres qdrant
docker compose -f docker-compose.yml ps
```

Check storage health:

```sh
docker compose -f docker-compose.yml exec -T postgres \
  pg_isready -U "${ELF_POSTGRES_USER}" -d "${ELF_POSTGRES_DB}"

curl -fsS "http://127.0.0.1:${ELF_QDRANT_REST_PORT}/collections" >/dev/null
```

## 3. Build And Start ELF Services

Build once, then run the binaries directly to avoid multiple `cargo run` processes contending for
Cargo locks:

```sh
cargo build -p elf-api -p elf-worker -p elf-mcp
```

Start the worker in one terminal:

```sh
target/debug/elf-worker -c elf.production.toml
```

Start the API in a second terminal:

```sh
target/debug/elf-api -c elf.production.toml
```

Optional: start MCP in a third terminal when a client needs the MCP adapter:

```sh
target/debug/elf-mcp -c elf.production.toml
```

On startup, `elf-api` and `elf-worker` initialize the Postgres schema and ensure the Qdrant
collections and docs payload indexes exist. Startup fails closed if the config file is missing,
required config is absent, `security.reject_non_english` is false, vector dimensions mismatch, or
loopback/auth rules are violated.

## 4. Health And Migration Checks

Check API health:

```sh
curl -fsS http://127.0.0.1:51892/health
```

Check that schema initialization or migration has reached the configured database:

```sh
docker compose -f docker-compose.yml exec -T postgres \
  psql -U "${ELF_POSTGRES_USER}" -d "${ELF_POSTGRES_DB}" -v ON_ERROR_STOP=1 \
  -c "SELECT COUNT(*) AS active_notes FROM memory_notes WHERE status = 'active';"
```

Before upgrading ELF binaries or changing config, take a Postgres backup. There is no reverse
migration command in the minimum runbook; rollback means stopping ELF, restoring the previous
Postgres backup, starting the previous known-good binary/config, and rebuilding Qdrant.

## 5. Back Up Postgres

Stop or pause writers first. For this single-user runbook, that means stop `elf-api`, `elf-worker`,
and `elf-mcp` with Ctrl-C in their terminals. Leave the `postgres` container running.

Create a custom-format Postgres backup:

```sh
mkdir -p backups/postgres
BACKUP="backups/postgres/elf-$(date -u +%Y%m%dT%H%M%SZ).dump"

docker compose -f docker-compose.yml exec -T postgres \
  pg_dump -U "${ELF_POSTGRES_USER}" -d "${ELF_POSTGRES_DB}" -Fc > "${BACKUP}"

chmod 600 "${BACKUP}"
printf 'Wrote %s\n' "${BACKUP}"
```

Copy the backup to your normal encrypted backup location. Do not commit it.

## 6. Restore Postgres

Use this path for a fresh machine restore or rollback. Stop `elf-api`, `elf-worker`, and `elf-mcp`
before restoring. Start only storage:

```sh
docker compose -f docker-compose.yml up -d postgres qdrant
```

Restore the selected backup into the configured database:

```sh
RESTORE="backups/postgres/elf-YYYYMMDDTHHMMSSZ.dump"

docker compose -f docker-compose.yml exec -T postgres \
  dropdb -U "${ELF_POSTGRES_USER}" --force --if-exists "${ELF_POSTGRES_DB}"

docker compose -f docker-compose.yml exec -T postgres \
  createdb -U "${ELF_POSTGRES_USER}" "${ELF_POSTGRES_DB}"

docker compose -f docker-compose.yml exec -T postgres \
  pg_restore -U "${ELF_POSTGRES_USER}" -d "${ELF_POSTGRES_DB}" \
  --no-owner --role="${ELF_POSTGRES_USER}" < "${RESTORE}"
```

Verify the restored source-of-truth rows:

```sh
docker compose -f docker-compose.yml exec -T postgres \
  psql -U "${ELF_POSTGRES_USER}" -d "${ELF_POSTGRES_DB}" -v ON_ERROR_STOP=1 \
  -c "SELECT COUNT(*) AS notes FROM memory_notes;"
```

## 7. Rebuild Qdrant From Postgres

Qdrant is rebuildable. If the Qdrant volume or memory-note collection is missing, stale, or
restored from the wrong point in time, discard the memory-note collection and rebuild it from
Postgres.

Delete the derived memory-note collection. A missing collection is acceptable:

```sh
QDRANT_REST="http://127.0.0.1:${ELF_QDRANT_REST_PORT}"

curl -fsS -X DELETE "${QDRANT_REST}/collections/${ELF_QDRANT_COLLECTION}?wait=true" || true
```

Start or restart `elf-api` after deleting collections so startup recreates them:

```sh
target/debug/elf-api -c elf.production.toml
```

Then call the admin rebuild endpoint from another terminal. If `security.auth_mode = "static_keys"`,
use an admin or super-admin token:

```sh
curl -fsS -X POST http://127.0.0.1:51891/v2/admin/qdrant/rebuild \
  -H "Authorization: Bearer ${ELF_ADMIN_TOKEN}"
```

Expected result:

```json
{
  "rebuilt_count": 1,
  "missing_vector_count": 0,
  "error_count": 0
}
```

`rebuilt_count` depends on how many active chunks exist. `missing_vector_count` and `error_count`
must be `0` for a clean production restore. The rebuild uses persisted Postgres vectors and must not
call the embedding provider.

This endpoint rebuilds memory-note chunks. Do not treat it as a Doc Extension rebuild procedure for
`storage.qdrant.docs_collection`.

## 8. Smoke And Restore Proof

With `elf-worker` and `elf-api` running, ingest one deterministic note. If auth is off, omit the
`Authorization` header. If static-key auth is on, use a token whose configured context matches the
tenant, project, agent, and read profile used by the smoke commands.

```sh
curl -fsS -X POST http://127.0.0.1:51892/v2/notes/ingest \
  -H "Authorization: Bearer ${ELF_USER_TOKEN}" \
  -H 'content-type: application/json' \
  -H 'X-ELF-Tenant-Id: local-tenant' \
  -H 'X-ELF-Project-Id: local-project' \
  -H 'X-ELF-Agent-Id: local-agent' \
  -d '{
    "scope": "agent_private",
    "notes": [
      {
        "type": "fact",
        "key": "single_user_restore_probe",
        "text": "The single-user production restore probe is stored in Postgres and searchable after Qdrant rebuild.",
        "importance": 0.8,
        "confidence": 0.95,
        "ttl_days": 14,
        "source_ref": {"schema": "single_user_runbook/v1", "ref": {"step": "restore_probe"}}
      }
    ]
  }'
```

Wait a few seconds for the worker, then search:

```sh
curl -fsS -X POST http://127.0.0.1:51892/v2/searches \
  -H "Authorization: Bearer ${ELF_USER_TOKEN}" \
  -H 'content-type: application/json' \
  -H 'X-ELF-Tenant-Id: local-tenant' \
  -H 'X-ELF-Project-Id: local-project' \
  -H 'X-ELF-Agent-Id: local-agent' \
  -H 'X-ELF-Read-Profile: private_only' \
  -d '{
    "mode": "quick_find",
    "query": "Where is the single-user production restore probe stored?",
    "top_k": 5,
    "candidate_k": 20,
    "payload_level": "l0"
  }'
```

To prove restore and rebuild:

1. Run the backup step.
2. Stop `elf-api`, `elf-worker`, and `elf-mcp`.
3. Restore the backup into Postgres.
4. Delete the Qdrant memory-note collection.
5. Start `elf-api`, call `/v2/admin/qdrant/rebuild`, then start `elf-worker`.
6. Re-run the search command and confirm the restored note appears.

## 9. Failure And Secret Rules

- Missing or invalid config fails startup.
- `security.reject_non_english = false` fails config validation.
- Non-English API inputs fail with HTTP 422.
- API binds outside loopback fail unless authenticated static-key mode is configured; admin bind is
  loopback-only.
- `add_note` is deterministic and does not call an LLM. `add_event` requires the configured LLM
  extractor and evidence-bound quotes.
- Secret-like note text is rejected by the write gate.
- Qdrant can be stale, empty, or deleted; Postgres remains authoritative.
- Never commit `.env`, `elf.production.toml`, backups, dumps, API keys, bearer tokens, or provider
  credentials.

## Related Guides

- Local bootstrap: `docs/guide/getting_started.md`
- Integration testing: `docs/guide/integration-testing.md`
- System contract: `docs/spec/system_elf_memory_service_v2.md`
