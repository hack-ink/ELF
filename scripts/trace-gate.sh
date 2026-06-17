#!/usr/bin/env bash
set -euo pipefail

DSN="${TRACE_GATE_PG_DSN:-${PG_DSN:-postgres://postgres:postgres@127.0.0.1:5432/elf}}"
VECTOR_DIM="${TRACE_GATE_VECTOR_DIM:-4}"
SCHEMA_PATH="tmp/trace_gate.schema.sql"
REPORT_PATH="${TRACE_GATE_REPORT_PATH:-tmp/trace_gate.report.json}"

mkdir -p tmp

TRACE_GATE_VECTOR_DIM="${VECTOR_DIM}" python3 - <<'PY' > "${SCHEMA_PATH}"
import os
from pathlib import Path

vector_dim = int(os.environ["TRACE_GATE_VECTOR_DIM"])
root = Path(".")
sql_dir = root / "sql"

out = []
for raw_line in (sql_dir / "init.sql").read_text(encoding="utf-8").splitlines():
	line = raw_line.strip()
	if line.startswith(r"\ir "):
		rel = line[len(r"\ir ") :].strip()
		out.append((sql_dir / rel).read_text(encoding="utf-8"))
	else:
		out.append(raw_line)

expanded = "\n".join(out) + "\n"
print(expanded.replace("<VECTOR_DIM>", str(vector_dim)), end="")
PY

psql "${DSN}" -v ON_ERROR_STOP=1 -f "${SCHEMA_PATH}"
psql "${DSN}" -v ON_ERROR_STOP=1 -f .github/fixtures/trace_gate/fixture.sql
cargo run -p elf-eval --bin trace_regression_gate -- \
	--config .github/fixtures/trace_gate/config.toml \
	--gate .github/fixtures/trace_gate/gate.json \
	--out "${REPORT_PATH}"
