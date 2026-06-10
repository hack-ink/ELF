#!/usr/bin/env python3
"""Cost-bounded GraphRAG Docker smoke for real-world external adapters."""

from __future__ import annotations

import csv
import json
import os
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent
REPORT_DIR = Path(
    os.environ.get(
        "ELF_GRAPHRAG_SMOKE_REPORT_DIR",
        ROOT_DIR / "tmp" / "real-world-memory" / "graphrag-smoke",
    )
)
WORK_DIR = Path(os.environ.get("ELF_GRAPHRAG_SMOKE_WORK_DIR", REPORT_DIR / "work"))
OUT = Path(os.environ.get("ELF_GRAPHRAG_SMOKE_OUT", REPORT_DIR / "graphrag-smoke.json"))
MANIFEST_OUT = Path(
    os.environ.get(
        "ELF_GRAPHRAG_SMOKE_MANIFEST_OUT",
        REPORT_DIR / "memory_projects_manifest.graphrag-smoke.json",
    )
)
SUMMARY_OUT = Path(os.environ.get("ELF_GRAPHRAG_SMOKE_SUMMARY_OUT", REPORT_DIR / "summary.json"))
FIXTURE_DIR = REPORT_DIR / "graphrag-fixtures"
OUTPUT_CAPTURE_DIR = REPORT_DIR / "graphrag-output"
LOG_DIR = REPORT_DIR / "logs"

RUN_ID = os.environ.get(
    "ELF_GRAPHRAG_SMOKE_RUN_ID",
    f"graphrag-docker-smoke-{datetime.now(timezone.utc).strftime('%Y%m%d%H%M%S')}",
)
RUN_LIVE = os.environ.get("ELF_GRAPHRAG_SMOKE_RUN", "0") == "1"
ALLOW_HOST = os.environ.get("ELF_GRAPHRAG_SMOKE_ALLOW_HOST", "0") == "1"
INSTALL_GRAPHRAG = os.environ.get("ELF_GRAPHRAG_SMOKE_INSTALL", "1") == "1"
GRAPH_RAG_VERSION = os.environ.get("ELF_GRAPHRAG_VERSION", "3.1.0")
GRAPH_RAG_PACKAGE = os.environ.get("ELF_GRAPHRAG_PACKAGE", f"graphrag=={GRAPH_RAG_VERSION}")
GRAPH_RAG_REF = os.environ.get("ELF_GRAPHRAG_REF", f"pypi:{GRAPH_RAG_PACKAGE}")
CHAT_MODEL = os.environ.get("ELF_GRAPHRAG_CHAT_MODEL", "gpt-4o-mini")
EMBEDDING_MODEL = os.environ.get("ELF_GRAPHRAG_EMBEDDING_MODEL", "text-embedding-3-small")
API_BASE = os.environ.get("ELF_GRAPHRAG_API_BASE", "")
API_KEY = os.environ.get("ELF_GRAPHRAG_API_KEY", os.environ.get("GRAPHRAG_API_KEY", ""))
INDEX_METHOD = os.environ.get("ELF_GRAPHRAG_INDEX_METHOD", "fast")
QUERY_METHOD = os.environ.get("ELF_GRAPHRAG_QUERY_METHOD", "local")
TIMEOUT_SECONDS = int(os.environ.get("ELF_GRAPHRAG_TIMEOUT_SECONDS", "900"))
MAX_DOCS = max(1, min(int(os.environ.get("ELF_GRAPHRAG_MAX_DOCS", "2")), 3))
MAX_INPUT_CHARS = max(400, min(int(os.environ.get("ELF_GRAPHRAG_MAX_INPUT_CHARS", "2400")), 6000))

TABLES = (
    "documents",
    "text_units",
    "communities",
    "community_reports",
    "entities",
    "relationships",
)


@dataclass
class StatusState:
    """Typed status for generated GraphRAG smoke artifacts."""

    setup: str = "blocked"
    run: str = "not_encoded"
    result: str = "blocked"
    overall: str = "blocked"
    evidence_class: str = "research_gate"
    failure_class: str = "graphrag_live_run_disabled"
    failure_reason: str = (
        "GraphRAG indexing is model-call intensive; set ELF_GRAPHRAG_SMOKE_RUN=1 "
        "and provide explicit provider configuration to attempt the live Docker smoke."
    )


@dataclass
class CommandRecord:
    """Captured command result without secret-bearing environment values."""

    label: str
    command: list[str]
    status: str
    elapsed_ms: float
    stdout_artifact: str | None
    stderr_artifact: str | None
    returncode: int | None
    reason: str


def utc_now() -> str:
    """Return an RFC3339 UTC timestamp."""

    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def rel(path: Path) -> str:
    """Return a repository-relative path when possible."""

    try:
        return str(path.resolve().relative_to(ROOT_DIR))
    except ValueError:
        return str(path)


def mkdirs() -> None:
    """Create output directories."""

    for path in (REPORT_DIR, WORK_DIR, FIXTURE_DIR, OUTPUT_CAPTURE_DIR, LOG_DIR):
        path.mkdir(parents=True, exist_ok=True)


def write_json(path: Path, payload: Any) -> None:
    """Write stable, pretty JSON."""

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def dir_size(path: Path) -> int:
    """Return total file size for a directory or file."""

    if not path.exists():
        return 0
    if path.is_file():
        return path.stat().st_size

    return sum(item.stat().st_size for item in path.rglob("*") if item.is_file())


def file_count(path: Path) -> int:
    """Return file count for a directory."""

    if not path.exists():
        return 0

    return sum(1 for item in path.rglob("*") if item.is_file())


def command_available(command: str) -> bool:
    """Return whether a command is on PATH."""

    return shutil.which(command) is not None


def run_command(
    label: str,
    command: list[str],
    cwd: Path,
    timeout: int = TIMEOUT_SECONDS,
    extra_env: dict[str, str] | None = None,
) -> CommandRecord:
    """Run a subprocess and capture stdout/stderr artifacts."""

    cwd.mkdir(parents=True, exist_ok=True)
    stdout_path = LOG_DIR / f"{label}.stdout.log"
    stderr_path = LOG_DIR / f"{label}.stderr.log"
    env = os.environ.copy()

    if extra_env:
        env.update(extra_env)

    started = time.monotonic()
    try:
        proc = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            text=True,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
        elapsed_ms = (time.monotonic() - started) * 1000
        stdout_path.write_text(proc.stdout, encoding="utf-8")
        stderr_path.write_text(proc.stderr, encoding="utf-8")
        status = "pass" if proc.returncode == 0 else "incomplete"
        reason = "Command completed." if proc.returncode == 0 else f"Command exited {proc.returncode}."

        return CommandRecord(
            label=label,
            command=command,
            status=status,
            elapsed_ms=elapsed_ms,
            stdout_artifact=rel(stdout_path),
            stderr_artifact=rel(stderr_path),
            returncode=proc.returncode,
            reason=reason,
        )
    except subprocess.TimeoutExpired as err:
        elapsed_ms = (time.monotonic() - started) * 1000
        stdout_path.write_text(err.stdout or "", encoding="utf-8")
        stderr_path.write_text(err.stderr or "", encoding="utf-8")

        return CommandRecord(
            label=label,
            command=command,
            status="incomplete",
            elapsed_ms=elapsed_ms,
            stdout_artifact=rel(stdout_path),
            stderr_artifact=rel(stderr_path),
            returncode=None,
            reason=f"Command timed out after {timeout} seconds.",
        )


def command_to_json(record: CommandRecord) -> dict[str, Any]:
    """Serialize a command record."""

    return {
        "label": record.label,
        "status": record.status,
        "command": record.command,
        "elapsed_ms": round(record.elapsed_ms, 3),
        "stdout_artifact": record.stdout_artifact,
        "stderr_artifact": record.stderr_artifact,
        "returncode": record.returncode,
        "reason": record.reason,
    }


def generated_corpus() -> list[dict[str, str]]:
    """Return the bounded generated-public corpus."""

    docs = [
        {
            "evidence_id": "graphrag-smoke-nova-observatory",
            "title": "Nova Observatory memo",
            "text": (
                "Evidence ID graphrag-smoke-nova-observatory. Nova Observatory "
                "operates the public Aurora Index review. The Aurora Index links "
                "skyglow measurements to open weather station readings for civic "
                "science audits. The GraphRAG smoke must map this source document "
                "and its text unit back to the Nova Observatory evidence id."
            ),
        },
        {
            "evidence_id": "graphrag-smoke-aurora-index",
            "title": "Aurora Index field note",
            "text": (
                "Evidence ID graphrag-smoke-aurora-index. The Aurora Index uses "
                "Nova Observatory calibration notes when explaining why a public "
                "skyglow reading changed. The GraphRAG smoke must keep the Aurora "
                "Index source document and text unit evidence id recoverable."
            ),
        },
        {
            "evidence_id": "graphrag-smoke-stale-trap",
            "title": "Retired skyglow note",
            "text": (
                "Evidence ID graphrag-smoke-stale-trap. Retired note: Nova "
                "Observatory previously used the obsolete Zenith Ledger. This note "
                "is a distractor and must not be used as the primary answer."
            ),
        },
    ]
    trimmed: list[dict[str, str]] = []
    used_chars = 0

    for doc in docs[:MAX_DOCS]:
        remaining = MAX_INPUT_CHARS - used_chars

        if remaining <= 0:
            break

        text = doc["text"][:remaining].strip()
        used_chars += len(text)
        trimmed.append({**doc, "text": text})

    return trimmed


def write_corpus(project_dir: Path, corpus: list[dict[str, str]]) -> Path:
    """Write GraphRAG plain text input plus a CSV mapping copy."""

    input_dir = project_dir / "input"
    input_dir.mkdir(parents=True, exist_ok=True)
    csv_path = REPORT_DIR / "generated-corpus.csv"

    with csv_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=("evidence_id", "title", "text"))
        writer.writeheader()

        for item in corpus:
            writer.writerow(item)

    for item in corpus:
        file_name = f"{slug(item['evidence_id'])}.txt"
        (input_dir / file_name).write_text(
            f"Title: {item['title']}\nEvidence ID: {item['evidence_id']}\n\n{item['text']}\n",
            encoding="utf-8",
        )

    return csv_path


def write_fixture(corpus: list[dict[str, str]], status: StatusState, mapped_ids: list[str]) -> Path:
    """Write a generated real_world_job fixture for the smoke."""

    fixture_path = FIXTURE_DIR / "knowledge" / "graphrag_tiny_corpus.json"
    expected_ids = [item["evidence_id"] for item in corpus if item["evidence_id"] != "graphrag-smoke-stale-trap"]
    used_ids = [item for item in mapped_ids if item in expected_ids]
    response = {
        "adapter_id": "graphrag_docker_smoke",
        "answer": {
            "content": (
                "Nova Observatory and the Aurora Index are connected by calibration "
                "and public skyglow review evidence."
                if used_ids
                else ""
            ),
            "claims": [
                {
                    "claim_id": "nova_aurora_link",
                    "text": (
                        "Nova Observatory and the Aurora Index are connected by "
                        "calibration and public skyglow review evidence."
                    ),
                    "evidence_ids": used_ids,
                    "confidence": "derived_from_graphrag_table_mapping",
                }
            ]
            if used_ids
            else [],
            "evidence_ids": used_ids,
            "latency_ms": 0.0,
            "cost": {
                "currency": "USD",
                "amount": 0.0,
                "input_tokens": 0,
                "output_tokens": 0,
            },
        },
    }
    fixture: dict[str, Any] = {
        "schema": "elf.real_world_job/v1",
        "job_id": "graphrag-tiny-corpus-001",
        "suite": "knowledge_compilation",
        "title": "Map GraphRAG output tables to generated evidence",
        "corpus": {
            "corpus_id": "graphrag-generated-public-smoke",
            "profile": "generated_public",
            "items": [
                {
                    "evidence_id": item["evidence_id"],
                    "kind": "document",
                    "text": item["text"],
                    "source_ref": {
                        "schema": "source_ref/v1",
                        "resolver": "graphrag_smoke/v1",
                        "ref": {
                            "run_id": RUN_ID,
                            "evidence_id": item["evidence_id"],
                            "title": item["title"],
                        },
                    },
                    "created_at": "2026-06-10T00:00:00Z",
                }
                for item in corpus
            ],
            "adapter_response": response,
        },
        "timeline": [
            {
                "event_id": "graphrag-smoke-corpus-generated",
                "ts": "2026-06-10T00:00:00Z",
                "actor": "system",
                "action": "generated_public_corpus",
                "evidence_ids": expected_ids,
                "summary": "The GraphRAG smoke generated a tiny public corpus for source mapping.",
            }
        ],
        "prompt": {
            "role": "user",
            "content": "What connects Nova Observatory and the Aurora Index in the generated corpus?",
            "job_mode": "compile",
            "constraints": ["cite_evidence", "avoid_stale_facts"],
        },
        "expected_answer": {
            "must_include": [
                {
                    "claim_id": "nova_aurora_link",
                    "text": (
                        "Nova Observatory and the Aurora Index are connected by "
                        "calibration and public skyglow review evidence."
                    ),
                }
            ],
            "must_not_include": ["Zenith Ledger is the current source."],
            "evidence_links": {"nova_aurora_link": expected_ids},
            "answer_type": "direct_answer",
            "accepted_alternates": [],
            "requires_caveat": False,
            "requires_refusal": False,
        },
        "required_evidence": [
            {
                "evidence_id": evidence_id,
                "claim_id": "nova_aurora_link",
                "requirement": "cite",
                "quote": "Aurora Index",
            }
            for evidence_id in expected_ids
        ],
        "negative_traps": [
            {
                "trap_id": "retired-zenith-ledger",
                "type": "stale_fact",
                "evidence_ids": ["graphrag-smoke-stale-trap"],
                "failure_if_used": True,
            }
        ],
        "scoring_rubric": {
            "dimensions": {
                "answer_correctness": {
                    "weight": 0.35,
                    "max_points": 1.0,
                    "criteria": "States the Nova Observatory and Aurora Index relationship.",
                },
                "evidence_grounding": {
                    "weight": 0.35,
                    "max_points": 1.0,
                    "criteria": "Maps output table identifiers to generated evidence ids.",
                },
                "trap_avoidance": {
                    "weight": 0.2,
                    "max_points": 1.0,
                    "criteria": "Does not use the retired Zenith Ledger distractor.",
                },
                "uncertainty": {
                    "weight": 0.1,
                    "max_points": 1.0,
                    "criteria": "Does not claim broad GraphRAG quality from the tiny smoke.",
                },
            },
            "pass_threshold": 0.75,
            "hard_fail_rules": [],
        },
        "allowed_uncertainty": {
            "phrases": ["tiny generated corpus", "smoke only"],
            "fallback": "Report typed failure when GraphRAG output identifiers cannot be mapped.",
        },
        "operator_debug": None,
        "encoding": {},
        "memory_evolution": None,
        "tags": ["external_adapter", "generated_public", "no_live_claim"],
    }

    if status.result in {"blocked", "incomplete"}:
        fixture["encoding"] = {
            "status": status.result,
            "reason": status.failure_reason,
        }

    write_json(fixture_path, fixture)

    return fixture_path


def slug(value: str) -> str:
    """Return a small ASCII slug."""

    out: list[str] = []
    last_dash = False

    for char in value.lower():
        if char.isascii() and char.isalnum():
            out.append(char)
            last_dash = False
        elif not last_dash and out:
            out.append("-")
            last_dash = True

    while out and out[-1] == "-":
        out.pop()

    return "".join(out) or "item"


def init_project(project_dir: Path, command_records: list[CommandRecord]) -> bool:
    """Create a venv, install GraphRAG, and initialize the project."""

    venv_dir = WORK_DIR / ".venv"
    python = venv_dir / "bin" / "python"
    graphrag = venv_dir / "bin" / "graphrag"

    if INSTALL_GRAPHRAG:
        venv_record = run_command("python-venv", [sys.executable, "-m", "venv", str(venv_dir)], WORK_DIR)
        command_records.append(venv_record)
        if venv_record.status != "pass":
            return False

        install_record = run_command(
            "graphrag-install",
            [str(python), "-m", "pip", "install", "--disable-pip-version-check", GRAPH_RAG_PACKAGE],
            WORK_DIR,
        )
        command_records.append(install_record)
        if install_record.status != "pass":
            return False
    elif not graphrag.exists():
        command_records.append(
            CommandRecord(
                label="graphrag-install",
                command=["graphrag"],
                status="incomplete",
                elapsed_ms=0.0,
                stdout_artifact=None,
                stderr_artifact=None,
                returncode=None,
                reason="GraphRAG install was disabled and no venv graphrag executable exists.",
            )
        )

        return False

    init_record = run_command(
        "graphrag-init",
        [
            str(graphrag),
            "init",
            "--root",
            str(project_dir),
            "--model",
            CHAT_MODEL,
            "--embedding",
            EMBEDDING_MODEL,
            "--force",
        ],
        WORK_DIR,
        extra_env={"GRAPHRAG_API_KEY": API_KEY, "GRAPHRAG_API_BASE": API_BASE},
    )
    command_records.append(init_record)

    if init_record.status != "pass":
        return False

    patch_settings(project_dir / "settings.yaml")

    return True


def patch_settings(settings_path: Path) -> None:
    """Apply bounded model, chunking, and output configuration to settings.yaml."""

    if not settings_path.exists():
        return

    lines = settings_path.read_text(encoding="utf-8").splitlines()
    patched: list[str] = []
    inserted_api_base = False

    for line in lines:
        patched.append(line)
        stripped = line.strip()
        indent = line[: len(line) - len(line.lstrip())]

        if API_BASE and stripped.startswith("api_key:") and not inserted_api_base:
            patched.append(f"{indent}api_base: ${{GRAPHRAG_API_BASE}}")
            inserted_api_base = True

    patched.extend(
        [
            "",
            "# ELF GraphRAG smoke bounds.",
            "chunks:",
            "  size: 220",
            "  overlap: 20",
            "  prepend_metadata: false",
            "extract_graph:",
            "  max_gleanings: 0",
            "summarize_descriptions:",
            "  max_length: 160",
            "  max_input_length: 600",
            "community_reports:",
            "  max_length: 220",
            "  max_input_length: 800",
            "parallelization:",
            "  stagger: 0.0",
            "  num_threads: 1",
            "async_mode: threaded",
        ]
    )
    settings_path.write_text("\n".join(patched) + "\n", encoding="utf-8")


def run_graphrag(project_dir: Path, command_records: list[CommandRecord]) -> Path | None:
    """Run GraphRAG index and local query."""

    graphrag = WORK_DIR / ".venv" / "bin" / "graphrag"
    env = {"GRAPHRAG_API_KEY": API_KEY, "GRAPHRAG_API_BASE": API_BASE}
    index_record = run_command(
        "graphrag-index",
        [
            str(graphrag),
            "index",
            "--root",
            str(project_dir),
            "--method",
            INDEX_METHOD,
            "--cache",
        ],
        WORK_DIR,
        extra_env=env,
    )
    command_records.append(index_record)
    if index_record.status != "pass":
        return None

    output_dir = find_output_dir(project_dir)
    if output_dir is None:
        command_records.append(
            CommandRecord(
                label="graphrag-output-discovery",
                command=["find", str(project_dir / "output"), "-name", "*.parquet"],
                status="incomplete",
                elapsed_ms=0.0,
                stdout_artifact=None,
                stderr_artifact=None,
                returncode=None,
                reason="GraphRAG index completed but no parquet output directory was found.",
            )
        )

        return None

    query_record = run_command(
        "graphrag-query-local",
        [
            str(graphrag),
            "query",
            "--root",
            str(project_dir),
            "--method",
            QUERY_METHOD,
            "--data",
            str(output_dir),
            "--response-type",
            "Single Sentence",
            "What connects Nova Observatory and the Aurora Index in the generated corpus?",
        ],
        WORK_DIR,
        extra_env=env,
    )
    command_records.append(query_record)

    if query_record.status != "pass":
        return None

    return output_dir


def find_output_dir(project_dir: Path) -> Path | None:
    """Find a GraphRAG output directory containing parquet tables."""

    output_root = project_dir / "output"
    candidates: list[Path] = []

    if output_root.exists():
        for parquet in output_root.rglob("*.parquet"):
            candidates.append(parquet.parent)

    if not candidates:
        return None

    candidates.sort(key=lambda path: path.stat().st_mtime if path.exists() else 0.0)

    return candidates[-1]


def map_tables(output_dir: Path, corpus: list[dict[str, str]]) -> tuple[list[dict[str, Any]], list[str]]:
    """Map GraphRAG parquet table identifiers to real_world_job evidence ids."""

    try:
        import pandas as pd  # type: ignore[import-not-found]
    except ImportError as err:
        return (
            [
                {
                    "table": table,
                    "mapping_status": "reader_missing",
                    "error": f"pandas/pyarrow unavailable: {err}",
                    "row_count": 0,
                    "mapped_row_count": 0,
                    "rows": [],
                }
                for table in TABLES
            ],
            [],
        )

    table_paths = capture_table_artifacts(output_dir)
    mapped_by_table: dict[str, dict[str, list[str]]] = {}
    mappings: list[dict[str, Any]] = []

    for table in TABLES:
        path = table_paths.get(table)

        if path is None:
            mappings.append(
                {
                    "table": table,
                    "mapping_status": "missing_table",
                    "artifact": None,
                    "row_count": 0,
                    "mapped_row_count": 0,
                    "rows": [],
                }
            )
            mapped_by_table[table] = {}
            continue

        try:
            frame = pd.read_parquet(path)
        except Exception as err:  # noqa: BLE001
            mappings.append(
                {
                    "table": table,
                    "mapping_status": "read_failed",
                    "artifact": rel(path),
                    "error": str(err),
                    "row_count": 0,
                    "mapped_row_count": 0,
                    "rows": [],
                }
            )
            mapped_by_table[table] = {}
            continue

        rows, by_id = map_frame(table, frame, corpus, mapped_by_table)
        mapped_count = sum(1 for row in rows if row["evidence_ids"])
        status = "pass"

        if table in {"documents", "text_units"} and mapped_count < len(rows):
            status = "unmapped_required_rows"
        elif mapped_count == 0 and len(rows) > 0:
            status = "unmapped_rows"

        mappings.append(
            {
                "table": table,
                "mapping_status": status,
                "artifact": rel(path),
                "row_count": len(rows),
                "mapped_row_count": mapped_count,
                "rows": rows,
            }
        )
        mapped_by_table[table] = by_id

    evidence_ids: list[str] = []

    for mapping in mappings:
        for row in mapping["rows"]:
            for evidence_id in row["evidence_ids"]:
                if evidence_id not in evidence_ids:
                    evidence_ids.append(evidence_id)

    return mappings, evidence_ids


def empty_table_mappings(mapping_status: str) -> list[dict[str, Any]]:
    """Return explicit table mapping placeholders for non-live typed outcomes."""

    return [
        {
            "table": table,
            "mapping_status": mapping_status,
            "artifact": None,
            "row_count": 0,
            "mapped_row_count": 0,
            "rows": [],
        }
        for table in TABLES
    ]


def capture_table_artifacts(output_dir: Path) -> dict[str, Path]:
    """Copy known GraphRAG parquet tables into the report artifact directory."""

    table_paths: dict[str, Path] = {}

    if OUTPUT_CAPTURE_DIR.exists():
        shutil.rmtree(OUTPUT_CAPTURE_DIR)
    OUTPUT_CAPTURE_DIR.mkdir(parents=True, exist_ok=True)

    for table in TABLES:
        source = find_table_path(output_dir, table)

        if source is None:
            continue

        destination = OUTPUT_CAPTURE_DIR / f"{table}.parquet"
        shutil.copy2(source, destination)
        table_paths[table] = destination

    return table_paths


def find_table_path(output_dir: Path, table: str) -> Path | None:
    """Find a parquet file for a GraphRAG logical table name."""

    candidates = list(output_dir.rglob("*.parquet"))
    exact_names = {
        f"{table}.parquet",
        f"create_final_{table}.parquet",
        f"final_{table}.parquet",
    }

    for path in candidates:
        if path.name in exact_names:
            return path

    for path in candidates:
        stem = path.stem.lower()

        if stem.endswith(table) or stem == table or f"_{table}" in stem:
            return path

    return None


def map_frame(
    table: str,
    frame: Any,
    corpus: list[dict[str, str]],
    mapped_by_table: dict[str, dict[str, list[str]]],
) -> tuple[list[dict[str, Any]], dict[str, list[str]]]:
    """Map rows for a GraphRAG output table."""

    rows: list[dict[str, Any]] = []
    by_id: dict[str, list[str]] = {}

    for _, row in frame.iterrows():
        row_dict = {key: normalize_cell(value) for key, value in row.to_dict().items()}
        row_id = str(row_dict.get("id") or row_dict.get("human_readable_id") or row_dict.get("community") or "")
        evidence_ids = evidence_from_row(table, row_dict, corpus, mapped_by_table)
        rows.append(
            {
                "row_id": row_id,
                "human_readable_id": row_dict.get("human_readable_id"),
                "document_id": row_dict.get("document_id"),
                "community": row_dict.get("community"),
                "text_unit_ids": row_dict.get("text_unit_ids") or row_dict.get("text_units") or [],
                "evidence_ids": evidence_ids,
            }
        )

        if row_id:
            by_id[row_id] = evidence_ids

    return rows, by_id


def normalize_cell(value: Any) -> Any:
    """Normalize dataframe cell values into JSON-safe values."""

    if value is None:
        return None
    if hasattr(value, "tolist"):
        return normalize_cell(value.tolist())
    if isinstance(value, float) and value != value:
        return None
    if isinstance(value, (list, tuple, set)):
        return [normalize_cell(item) for item in value]
    if isinstance(value, dict):
        return {str(key): normalize_cell(item) for key, item in value.items()}

    return value


def evidence_from_row(
    table: str,
    row: dict[str, Any],
    corpus: list[dict[str, str]],
    mapped_by_table: dict[str, dict[str, list[str]]],
) -> list[str]:
    """Return mapped evidence ids for one output row."""

    evidence_ids: list[str] = []
    haystack = json.dumps(row, sort_keys=True, default=str)

    for item in corpus:
        evidence_id = item["evidence_id"]
        title = item["title"]
        signature = item["text"].split(".")[0]

        if (
            evidence_id in haystack
            or slug(evidence_id) in haystack
            or title in haystack
            or signature in haystack
        ):
            append_unique(evidence_ids, evidence_id)

    document_id = row.get("document_id")
    if document_id is not None:
        for evidence_id in mapped_by_table.get("documents", {}).get(str(document_id), []):
            append_unique(evidence_ids, evidence_id)

    for text_unit_id in row.get("text_unit_ids") or []:
        for evidence_id in mapped_by_table.get("text_units", {}).get(str(text_unit_id), []):
            append_unique(evidence_ids, evidence_id)

    if table == "community_reports":
        community = row.get("community")

        if community is not None:
            for candidate_id, candidate_evidence in mapped_by_table.get("communities", {}).items():
                if str(candidate_id) == str(community):
                    for evidence_id in candidate_evidence:
                        append_unique(evidence_ids, evidence_id)

    return evidence_ids


def append_unique(values: list[str], value: str) -> None:
    """Append a value if absent."""

    if value not in values:
        values.append(value)


def mapping_is_valid(mappings: list[dict[str, Any]], expected_ids: list[str]) -> tuple[bool, str]:
    """Validate source document/text-unit evidence mapping."""

    mapping_by_table = {mapping["table"]: mapping for mapping in mappings}

    for table in TABLES:
        mapping = mapping_by_table.get(table)

        if mapping is None or mapping["mapping_status"] in {"missing_table", "read_failed", "reader_missing"}:
            return False, f"GraphRAG output table {table} was not available for evidence mapping."

    for table in ("documents", "text_units"):
        mapping = mapping_by_table[table]

        if mapping["mapping_status"] != "pass":
            return False, f"GraphRAG {table} rows include identifiers that did not map to evidence ids."

    seen: list[str] = []
    for mapping in mappings:
        for row in mapping["rows"]:
            for evidence_id in row["evidence_ids"]:
                append_unique(seen, evidence_id)

    missing = [evidence_id for evidence_id in expected_ids if evidence_id not in seen]

    if missing:
        return False, f"GraphRAG output mappings missed expected evidence ids: {', '.join(missing)}."

    return True, "GraphRAG output tables mapped to expected generated evidence ids."


def write_materialization(
    status: StatusState,
    corpus: list[dict[str, str]],
    fixture_path: Path,
    corpus_csv: Path,
    command_records: list[CommandRecord],
    mappings: list[dict[str, Any]],
    mapped_ids: list[str],
    started_at: float,
) -> dict[str, Any]:
    """Write the primary smoke artifact."""

    cache_dir = WORK_DIR / "project" / "cache"
    output_dir = WORK_DIR / "project" / "output"
    elapsed_ms = (time.monotonic() - started_at) * 1000
    expected_ids = [item["evidence_id"] for item in corpus if item["evidence_id"] != "graphrag-smoke-stale-trap"]
    payload = {
        "schema": "elf.graphrag_docker_smoke/v1",
        "generated_at": utc_now(),
        "run_id": RUN_ID,
        "adapter_id": "graphrag_docker_smoke",
        "evidence_class": status.evidence_class,
        "status": {
            "setup": status.setup,
            "run": status.run,
            "result": status.result,
            "overall": status.overall,
            "failure_class": status.failure_class,
            "failure_reason": status.failure_reason,
        },
        "artifacts": {
            "generated_corpus_csv": rel(corpus_csv),
            "generated_fixture": rel(fixture_path),
            "graph_output_dir": rel(OUTPUT_CAPTURE_DIR),
            "manifest": rel(MANIFEST_OUT),
            "summary": rel(SUMMARY_OUT),
        },
        "docker_boundary": {
            "compose_file": "docker-compose.baseline.yml",
            "runner_service": "baseline-runner",
            "runner": "scripts/graphrag-docker-smoke.py",
            "host_global_installs_required": False,
            "docker_only": True,
        },
        "provider_configuration": {
            "package": GRAPH_RAG_REF,
            "package_spec": GRAPH_RAG_PACKAGE,
            "chat_model": CHAT_MODEL,
            "embedding_model": EMBEDDING_MODEL,
            "api_base_configured": bool(API_BASE),
            "api_key_provided": bool(API_KEY),
            "operator_owned_provider_credentials_used": False,
            "index_method": INDEX_METHOD,
            "query_method": QUERY_METHOD,
            "live_run_enabled": RUN_LIVE,
        },
        "resource_bounds": {
            "max_docs": MAX_DOCS,
            "max_input_chars": MAX_INPUT_CHARS,
            "actual_doc_count": len(corpus),
            "actual_input_chars": sum(len(item["text"]) for item in corpus),
            "timeout_seconds": TIMEOUT_SECONDS,
            "elapsed_ms": round(elapsed_ms, 3),
            "cache_size_bytes": dir_size(cache_dir),
            "cache_file_count": file_count(cache_dir),
            "output_size_bytes": dir_size(output_dir),
            "captured_output_size_bytes": dir_size(OUTPUT_CAPTURE_DIR),
            "model_call_observation": {
                "source": "GraphRAG cache artifact count when available",
                "observed_cache_entries": file_count(cache_dir),
                "raw_provider_usage_tokens_recorded": False,
            },
        },
        "commands": [command_to_json(record) for record in command_records],
        "evidence_mapping": {
            "expected_evidence_ids": expected_ids,
            "mapped_evidence_ids": mapped_ids,
            "tables": mappings,
        },
    }
    write_json(OUT, payload)

    return payload


def write_manifest(status: StatusState) -> dict[str, Any]:
    """Write a generated external adapter manifest for this smoke."""

    manifest = {
        "schema": "elf.real_world_external_adapter_manifest/v1",
        "manifest_id": f"graphrag-docker-smoke-{RUN_ID}",
        "docker_isolation": {
            "default": True,
            "compose_file": "docker-compose.baseline.yml",
            "runner": "scripts/graphrag-docker-smoke.py",
            "artifact_dir": "tmp/real-world-memory/graphrag-smoke",
            "host_global_installs_required": False,
            "notes": [
                f"Generated by the GraphRAG Docker smoke at {utc_now()}.",
                "The smoke uses a generated public corpus and records typed setup/runtime failures.",
            ],
        },
        "adapters": [
            {
                "adapter_id": "graphrag_docker_smoke",
                "project": "GraphRAG",
                "adapter_kind": "docker_python_cli_api_smoke",
                "evidence_class": status.evidence_class,
                "docker_default": True,
                "host_global_installs_required": False,
                "overall_status": status.overall,
                "setup": {
                    "status": status.setup,
                    "evidence": "The smoke runs inside the baseline Docker runner and installs or invokes GraphRAG only in the container-local work directory.",
                    "command": "cargo make graphrag-docker-smoke",
                    "artifact": rel(OUT),
                },
                "run": {
                    "status": status.run,
                    "evidence": "The live path generates a tiny public corpus, initializes GraphRAG, indexes with bounded inputs, and runs local search when provider config is supplied.",
                    "command": "ELF_GRAPHRAG_SMOKE_RUN=1 cargo make graphrag-docker-smoke",
                    "artifact": rel(OUT),
                },
                "result": {
                    "status": status.result,
                    "evidence": status.failure_reason
                    if status.failure_reason
                    else "GraphRAG parquet output tables mapped to generated real_world_job evidence ids.",
                    "artifact": rel(OUT),
                },
                "capabilities": [
                    {
                        "capability": "docker_python_cli_boundary",
                        "status": status.setup,
                        "evidence": "The runner is Python-only inside docker-compose.baseline.yml baseline-runner and does not require host-global GraphRAG installs.",
                    },
                    {
                        "capability": "graphrag_index_query",
                        "status": status.run,
                        "evidence": "The opt-in live path runs GraphRAG index and local query over the generated public corpus.",
                    },
                    {
                        "capability": "parquet_table_evidence_mapping",
                        "status": status.result,
                        "evidence": "documents, text_units, communities, community_reports, entities, and relationships parquet table identifiers are mapped to evidence ids when available.",
                    },
                    {
                        "capability": "quality_or_scale_claim",
                        "status": "not_encoded",
                        "evidence": "The smoke does not claim graph-navigation quality, synthesis quality, private-corpus behavior, or large-corpus indexing.",
                    },
                ],
                "suites": [
                    {
                        "suite_id": "knowledge_compilation",
                        "status": status.result,
                        "evidence": "Only the generated tiny-corpus table-mapping job is represented.",
                    },
                    {
                        "suite_id": "retrieval",
                        "status": status.run if status.run != "pass" else "not_encoded",
                        "evidence": "The smoke may run local search for reachability, but retrieval quality scoring is not encoded.",
                    },
                    {
                        "suite_id": "production_ops",
                        "status": "not_encoded",
                        "evidence": "The smoke records resource bounds but does not encode backup, restore, provider credential, or private corpus production-ops checks.",
                    },
                    {
                        "suite_id": "memory_evolution",
                        "status": "not_encoded",
                        "evidence": "GraphRAG update/delete/current-versus-historical behavior is not encoded by this smoke.",
                    },
                ],
                "evidence": [
                    {"kind": "artifact", "ref": rel(OUT), "status": status.result},
                    {"kind": "artifact", "ref": rel(OUTPUT_CAPTURE_DIR), "status": status.result},
                    {"kind": "manifest", "ref": rel(MANIFEST_OUT), "status": status.overall},
                    {"kind": "source", "ref": "https://github.com/microsoft/graphrag", "status": "real"},
                    {"kind": "source", "ref": "https://microsoft.github.io/graphrag/", "status": "real"},
                    {
                        "kind": "source",
                        "ref": "https://microsoft.github.io/graphrag/index/outputs/",
                        "status": "real",
                    },
                ],
                "execution_metadata": {
                    "sources": [
                        {
                            "label": "GraphRAG repository",
                            "url": "https://github.com/microsoft/graphrag",
                            "evidence": "Official source and package for GraphRAG.",
                        },
                        {
                            "label": "GraphRAG CLI docs",
                            "url": "https://microsoft.github.io/graphrag/cli/",
                            "evidence": "Official index and query command contract.",
                        },
                        {
                            "label": "GraphRAG input docs",
                            "url": "https://microsoft.github.io/graphrag/index/inputs/",
                            "evidence": "Official input formats and document schema.",
                        },
                        {
                            "label": "GraphRAG output tables",
                            "url": "https://microsoft.github.io/graphrag/index/outputs/",
                            "evidence": "Official parquet output table schema for evidence mapping.",
                        },
                        {
                            "label": "GraphRAG local search docs",
                            "url": "https://microsoft.github.io/graphrag/query/local_search/",
                            "evidence": "Official local-search context and graph traversal reference.",
                        },
                    ],
                    "setup_path": "Run cargo make graphrag-docker-smoke for a typed artifact; set ELF_GRAPHRAG_SMOKE_RUN=1 with explicit provider configuration for a live index/query attempt.",
                    "runtime_boundary": "docker-compose.baseline.yml baseline-runner, container-local Python venv, generated public corpus, and report artifacts under tmp/real-world-memory/graphrag-smoke.",
                    "resource_expectation": f"GraphRAG package {GRAPH_RAG_REF}, max_docs={MAX_DOCS}, max_input_chars={MAX_INPUT_CHARS}, timeout_seconds={TIMEOUT_SECONDS}, index_method={INDEX_METHOD}.",
                    "retry_guidance": [
                        "Default command records a typed blocked artifact without model calls.",
                        "Enable the live path only with explicit provider configuration and generated public corpus.",
                        "Treat missing or unmapped documents/text_units as wrong_result, not as pass.",
                    ],
                    "research_depth": "D2 feasibility plus XY-887 cost-bounded Docker smoke implementation; generated artifact decides live evidence class.",
                },
                "notes": [
                    "The checked-in manifest record remains research_gate; generated smoke artifacts carry live status.",
                    "Failure before GraphRAG output remains typed as blocked or incomplete.",
                    "The smoke does not use private corpora or unrecorded provider credentials.",
                ],
            }
        ],
    }
    write_json(MANIFEST_OUT, manifest)

    return manifest


def write_summary(materialization: dict[str, Any], manifest: dict[str, Any]) -> None:
    """Write a small summary artifact."""

    write_json(
        SUMMARY_OUT,
        {
            "schema": "elf.graphrag_docker_smoke_summary/v1",
            "generated_at": utc_now(),
            "adapter_id": "graphrag_docker_smoke",
            "evidence_class": materialization["evidence_class"],
            "materialization": materialization,
            "manifest": {
                "json": rel(MANIFEST_OUT),
                "summary": manifest["adapters"][0]["overall_status"],
                "suites": manifest["adapters"][0]["suites"],
            },
        },
    )


def scrub_report_secrets(project_dir: Path) -> None:
    """Remove provider secrets from text artifacts before reporting."""

    if not API_KEY:
        return

    for root in (project_dir, LOG_DIR):
        if not root.exists():
            continue

        for path in root.rglob("*"):
            if not path.is_file() or path.suffix not in {".env", ".json", ".log", ".txt", ".yaml", ".yml"}:
                continue

            try:
                content = path.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                continue

            if API_KEY in content:
                path.write_text(content.replace(API_KEY, "<redacted>"), encoding="utf-8")


def main() -> int:
    """Run the smoke and always emit typed artifacts when possible."""

    started_at = time.monotonic()
    mkdirs()
    status = StatusState()
    command_records: list[CommandRecord] = []
    mappings: list[dict[str, Any]] = empty_table_mappings("not_encoded")
    mapped_ids: list[str] = []
    corpus = generated_corpus()
    project_dir = WORK_DIR / "project"
    corpus_csv = write_corpus(project_dir, corpus)

    if not Path("/.dockerenv").exists() and not ALLOW_HOST:
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "not_running_in_docker"
        status.failure_reason = "GraphRAG smoke must run inside Docker; use cargo make graphrag-docker-smoke."
    elif not command_available("python3"):
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "python_missing"
        status.failure_reason = "python3 is required for the GraphRAG smoke runner."
    elif not RUN_LIVE:
        pass
    elif not API_KEY:
        status.setup = "blocked"
        status.run = "not_encoded"
        status.result = "blocked"
        status.overall = "blocked"
        status.failure_class = "provider_api_key_missing"
        status.failure_reason = "GraphRAG live indexing requires an explicit provider API key; no private or unrecorded provider credentials were used."
    elif not init_project(project_dir, command_records):
        status.setup = "incomplete"
        status.run = "not_encoded"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "graphrag_setup_failed"
        status.failure_reason = "GraphRAG installation or initialization failed inside the Docker runner."
    else:
        status.setup = "pass"
        output_dir = run_graphrag(project_dir, command_records)

        if output_dir is None:
            status.run = "incomplete"
            status.result = "incomplete"
            status.overall = "incomplete"
            status.failure_class = "graphrag_index_or_query_failed"
            status.failure_reason = "GraphRAG did not complete both index and local query for the generated corpus."
        else:
            status.run = "pass"
            status.evidence_class = "live_real_world"
            mappings, mapped_ids = map_tables(output_dir, corpus)
            expected_ids = [
                item["evidence_id"]
                for item in corpus
                if item["evidence_id"] != "graphrag-smoke-stale-trap"
            ]
            valid, reason = mapping_is_valid(mappings, expected_ids)

            if valid:
                status.result = "pass"
                status.overall = "pass"
                status.failure_class = ""
                status.failure_reason = ""
            else:
                status.result = "wrong_result"
                status.overall = "wrong_result"
                status.failure_class = "graphrag_evidence_mapping_failed"
                status.failure_reason = reason

    scrub_report_secrets(project_dir)
    fixture_path = write_fixture(corpus, status, mapped_ids)
    materialization = write_materialization(
        status,
        corpus,
        fixture_path,
        corpus_csv,
        command_records,
        mappings,
        mapped_ids,
        started_at,
    )
    manifest = write_manifest(status)
    write_summary(materialization, manifest)
    print(f"GraphRAG smoke artifact: {OUT}")
    print(f"GraphRAG smoke manifest: {MANIFEST_OUT}")
    print(f"GraphRAG smoke summary: {SUMMARY_OUT}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
