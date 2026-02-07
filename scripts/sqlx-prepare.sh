#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -f "${ROOT_DIR}/.env" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "${ROOT_DIR}/.env"
  set +a
fi

: "${ELF_PG_DSN:?Set ELF_PG_DSN to a Postgres DSN (usually .../postgres).}"

if ! command -v psql >/dev/null 2>&1; then
  echo "Missing psql." >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "Missing cargo." >&2
  exit 1
fi

if ! command -v perl >/dev/null 2>&1; then
  echo "Missing perl (required for template substitution)." >&2
  exit 1
fi

DB_NAME="${ELF_SQLX_PREPARE_DB:-elf_sqlx_prepare}"
VECTOR_DIM="${ELF_SQLX_VECTOR_DIM:-4096}"

PG_DSN_BASE="${ELF_PG_DSN%/*}"
DATABASE_URL="${PG_DSN_BASE}/${DB_NAME}"

TMP_DIR="${ROOT_DIR}/tmp/sqlx.prepare.sql"
TMP_SQL="${TMP_DIR}/init.sql"

cleanup() {
  set +e
  psql "${ELF_PG_DSN}" -tAc \
    "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '${DB_NAME}' AND pid <> pg_backend_pid();" \
    >/dev/null 2>&1 || true
  psql "${ELF_PG_DSN}" -v ON_ERROR_STOP=1 -c "DROP DATABASE IF EXISTS ${DB_NAME};" >/dev/null 2>&1 || true
}

trap cleanup EXIT

echo "Recreating database ${DB_NAME}."
psql "${ELF_PG_DSN}" -v ON_ERROR_STOP=1 -c "DROP DATABASE IF EXISTS ${DB_NAME};" >/dev/null
psql "${ELF_PG_DSN}" -v ON_ERROR_STOP=1 -c "CREATE DATABASE ${DB_NAME};" >/dev/null

echo "Applying schema to ${DB_NAME} (VECTOR_DIM=${VECTOR_DIM})."
rm -rf "${TMP_DIR}"
mkdir -p "${TMP_DIR}/tables"

perl -pe "s/<VECTOR_DIM>/${VECTOR_DIM}/g" "${ROOT_DIR}/sql/init.sql" >"${TMP_DIR}/init.sql"
perl -pe "s/<VECTOR_DIM>/${VECTOR_DIM}/g" "${ROOT_DIR}/sql/00_extensions.sql" >"${TMP_DIR}/00_extensions.sql"

for path in "${ROOT_DIR}"/sql/tables/*.sql; do
  name="$(basename "${path}")"
  perl -pe "s/<VECTOR_DIM>/${VECTOR_DIM}/g" "${path}" >"${TMP_DIR}/tables/${name}"
done

psql "${DATABASE_URL}" -v ON_ERROR_STOP=1 -f "${TMP_SQL}" >/dev/null

echo "Generating SQLx offline metadata (.sqlx/)."
(cd "${ROOT_DIR}" && DATABASE_URL="${DATABASE_URL}" cargo sqlx prepare --workspace -- --all-targets --all-features)
