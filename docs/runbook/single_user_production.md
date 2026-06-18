---
type: Runbook
title: "Single-User Production Runbook"
description: "Runbook for operating one local ELF instance with Docker Compose managed services."
resource: docs/runbook/single_user_production.md
status: active
authority: procedural
owner: runbook
last_verified: 2026-06-18
tags:
  - docs
  - runbook
---
# Single-User Production Runbook

Goal: Operate one local ELF instance with Docker Compose managed Postgres and Qdrant,
plus ELF API, worker, and optional MCP processes.
Read this when: You are running ELF as a personal production memory service or proving backup,
restore, migration, and Qdrant rebuild behavior.
Preconditions: Docker Compose, this repository checkout, a Rust toolchain for building ELF
binaries, and provider credentials for production embeddings/rerank/extraction.
Depends on: `docker-compose.yml`, `elf.example.toml`, `docs/spec/system_elf_memory_service_v2.md`,
`docs/runbook/getting_started.md`, and `docs/runbook/integration-testing.md`.
Verification: Health succeeds, a note can be ingested and found, Postgres backup restores notes,
Qdrant search state can be rebuilt from Postgres, and the clean-volume proof path below can run
without host-global service installs.

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

Create `.env` for Docker Compose storage settings only. Docker Compose loads it automatically; ELF
itself does not read provider credentials or required config fields from environment variables.

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

Do not put provider credentials, bearer tokens, or static-key secrets in the Compose `.env` file.
Production provider settings belong in the untracked ELF config file, or in a local secret-rendering
step that writes that untracked config before startup. ELF fails closed when provider keys are empty,
required provider fields are absent, the embedding dimension does not match the Qdrant vector
dimension, or the config path is missing.

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

Stop storage without deleting data:

```sh
docker compose -f docker-compose.yml stop postgres qdrant
```

Start it again:

```sh
docker compose -f docker-compose.yml up -d postgres qdrant
```

Remove stopped containers while keeping volumes:

```sh
docker compose -f docker-compose.yml down
```

Delete all Compose-managed storage only when you have a verified backup or are running the
clean-volume proof below:

```sh
docker compose -f docker-compose.yml down -v
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

Stop ELF services by sending Ctrl-C in each service terminal. If you started them in the background,
stop those exact processes before backup, restore, upgrade, or rollback:

```sh
pkill -f "target/debug/elf-api -c elf.production.toml" || true
pkill -f "target/debug/elf-worker -c elf.production.toml" || true
pkill -f "target/debug/elf-mcp -c elf.production.toml" || true
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

## 5. Restart, Upgrade, And Roll Back

For a config-only restart:

```sh
pkill -f "target/debug/elf-api -c elf.production.toml" || true
pkill -f "target/debug/elf-worker -c elf.production.toml" || true
pkill -f "target/debug/elf-mcp -c elf.production.toml" || true
```

Then start the worker and API again in separate terminals:

```sh
target/debug/elf-worker -c elf.production.toml
```

```sh
target/debug/elf-api -c elf.production.toml
```

For an ELF binary upgrade:

```sh
# 1. Run Section 6 and keep the backup path.
# 2. Stop ELF service processes.
pkill -f "target/debug/elf-api -c elf.production.toml" || true
pkill -f "target/debug/elf-worker -c elf.production.toml" || true
pkill -f "target/debug/elf-mcp -c elf.production.toml" || true

# 3. Move the checkout to the desired release or commit, then rebuild.
cargo build -p elf-api -p elf-worker -p elf-mcp

# 4. Start worker in one terminal.
target/debug/elf-worker -c elf.production.toml
```

```sh
# 5. Start API in another terminal, then run Section 4 health and migration checks.
target/debug/elf-api -c elf.production.toml
```

For rollback, restore the pre-upgrade backup and rebuild Qdrant:

```sh
# 1. Stop ELF service processes.
pkill -f "target/debug/elf-api -c elf.production.toml" || true
pkill -f "target/debug/elf-worker -c elf.production.toml" || true
pkill -f "target/debug/elf-mcp -c elf.production.toml" || true

# 2. Move the checkout and elf.production.toml back to the previous known-good version.
# 3. Run Section 7 restore.
# 4. Run Section 8 Qdrant rebuild.
# 5. Start the previous known-good worker and API, then run Section 4 health checks.
```

## 6. Back Up Postgres

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

## 7. Restore Postgres

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

## 8. Rebuild Qdrant From Postgres

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

## 9. Smoke And Restore Proof

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

### Clean-Volume Proof Path

Run this from the repository root when you need a local proof that backup, clean-volume restore,
Qdrant rebuild, and search recovery work without host-global service installs. It uses the
checked-in deterministic local providers, a temporary config under `tmp/`, ports `51988-51993`,
and isolated Docker volume names.

```sh
bash <<'EOF'
set -euo pipefail

PROOF_DIR="tmp/single-user-restore-proof"
PROOF_CONFIG="${PROOF_DIR}/elf.restore-proof.toml"
mkdir -p "${PROOF_DIR}/backups"
cp config/local/elf.docker.toml "${PROOF_CONFIG}"
perl -0pi -e 's/127\.0\.0\.1:51888/127.0.0.1:51988/g; s/127\.0\.0\.1:51889/127.0.0.1:51989/g; s/127\.0\.0\.1:51890/127.0.0.1:51990/g; s/127\.0\.0\.1:51891/127.0.0.1:51991/g; s/127\.0\.0\.1:51892/127.0.0.1:51992/g; s/127\.0\.0\.1:51893/127.0.0.1:51993/g; s/elf_local_notes/elf_restore_proof_notes/g; s/elf_local_doc_chunks/elf_restore_proof_doc_chunks/g' "${PROOF_CONFIG}"

export ELF_COMPOSE_PROJECT=elf-restore-proof
export ELF_POSTGRES_DB=elf_local
export ELF_POSTGRES_USER=elf_dev
export ELF_POSTGRES_PASSWORD=elf_dev_password
export ELF_POSTGRES_PORT=51988
export ELF_POSTGRES_VOLUME=elf-restore-proof-postgres-data
export ELF_QDRANT_REST_PORT=51989
export ELF_QDRANT_GRPC_PORT=51990
export ELF_QDRANT_VOLUME=elf-restore-proof-qdrant-data

API_PID=""
WORKER_PID=""
cleanup() {
  for pid in ${API_PID:-} ${WORKER_PID:-}; do
    if [ -n "${pid}" ]; then
      kill "${pid}" 2>/dev/null || true
      wait "${pid}" 2>/dev/null || true
    fi
  done
  docker compose -f docker-compose.yml down -v --remove-orphans >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker compose -f docker-compose.yml down -v --remove-orphans
docker compose -f docker-compose.yml config >/dev/null
docker compose -f docker-compose.yml up -d postgres qdrant
for _ in $(seq 1 60); do
  docker compose -f docker-compose.yml exec -T postgres \
    pg_isready -U "${ELF_POSTGRES_USER}" -d "${ELF_POSTGRES_DB}" >/dev/null 2>&1 && break
  sleep 1
done
docker compose -f docker-compose.yml exec -T postgres \
  pg_isready -U "${ELF_POSTGRES_USER}" -d "${ELF_POSTGRES_DB}"
for _ in $(seq 1 60); do
  curl -fsS "http://127.0.0.1:${ELF_QDRANT_REST_PORT}/collections" >/dev/null && break
  sleep 1
done
curl -fsS "http://127.0.0.1:${ELF_QDRANT_REST_PORT}/collections" >/dev/null

cargo build -p elf-api -p elf-worker

target/debug/elf-worker -c "${PROOF_CONFIG}" > "${PROOF_DIR}/worker-before.log" 2>&1 &
WORKER_PID="$!"
target/debug/elf-api -c "${PROOF_CONFIG}" > "${PROOF_DIR}/api-before.log" 2>&1 &
API_PID="$!"

for _ in $(seq 1 60); do
  curl -fsS http://127.0.0.1:51992/health >/dev/null && break
  sleep 1
done
curl -fsS http://127.0.0.1:51992/health | tee "${PROOF_DIR}/health-before.json"

curl -fsS -X POST http://127.0.0.1:51992/v2/notes/ingest \
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
        "text": "The single-user production restore proof note is stored in Postgres and searchable after Qdrant rebuild.",
        "importance": 0.8,
        "confidence": 0.95,
        "ttl_days": 14,
        "source_ref": {"schema": "single_user_runbook/v1", "ref": {"step": "clean_volume_restore_proof"}}
      }
    ]
  }' | tee "${PROOF_DIR}/add-note.json"

for _ in $(seq 1 60); do
  OPEN_OUTBOX="$(docker compose -f docker-compose.yml exec -T postgres \
    psql -U "${ELF_POSTGRES_USER}" -d "${ELF_POSTGRES_DB}" -At \
    -c "SELECT COUNT(*) FROM indexing_outbox WHERE status <> 'DONE';")"
  [ "${OPEN_OUTBOX}" = "0" ] && break
  sleep 1
done
test "${OPEN_OUTBOX}" = "0"

curl -fsS -X POST http://127.0.0.1:51992/v2/searches \
  -H 'content-type: application/json' \
  -H 'X-ELF-Tenant-Id: local-tenant' \
  -H 'X-ELF-Project-Id: local-project' \
  -H 'X-ELF-Agent-Id: local-agent' \
  -H 'X-ELF-Read-Profile: private_only' \
  -d '{
    "mode": "quick_find",
    "query": "Where is the single-user production restore proof note stored?",
    "top_k": 5,
    "candidate_k": 20,
    "payload_level": "l0"
  }' | tee "${PROOF_DIR}/search-before.json"
grep -F "single-user production restore proof note" "${PROOF_DIR}/search-before.json"

BACKUP="${PROOF_DIR}/backups/elf-proof.dump"
docker compose -f docker-compose.yml exec -T postgres \
  pg_dump -U "${ELF_POSTGRES_USER}" -d "${ELF_POSTGRES_DB}" -Fc > "${BACKUP}"
test -s "${BACKUP}"

kill "${API_PID}" "${WORKER_PID}" 2>/dev/null || true
wait "${API_PID}" "${WORKER_PID}" 2>/dev/null || true
API_PID=""
WORKER_PID=""

docker compose -f docker-compose.yml down -v --remove-orphans
docker compose -f docker-compose.yml up -d postgres qdrant
for _ in $(seq 1 60); do
  docker compose -f docker-compose.yml exec -T postgres \
    pg_isready -U "${ELF_POSTGRES_USER}" -d "${ELF_POSTGRES_DB}" >/dev/null 2>&1 && break
  sleep 1
done
docker compose -f docker-compose.yml exec -T postgres \
  pg_isready -U "${ELF_POSTGRES_USER}" -d "${ELF_POSTGRES_DB}"
for _ in $(seq 1 60); do
  curl -fsS "http://127.0.0.1:${ELF_QDRANT_REST_PORT}/collections" >/dev/null && break
  sleep 1
done

docker compose -f docker-compose.yml exec -T postgres \
  dropdb -U "${ELF_POSTGRES_USER}" --force --if-exists "${ELF_POSTGRES_DB}"
docker compose -f docker-compose.yml exec -T postgres \
  createdb -U "${ELF_POSTGRES_USER}" "${ELF_POSTGRES_DB}"
docker compose -f docker-compose.yml exec -T postgres \
  pg_restore -U "${ELF_POSTGRES_USER}" -d "${ELF_POSTGRES_DB}" \
  --no-owner --role="${ELF_POSTGRES_USER}" < "${BACKUP}"

RESTORED_NOTES="$(docker compose -f docker-compose.yml exec -T postgres \
  psql -U "${ELF_POSTGRES_USER}" -d "${ELF_POSTGRES_DB}" -At \
  -c "SELECT COUNT(*) FROM memory_notes WHERE key = 'single_user_restore_probe';")"
test "${RESTORED_NOTES}" = "1"

target/debug/elf-api -c "${PROOF_CONFIG}" > "${PROOF_DIR}/api-after.log" 2>&1 &
API_PID="$!"
for _ in $(seq 1 60); do
  curl -fsS http://127.0.0.1:51992/health >/dev/null && break
  sleep 1
done

curl -fsS -X POST http://127.0.0.1:51991/v2/admin/qdrant/rebuild \
  | tee "${PROOF_DIR}/qdrant-rebuild.json"
grep -F '"missing_vector_count":0' "${PROOF_DIR}/qdrant-rebuild.json"
grep -F '"error_count":0' "${PROOF_DIR}/qdrant-rebuild.json"

curl -fsS -X POST http://127.0.0.1:51992/v2/searches \
  -H 'content-type: application/json' \
  -H 'X-ELF-Tenant-Id: local-tenant' \
  -H 'X-ELF-Project-Id: local-project' \
  -H 'X-ELF-Agent-Id: local-agent' \
  -H 'X-ELF-Read-Profile: private_only' \
  -d '{
    "mode": "quick_find",
    "query": "Where is the single-user production restore proof note stored?",
    "top_k": 5,
    "candidate_k": 20,
    "payload_level": "l0"
  }' | tee "${PROOF_DIR}/search-after.json"
grep -F "single-user production restore proof note" "${PROOF_DIR}/search-after.json"

printf 'Single-user restore proof passed. Evidence files remain under %s.\n' "${PROOF_DIR}"
EOF
```

The proof fails closed on missing Docker services, occupied ports, failed service health, undrained
indexing outbox rows, an empty backup, missing restored source rows, non-zero Qdrant rebuild errors,
or a search response that does not contain the restored note.

### Recorded Local Proof - June 9, 2026

The clean-volume proof path above was executed locally against this worktree after aligning
`docker-compose.yml` with the PostgreSQL 18 volume layout. It used the checked-in local deterministic
providers, isolated Compose volumes, and ports `51988-51993`.

Recorded evidence:

- Compose storage started cleanly with Postgres accepting connections.
- `cargo build -p elf-api -p elf-worker` completed.
- `POST /v2/notes/ingest` returned `op = "ADD"` and `policy_decision = "remember"` for
  `key = "single_user_restore_probe"`.
- Search before backup returned the note summary:
  "The single-user production restore proof note is stored in Postgres and searchable after Qdrant
  rebuild."
- The custom-format Postgres backup was non-empty (`88K` in the local proof run).
- The proof destroyed and recreated the isolated Compose volumes, restored Postgres with
  `pg_restore`, and verified one restored source row for `single_user_restore_probe`.
- `POST /v2/admin/qdrant/rebuild` returned
  `{"error_count":0,"missing_vector_count":0,"rebuilt_count":1}`.
- Search after restore and Qdrant rebuild returned the same restored note.
- Cleanup removed the isolated proof containers and volumes.

## 10. Local CLI Wrappers

The `elf` CLI is a thin local wrapper over the same HTTP contracts used above. It does not read or
write storage directly, bypass auth, or change scope/read-profile rules. Build it with the service
binaries:

```sh
cargo build -p elf --bin elf
```

By default the CLI targets the runbook loopback ports and smoke context:

- `ELF_API_URL` or `--api-url`: default `http://127.0.0.1:51892`.
- `ELF_ADMIN_URL` or `--admin-url`: default `http://127.0.0.1:51891`.
- `ELF_TENANT_ID`, `ELF_PROJECT_ID`, and `ELF_AGENT_ID`: default `local-tenant`,
  `local-project`, and `local-agent`.
- `ELF_READ_PROFILE` or `--read-profile`: default `private_only`.
- `ELF_USER_TOKEN` or `--token`: bearer token for public endpoints when static-key auth is enabled.
- `ELF_ADMIN_TOKEN` or `--admin-token`: admin bearer token for admin endpoints.

Check API health and get machine-readable status:

```sh
target/debug/elf status --pretty
```

Add a deterministic note through `POST /v2/notes/ingest`. `--source-id` is copied into
`source_ref.ref.source_id` and echoed in the CLI output for debugging:

```sh
target/debug/elf add-note \
  --key single_user_restore_probe_cli \
  --source-id single-user-runbook:restore-probe-cli \
  --text "The single-user production CLI smoke note is stored through the HTTP add-note contract." \
  --importance 0.8 \
  --confidence 0.95 \
  --ttl-days 14 \
  --pretty
```

Search through `POST /v2/searches`. The JSON output includes `trace_id`, `search_id`, and note ids:

```sh
target/debug/elf search \
  --query "Where is the single-user production CLI smoke note stored?" \
  --top-k 5 \
  --candidate-k 20 \
  --payload-level l0 \
  --pretty
```

Use admin diagnostics when you need source refs, trace bundles, provenance, or a Qdrant rebuild
readback. These commands require an admin token when `security.auth_mode = "static_keys"`:

```sh
target/debug/elf diagnostics raw-search \
  --query "Where is the single-user production CLI smoke note stored?" \
  --payload-level l2 \
  --pretty

target/debug/elf diagnostics recent-traces --limit 10 --pretty
target/debug/elf diagnostics trace-bundle --trace-id TRACE_ID --mode bounded --pretty
target/debug/elf diagnostics note-provenance --note-id NOTE_ID --pretty
target/debug/elf diagnostics qdrant-rebuild --pretty
```

For batch backfill and benchmark reports, use the wrappers documented in
`docs/runbook/benchmarking/live_baseline_benchmark.md`. Those wrappers delegate to the checked-in
`cargo make` tasks and keep benchmark artifacts under `tmp/live-baseline/`.

## 11. Failure And Secret Rules

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

## Related Runbooks

- Local bootstrap: `docs/runbook/getting_started.md`
- Integration testing: `docs/runbook/integration-testing.md`
- System contract: `docs/spec/system_elf_memory_service_v2.md`
