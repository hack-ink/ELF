#!/usr/bin/env python3
"""Docker-contained graphify graph/report smoke for real-world adapters."""

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
        "ELF_GRAPHIFY_SMOKE_REPORT_DIR",
        ROOT_DIR / "tmp" / "real-world-memory" / "graphify-smoke",
    )
)
WORK_DIR = Path(os.environ.get("ELF_GRAPHIFY_SMOKE_WORK_DIR", REPORT_DIR / "work"))
OUT = Path(os.environ.get("ELF_GRAPHIFY_SMOKE_OUT", REPORT_DIR / "graphify-smoke.json"))
MANIFEST_OUT = Path(
    os.environ.get(
        "ELF_GRAPHIFY_SMOKE_MANIFEST_OUT",
        REPORT_DIR / "memory_projects_manifest.graphify-smoke.json",
    )
)
SUMMARY_OUT = Path(os.environ.get("ELF_GRAPHIFY_SMOKE_SUMMARY_OUT", REPORT_DIR / "summary.json"))
REPORT_JSON = Path(os.environ.get("ELF_GRAPHIFY_SMOKE_REPORT_JSON", REPORT_DIR / "graphify-report.json"))
REPORT_MD = Path(os.environ.get("ELF_GRAPHIFY_SMOKE_REPORT_MD", REPORT_DIR / "graphify-report.md"))
FIXTURE_DIR = REPORT_DIR / "graphify-fixtures"
CORPUS_DIR = WORK_DIR / "generated-public-corpus"
OUTPUT_CAPTURE_DIR = REPORT_DIR / "graphify-out"
LOG_DIR = REPORT_DIR / "logs"

RUN_ID = os.environ.get(
    "ELF_GRAPHIFY_SMOKE_RUN_ID",
    f"graphify-docker-smoke-{datetime.now(timezone.utc).strftime('%Y%m%d%H%M%S')}",
)
RUN_GRAPHIFY = os.environ.get("ELF_GRAPHIFY_SMOKE_RUN", "1") == "1"
ALLOW_HOST = os.environ.get("ELF_GRAPHIFY_SMOKE_ALLOW_HOST", "0") == "1"
INSTALL_GRAPHIFY = os.environ.get("ELF_GRAPHIFY_SMOKE_INSTALL", "1") == "1"
GRAPHIFY_PACKAGE = os.environ.get("ELF_GRAPHIFY_PACKAGE", "graphifyy")
GRAPHIFY_REF = os.environ.get("ELF_GRAPHIFY_REF", f"pypi:{GRAPHIFY_PACKAGE}")
TIMEOUT_SECONDS = int(os.environ.get("ELF_GRAPHIFY_TIMEOUT_SECONDS", "600"))
QUERY_BUDGET = int(os.environ.get("ELF_GRAPHIFY_QUERY_BUDGET", "1200"))


@dataclass
class CorpusItem:
    """Generated public corpus item with source mapping metadata."""

    evidence_id: str
    claim_id: str
    title: str
    file_name: str
    text: str
    expected: bool
    kind: str = "document"
    line: int = 1


@dataclass
class StatusState:
    """Typed status for generated graphify smoke artifacts."""

    setup: str = "blocked"
    run: str = "not_encoded"
    result: str = "blocked"
    overall: str = "blocked"
    evidence_class: str = "research_gate"
    failure_class: str = "graphify_live_run_disabled"
    failure_reason: str = (
        "graphify graph/report execution is disabled; set ELF_GRAPHIFY_SMOKE_RUN=1 "
        "to install and run graphify inside Docker."
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
    """Create and reset output directories owned by this smoke."""

    for path in (FIXTURE_DIR, OUTPUT_CAPTURE_DIR, LOG_DIR):
        if path.exists():
            shutil.rmtree(path)

    for path in (REPORT_DIR, WORK_DIR, FIXTURE_DIR, OUTPUT_CAPTURE_DIR, LOG_DIR):
        path.mkdir(parents=True, exist_ok=True)

    for path in (
        OUT,
        MANIFEST_OUT,
        SUMMARY_OUT,
        REPORT_JSON,
        REPORT_MD,
        REPORT_DIR / "generated-corpus.csv",
    ):
        if path.exists():
            path.unlink()


def write_json(path: Path, payload: Any) -> None:
    """Write stable, pretty JSON."""

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def run_scored_report(fixture_path: Path, manifest_path: Path, status: StatusState) -> dict[str, Any]:
    """Score the generated graphify fixture through the real-world job runner."""

    run_cmd = [
        "cargo",
        "run",
        "-p",
        "elf-eval",
        "--bin",
        "real_world_job_benchmark",
        "--",
        "run",
        "--fixtures",
        str(fixture_path),
        "--out",
        str(REPORT_JSON),
        "--run-id",
        "real-world-memory-live-graphify",
        "--adapter-id",
        "graphify_docker_smoke",
        "--adapter-name",
        "graphify Docker graph/report smoke adapter",
        "--adapter-behavior",
        "docker_cli_graph_report_smoke",
        "--adapter-storage-status",
        status.setup,
        "--adapter-runtime-status",
        status.overall,
        "--adapter-notes",
        "Generated by the graphify Docker graph/report smoke; pass or wrong_result requires graph.json, GRAPH_REPORT.md, and query output mapped to generated evidence ids, while setup/runtime limits remain typed.",
        "--external-adapter-manifest",
        str(manifest_path),
    ]
    publish_cmd = [
        "cargo",
        "run",
        "-p",
        "elf-eval",
        "--bin",
        "real_world_job_benchmark",
        "--",
        "publish",
        "--report",
        str(REPORT_JSON),
        "--out",
        str(REPORT_MD),
    ]

    subprocess.run(run_cmd, cwd=ROOT_DIR, check=True)
    subprocess.run(publish_cmd, cwd=ROOT_DIR, check=True)

    report = json.loads(REPORT_JSON.read_text(encoding="utf-8"))

    return {
        "json": rel(REPORT_JSON),
        "markdown": rel(REPORT_MD),
        "summary": report.get("summary", {}),
        "suites": report.get("suites", []),
    }


def scored_benchmark(report: dict[str, Any] | None) -> dict[str, Any]:
    """Extract the post-score benchmark status from a real_world_job report."""

    if report is None:
        return {
            "schema": "elf.scored_benchmark_status/v1",
            "source": "real_world_job_benchmark",
            "status": "pending",
            "reason": "The smoke materialization was written before benchmark scoring completed.",
        }

    summary = report.get("summary", {})
    counts = {
        status: int(summary.get(status, 0) or 0)
        for status in (
            "pass",
            "wrong_result",
            "lifecycle_fail",
            "incomplete",
            "blocked",
            "not_encoded",
        )
    }
    status = next((name for name, count in counts.items() if name != "pass" and count > 0), "pass")

    return {
        "schema": "elf.scored_benchmark_status/v1",
        "source": "real_world_job_benchmark",
        "status": status,
        "counts": counts,
        "job_count": int(summary.get("job_count", 0) or 0),
        "mean_score": summary.get("mean_score"),
        "evidence_coverage": summary.get("evidence_coverage"),
    }


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


def runtime_env() -> dict[str, str]:
    """Return an isolated graphify runtime environment."""

    home = WORK_DIR / "home"
    return {
        "HOME": str(home),
        "XDG_CONFIG_HOME": str(home / ".config"),
        "XDG_CACHE_HOME": str(home / ".cache"),
        "CODEX_HOME": str(home / ".codex"),
        "CLAUDE_CONFIG_DIR": str(home / ".claude"),
        "GEMINI_HOME": str(home / ".gemini"),
        "PYTHONUNBUFFERED": "1",
        "NO_COLOR": "1",
    }


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


def generated_corpus() -> list[CorpusItem]:
    """Return the bounded generated-public graphify corpus."""

    return [
        CorpusItem(
            evidence_id="graphify-smoke-memory-service",
            claim_id="memory_service_graph",
            title="ELF Memory Service Graph Note",
            file_name="elf_memory_service.py",
            text=(
                '"""Evidence ID graphify-smoke-memory-service.\n'
                "ELF stores evidence-linked facts as notes and keeps Postgres as the "
                "source of truth for graph/report validation.\n"
                '"""\n\n'
                "class ElfMemoryService:\n"
                "    \"\"\"Evidence ID graphify-smoke-memory-service maps memory notes "
                "to source-backed graph nodes.\"\"\"\n\n"
                "    def attach_evidence(self, note_id: str, source_ref: str) -> tuple[str, str]:\n"
                "        \"\"\"Attach source_ref evidence to a note before retrieval.\"\"\"\n"
                "        return note_id, source_ref\n"
            ),
            expected=True,
        ),
        CorpusItem(
            evidence_id="graphify-smoke-qdrant-rebuild",
            claim_id="qdrant_rebuild_graph",
            title="Qdrant Rebuild Graph Note",
            file_name="qdrant_rebuild.py",
            text=(
                '"""Evidence ID graphify-smoke-qdrant-rebuild.\n'
                "Qdrant is a derived, rebuildable index. The graphify smoke should "
                "connect Qdrant rebuild evidence to the ELF memory service node and "
                "preserve this source file as evidence for scoring.\n"
                '"""\n\n'
                "class QdrantRebuildIndex:\n"
                "    \"\"\"Evidence ID graphify-smoke-qdrant-rebuild maps rebuildable "
                "index behavior to source evidence.\"\"\"\n\n"
                "    def rebuild_from_postgres_vectors(self, collection: str) -> str:\n"
                "        \"\"\"Rebuild the derived Qdrant collection from Postgres vectors.\"\"\"\n"
                "        return collection\n"
            ),
            expected=True,
        ),
        CorpusItem(
            evidence_id="graphify-smoke-report-mapping",
            claim_id="graph_report_mapping",
            title="Graph Report Mapping Note",
            file_name="graph_report_mapping.py",
            text=(
                '"""Evidence ID graphify-smoke-report-mapping.\n'
                "GRAPH_REPORT.md and graph.json must be captured as derived adapter "
                "artifacts, then mapped back to real_world_job evidence ids.\n"
                '"""\n\n'
                "def map_graph_report_to_evidence(graph_json: str, graph_report: str) -> str:\n"
                "    \"\"\"Return graphify-smoke-report-mapping when graph artifacts cite sources.\"\"\"\n"
                "    return f\"{graph_json}:{graph_report}\"\n"
            ),
            expected=True,
        ),
        CorpusItem(
            evidence_id="graphify-smoke-stale-trap",
            claim_id="stale_authority_trap",
            title="Stale Graph Authority Trap",
            file_name="stale_vector_authority.py",
            text=(
                '"""Evidence ID graphify-smoke-stale-trap.\n'
                "Stale trap: graphify output is an authoritative ELF memory store. "
                "This is intentionally false; graphify is only a derived graph/report adapter.\n"
                '"""\n\n'
                "def stale_authority_claim() -> str:\n"
                "    \"\"\"Return the stale claim that must not drive the answer.\"\"\"\n"
                "    return \"graphify is authoritative\"\n"
            ),
            expected=False,
        ),
    ]


def write_corpus(corpus: list[CorpusItem]) -> Path:
    """Write graphify input files plus a CSV mapping copy."""

    if CORPUS_DIR.exists():
        shutil.rmtree(CORPUS_DIR)
    CORPUS_DIR.mkdir(parents=True, exist_ok=True)
    csv_path = REPORT_DIR / "generated-corpus.csv"

    with csv_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(
            handle,
            fieldnames=("evidence_id", "claim_id", "title", "file_name", "line", "text"),
        )
        writer.writeheader()

        for item in corpus:
            line = evidence_line(item.text, item.evidence_id)
            item.line = line
            writer.writerow(
                {
                    "evidence_id": item.evidence_id,
                    "claim_id": item.claim_id,
                    "title": item.title,
                    "file_name": item.file_name,
                    "line": line,
                    "text": item.text,
                }
            )
            (CORPUS_DIR / item.file_name).write_text(item.text, encoding="utf-8")

    (CORPUS_DIR / ".graphifyignore").write_text(
        "graphify-out/\n__pycache__/\n*.pyc\n",
        encoding="utf-8",
    )

    return csv_path


def evidence_line(text: str, evidence_id: str) -> int:
    """Return the first line containing an evidence id."""

    for index, line in enumerate(text.splitlines(), start=1):
        if evidence_id in line:
            return index

    return 1


def install_graphify(command_records: list[CommandRecord]) -> Path | None:
    """Create a venv and install graphify in the container-local work dir."""

    venv_dir = WORK_DIR / ".venv"
    python = venv_dir / "bin" / "python"
    graphify = venv_dir / "bin" / "graphify"

    if INSTALL_GRAPHIFY:
        venv_record = run_command("python-venv", [sys.executable, "-m", "venv", str(venv_dir)], WORK_DIR)
        command_records.append(venv_record)
        if venv_record.status != "pass":
            return None

        install_record = run_command(
            "graphify-install",
            [str(python), "-m", "pip", "install", "--disable-pip-version-check", GRAPHIFY_PACKAGE],
            WORK_DIR,
            extra_env=runtime_env(),
        )
        command_records.append(install_record)
        if install_record.status != "pass":
            return None
    elif not graphify.exists():
        command_records.append(
            CommandRecord(
                label="graphify-install",
                command=["graphify"],
                status="incomplete",
                elapsed_ms=0.0,
                stdout_artifact=None,
                stderr_artifact=None,
                returncode=None,
                reason="graphify install was disabled and no venv graphify executable exists.",
            )
        )
        return None

    version_record = run_command("graphify-help", [str(graphify), "--help"], WORK_DIR, extra_env=runtime_env())
    command_records.append(version_record)

    return graphify if version_record.status == "pass" else None


def run_graphify(graphify: Path, command_records: list[CommandRecord]) -> Path | None:
    """Run graphify build and query commands."""

    build_record = run_command(
        "graphify-build",
        [str(graphify), str(CORPUS_DIR), "--no-viz"],
        WORK_DIR,
        extra_env=runtime_env(),
    )
    command_records.append(build_record)
    if build_record.status != "pass":
        return None

    cluster_record = run_command(
        "graphify-cluster-report",
        [str(graphify), "cluster-only", str(CORPUS_DIR)],
        WORK_DIR,
        extra_env=runtime_env(),
    )
    command_records.append(cluster_record)

    output_dir = find_graphify_output_dir()

    if output_dir is None:
        command_records.append(
            CommandRecord(
                label="graphify-output-discovery",
                command=["find", str(WORK_DIR), "-path", "*/graphify-out/graph.json"],
                status="incomplete",
                elapsed_ms=0.0,
                stdout_artifact=None,
                stderr_artifact=None,
                returncode=None,
                reason="graphify completed but graphify-out/graph.json was not found.",
            )
        )
        return None

    copy_graphify_output(output_dir)
    graph_json = OUTPUT_CAPTURE_DIR / "graph.json"
    query_record = run_command(
        "graphify-query",
        [
            str(graphify),
            "query",
            "what connects the ELF memory service, Qdrant rebuild, and graph report evidence mapping?",
            "--graph",
            str(graph_json),
            "--budget",
            str(QUERY_BUDGET),
        ],
        WORK_DIR,
        extra_env=runtime_env(),
    )
    command_records.append(query_record)

    return OUTPUT_CAPTURE_DIR


def find_graphify_output_dir() -> Path | None:
    """Find the graphify output directory generated by the CLI."""

    candidates: list[Path] = []

    for base in (WORK_DIR, CORPUS_DIR):
        if not base.exists():
            continue

        for graph_path in base.rglob("graph.json"):
            if ".venv" in graph_path.parts:
                continue
            if graph_path.parent.name == "graphify-out":
                candidates.append(graph_path.parent)

    if not candidates:
        return None

    candidates.sort(key=lambda path: path.stat().st_mtime if path.exists() else 0.0)

    return candidates[-1]


def copy_graphify_output(output_dir: Path) -> None:
    """Copy graphify output artifacts into the report directory."""

    if OUTPUT_CAPTURE_DIR.exists():
        shutil.rmtree(OUTPUT_CAPTURE_DIR)
    shutil.copytree(output_dir, OUTPUT_CAPTURE_DIR)


def map_artifacts(corpus: list[CorpusItem], command_records: list[CommandRecord]) -> dict[str, Any]:
    """Map graphify graph/report/query output to real_world_job evidence ids."""

    graph_json = OUTPUT_CAPTURE_DIR / "graph.json"
    graph_report = OUTPUT_CAPTURE_DIR / "GRAPH_REPORT.md"
    graph_payload = read_json_or_none(graph_json)
    nodes, edges = extract_graph_rows(graph_payload)
    node_mappings = [map_graph_row("node", row, corpus) for row in nodes]
    edge_mappings = [map_graph_row("edge", row, corpus) for row in edges]
    report_mapping = map_text_artifact("graph_report", graph_report, corpus)
    query_mapping = map_query_output(command_records, corpus)
    mapped_ids: list[str] = []

    for section in (node_mappings, edge_mappings):
        for row in section:
            for evidence_id in row["evidence_ids"]:
                append_unique(mapped_ids, evidence_id)

    for row in (report_mapping, query_mapping):
        for evidence_id in row["evidence_ids"]:
            append_unique(mapped_ids, evidence_id)

    return {
        "expected_evidence_ids": expected_ids(corpus),
        "mapped_evidence_ids": mapped_ids,
        "graph_json": {
            "artifact": rel(graph_json) if graph_json.exists() else None,
            "exists": graph_json.exists(),
            "size_bytes": graph_json.stat().st_size if graph_json.exists() else 0,
        },
        "graph_report": report_mapping,
        "query_output": query_mapping,
        "nodes": node_mappings,
        "edges": edge_mappings,
    }


def read_json_or_none(path: Path) -> Any | None:
    """Read JSON and return None on missing or invalid payloads."""

    if not path.exists():
        return None

    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return None


def extract_graph_rows(payload: Any | None) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    """Extract node and edge rows from common graph JSON shapes."""

    if not isinstance(payload, dict):
        return [], []

    nodes = payload.get("nodes")
    edges = payload.get("edges") or payload.get("links") or payload.get("relationships")

    if nodes is None and isinstance(payload.get("elements"), dict):
        elements = payload["elements"]
        nodes = elements.get("nodes")
        edges = elements.get("edges")

    return rows_from_value(nodes), rows_from_value(edges)


def rows_from_value(value: Any) -> list[dict[str, Any]]:
    """Normalize a graph row container into dictionaries."""

    if not isinstance(value, list):
        return []

    rows: list[dict[str, Any]] = []
    for item in value:
        if isinstance(item, dict):
            data = item.get("data")
            rows.append(data if isinstance(data, dict) else item)

    return rows


def map_graph_row(kind: str, row: dict[str, Any], corpus: list[CorpusItem]) -> dict[str, Any]:
    """Map one graph node or edge row to evidence ids."""

    blob = json.dumps(row, sort_keys=True, default=str)
    evidence_ids = evidence_from_text(blob, corpus)
    return {
        "kind": kind,
        "row_id": str(row.get("id") or row.get("key") or row.get("source") or ""),
        "label": first_text(row, ("label", "name", "title", "type", "kind")),
        "edge_type": first_text(row, ("edge_type", "type", "relation", "relationship", "predicate")),
        "confidence": first_text(
            row,
            ("confidence", "confidence_score", "confidence_tag", "extraction_status", "status"),
        ),
        "source_files": source_values(row),
        "source_locations": source_location_values(row),
        "evidence_ids": evidence_ids,
    }


def first_text(row: dict[str, Any], keys: tuple[str, ...]) -> str | None:
    """Return the first scalar text value for a set of keys."""

    for key in keys:
        value = row.get(key)

        if isinstance(value, (str, int, float)):
            return str(value)

    return None


def source_values(value: Any) -> list[str]:
    """Collect source file-ish values from a graph row."""

    values: list[str] = []
    collect_source_values(value, values, ("source", "file", "path"))

    return values[:12]


def source_location_values(value: Any) -> list[str]:
    """Collect source location-ish values from a graph row."""

    values: list[str] = []
    collect_source_values(value, values, ("location", "line", "span", "range"))

    return values[:12]


def collect_source_values(value: Any, out: list[str], key_fragments: tuple[str, ...]) -> None:
    """Recursively collect bounded source-related values."""

    if isinstance(value, dict):
        for key, item in value.items():
            key_lower = key.lower()

            if any(fragment in key_lower for fragment in key_fragments) and isinstance(item, (str, int, float)):
                append_unique(out, str(item))
            else:
                collect_source_values(item, out, key_fragments)
    elif isinstance(value, list):
        for item in value:
            collect_source_values(item, out, key_fragments)


def map_text_artifact(kind: str, path: Path, corpus: list[CorpusItem]) -> dict[str, Any]:
    """Map a text artifact to evidence ids."""

    text = ""
    if path.exists():
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            text = ""

    return {
        "kind": kind,
        "artifact": rel(path) if path.exists() else None,
        "exists": path.exists(),
        "size_bytes": path.stat().st_size if path.exists() else 0,
        "evidence_ids": evidence_from_text(text, corpus),
    }


def map_query_output(command_records: list[CommandRecord], corpus: list[CorpusItem]) -> dict[str, Any]:
    """Map graphify query stdout to evidence ids."""

    query_record = next((record for record in command_records if record.label == "graphify-query"), None)
    text = ""
    artifact = query_record.stdout_artifact if query_record else None

    if artifact:
        path = ROOT_DIR / artifact
        if path.exists():
            text = path.read_text(encoding="utf-8")

    return {
        "kind": "query_output",
        "artifact": artifact,
        "exists": bool(artifact and (ROOT_DIR / artifact).exists()),
        "command_status": query_record.status if query_record else "not_encoded",
        "evidence_ids": evidence_from_text(text, corpus),
    }


def evidence_from_text(text: str, corpus: list[CorpusItem]) -> list[str]:
    """Return evidence ids whose signatures appear in a text blob."""

    evidence_ids: list[str] = []
    haystack = text.lower()

    for item in corpus:
        signatures = (
            item.evidence_id,
            slug(item.evidence_id),
            item.file_name,
            item.title,
            f"{item.file_name}:{item.line}",
        )

        if any(signature.lower() in haystack for signature in signatures):
            append_unique(evidence_ids, item.evidence_id)

    return evidence_ids


def append_unique(values: list[str], value: str) -> None:
    """Append a value if absent."""

    if value not in values:
        values.append(value)


def expected_ids(corpus: list[CorpusItem]) -> list[str]:
    """Return expected evidence ids for pass scoring."""

    return [item.evidence_id for item in corpus if item.expected]


def mapping_outcome(mappings: dict[str, Any], command_records: list[CommandRecord]) -> tuple[str, str]:
    """Return typed result status and explanation for evidence mapping."""

    graph_build = next((record for record in command_records if record.label == "graphify-build"), None)
    graph_query = next((record for record in command_records if record.label == "graphify-query"), None)

    if graph_build is None or graph_build.status != "pass":
        return "incomplete", "graphify did not complete graph/report build for the generated corpus."
    if not mappings["graph_json"]["exists"]:
        return "incomplete", "graphify did not produce graph.json."
    if not mappings["graph_report"]["exists"]:
        return "incomplete", "graphify did not produce GRAPH_REPORT.md."
    if graph_query is None or graph_query.status != "pass":
        return "incomplete", "graphify query output was not available for scoring."

    missing = [
        evidence_id
        for evidence_id in mappings["expected_evidence_ids"]
        if evidence_id not in mappings["mapped_evidence_ids"]
    ]

    if missing:
        return "wrong_result", f"graphify output mappings missed expected evidence ids: {', '.join(missing)}."

    return "pass", "graphify graph/report/query output mapped to expected generated evidence ids."


def write_fixture(corpus: list[CorpusItem], status: StatusState, mapped_ids: list[str]) -> Path:
    """Write a generated real_world_job fixture for the graphify smoke."""

    fixture_path = FIXTURE_DIR / "knowledge" / "graphify_graph_report.json"
    used_ids = [evidence_id for evidence_id in mapped_ids if evidence_id in expected_ids(corpus)]
    response = {
        "adapter_id": "graphify_docker_smoke",
        "answer": {
            "content": (
                "graphify connected the ELF memory service, Qdrant rebuild, and graph report mapping "
                "through graph/report artifacts that cite generated source evidence."
                if used_ids
                else ""
            ),
            "claims": [
                {
                    "claim_id": "graphify_report_evidence_mapping",
                    "text": (
                        "graphify graph/report artifacts map back to the generated ELF memory service, "
                        "Qdrant rebuild, and report mapping evidence ids."
                    ),
                    "evidence_ids": used_ids,
                    "confidence": "derived_from_graphify_graph_report_mapping",
                }
            ]
            if used_ids
            else [],
            "evidence_ids": used_ids,
            "pages": [
                {
                    "page_id": "graphify:graph-report",
                    "page_type": "concept",
                    "title": "graphify Graph Report",
                    "path": rel(OUTPUT_CAPTURE_DIR / "GRAPH_REPORT.md"),
                    "sections": [
                        {
                            "section_id": "derived-graph-report",
                            "heading": "Derived Graph Report",
                            "role": "summary",
                            "content": "GRAPH_REPORT.md is a derived graphify artifact, not authoritative ELF memory.",
                            "evidence_ids": used_ids,
                            "timeline_event_ids": ["graphify-smoke-built-graph-report"],
                            "unsupported_reason": None if used_ids else "graphify output was not mapped.",
                        }
                    ],
                    "backlinks": used_ids,
                    "lint_findings": [],
                }
            ]
            if (OUTPUT_CAPTURE_DIR / "GRAPH_REPORT.md").exists()
            else [],
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
        "job_id": "graphify-graph-report-001",
        "suite": "knowledge_compilation",
        "title": "Map graphify graph/report output to generated evidence",
        "corpus": {
            "corpus_id": "graphify-generated-public-smoke",
            "profile": "generated_public",
            "items": [
                {
                    "evidence_id": item.evidence_id,
                    "kind": item.kind,
                    "text": item.text,
                    "source_ref": {
                        "schema": "source_ref/v1",
                        "resolver": "graphify_smoke/v1",
                        "ref": {
                            "run_id": RUN_ID,
                            "file": item.file_name,
                            "line": item.line,
                            "evidence_id": item.evidence_id,
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
                "event_id": "graphify-smoke-corpus-generated",
                "ts": "2026-06-10T00:00:00Z",
                "actor": "system",
                "action": "generated_public_corpus",
                "evidence_ids": expected_ids(corpus),
                "summary": "The graphify smoke generated a tiny public corpus for source mapping.",
            },
            {
                "event_id": "graphify-smoke-built-graph-report",
                "ts": "2026-06-10T00:01:00Z",
                "actor": "system",
                "action": "built_derived_graph_report",
                "evidence_ids": used_ids,
                "summary": "graphify built derived graph/report artifacts when the Docker smoke reached execution.",
            },
        ],
        "prompt": {
            "role": "user",
            "content": "What does graphify connect in the generated ELF graph/report smoke?",
            "job_mode": "compile",
            "constraints": ["cite_evidence", "avoid_stale_facts", "do_not_claim_authoritative_store"],
        },
        "expected_answer": {
            "must_include": [
                {
                    "claim_id": "graphify_report_evidence_mapping",
                    "text": (
                        "graphify connects the ELF memory service, Qdrant rebuild, and graph report "
                        "mapping through derived graph/report artifacts."
                    ),
                }
            ],
            "must_not_include": ["graphify output is an authoritative ELF memory store."],
            "evidence_links": {"graphify_report_evidence_mapping": expected_ids(corpus)},
            "answer_type": "compiled_knowledge",
            "accepted_alternates": [],
            "requires_caveat": True,
            "requires_refusal": False,
        },
        "required_evidence": [
            {
                "evidence_id": item.evidence_id,
                "claim_id": "graphify_report_evidence_mapping",
                "requirement": "cite",
                "quote": item.evidence_id,
            }
            for item in corpus
            if item.expected
        ],
        "negative_traps": [
            {
                "trap_id": "graphify-authoritative-store",
                "type": "unsupported_claim",
                "evidence_ids": ["graphify-smoke-stale-trap"],
                "failure_if_used": True,
            }
        ],
        "scoring_rubric": {
            "dimensions": {
                "answer_correctness": {
                    "weight": 0.25,
                    "max_points": 1.0,
                    "criteria": "States the graph/report connection without broad quality claims.",
                },
                "evidence_grounding": {
                    "weight": 0.4,
                    "max_points": 1.0,
                    "criteria": "Maps graphify output back to generated evidence ids.",
                },
                "trap_avoidance": {
                    "weight": 0.2,
                    "max_points": 1.0,
                    "criteria": "Does not treat graphify output as an authoritative ELF memory store.",
                },
                "latency_resource": {
                    "weight": 0.15,
                    "max_points": 1.0,
                    "criteria": "Records build time, artifact sizes, provider boundary, and retry behavior.",
                },
            },
            "pass_threshold": 0.75,
            "hard_fail_rules": [],
        },
        "allowed_uncertainty": {
            "can_answer_unknown": False,
            "acceptable_phrases": ["tiny generated corpus", "derived graph/report adapter"],
            "fallback_action": "state_blocker",
        },
        "operator_debug": None,
        "encoding": {},
        "memory_evolution": None,
        "tags": ["external_adapter", "generated_public", "graphify", "no_live_claim"],
    }

    if status.result in {"blocked", "incomplete"}:
        fixture["encoding"] = {
            "status": status.result,
            "reason": status.failure_reason,
        }

    write_json(fixture_path, fixture)

    return fixture_path


def write_materialization(
    status: StatusState,
    corpus: list[CorpusItem],
    fixture_path: Path,
    corpus_csv: Path,
    command_records: list[CommandRecord],
    mappings: dict[str, Any],
    started_at: float,
    report: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Write the primary smoke artifact."""

    elapsed_ms = (time.monotonic() - started_at) * 1000
    graph_json = OUTPUT_CAPTURE_DIR / "graph.json"
    graph_report = OUTPUT_CAPTURE_DIR / "GRAPH_REPORT.md"
    cache_dir = OUTPUT_CAPTURE_DIR / "cache"
    query_record = next((record for record in command_records if record.label == "graphify-query"), None)
    payload = {
        "schema": "elf.graphify_docker_graph_report_smoke/v1",
        "generated_at": utc_now(),
        "run_id": RUN_ID,
        "adapter_id": "graphify_docker_smoke",
        "evidence_class": status.evidence_class,
        "status": {
            "source": "smoke_materialization",
            "setup": status.setup,
            "run": status.run,
            "result": status.result,
            "overall": status.overall,
            "failure_class": status.failure_class,
            "failure_reason": status.failure_reason,
        },
        "scored_benchmark": scored_benchmark(report),
        "artifacts": {
            "generated_corpus_csv": rel(corpus_csv),
            "generated_corpus_dir": rel(CORPUS_DIR),
            "generated_fixture": rel(fixture_path),
            "graph_output_dir": rel(OUTPUT_CAPTURE_DIR),
            "graph_json": rel(graph_json) if graph_json.exists() else None,
            "graph_report": rel(graph_report) if graph_report.exists() else None,
            "query_output": query_record.stdout_artifact if query_record else None,
            "manifest": rel(MANIFEST_OUT),
            "summary": rel(SUMMARY_OUT),
            "scored_report_json": rel(REPORT_JSON),
            "scored_report_markdown": rel(REPORT_MD),
        },
        "docker_boundary": {
            "compose_file": "docker-compose.baseline.yml",
            "runner_service": "baseline-runner",
            "runner": "scripts/graphify-docker-graph-report-smoke.py",
            "host_global_installs_required": False,
            "docker_only": True,
            "assistant_hook_install_used": False,
            "isolated_home": True,
        },
        "model_provider_boundary": {
            "package": GRAPHIFY_REF,
            "package_spec": GRAPHIFY_PACKAGE,
            "assistant_platform_hooks_used": False,
            "host_global_assistant_config_used": False,
            "operator_owned_provider_credentials_used": False,
            "provider_or_model_name": "graphify CLI default; no model configured by this runner",
            "live_run_enabled": RUN_GRAPHIFY,
        },
        "resource_bounds": {
            "generated_file_count": len(corpus),
            "generated_input_chars": sum(len(item.text) for item in corpus),
            "timeout_seconds": TIMEOUT_SECONDS,
            "elapsed_ms": round(elapsed_ms, 3),
            "graph_json_size_bytes": graph_json.stat().st_size if graph_json.exists() else 0,
            "graph_report_size_bytes": graph_report.stat().st_size if graph_report.exists() else 0,
            "graph_output_size_bytes": dir_size(OUTPUT_CAPTURE_DIR),
            "cache_size_bytes": dir_size(cache_dir),
            "cache_file_count": file_count(cache_dir),
        },
        "retry_behavior": {
            "max_attempts": 1,
            "retries_performed": 0,
            "retry_guidance": "Rerun the same Docker command after setup/runtime fixes; do not use host assistant hooks as proof.",
        },
        "commands": [command_to_json(record) for record in command_records],
        "evidence_mapping": mappings,
    }
    write_json(OUT, payload)

    return payload


def write_manifest(status: StatusState) -> dict[str, Any]:
    """Write a generated external adapter manifest for this smoke."""

    manifest = {
        "schema": "elf.real_world_external_adapter_manifest/v1",
        "manifest_id": f"graphify-docker-smoke-{RUN_ID}",
        "docker_isolation": {
            "default": True,
            "compose_file": "docker-compose.baseline.yml",
            "runner": "scripts/graphify-docker-graph-report-smoke.py",
            "artifact_dir": "tmp/real-world-memory/graphify-smoke",
            "host_global_installs_required": False,
            "notes": [
                f"Generated by the graphify Docker graph/report smoke at {utc_now()}.",
                "The smoke uses generated public source files and records typed setup/runtime failures.",
            ],
        },
        "adapters": [
            {
                "adapter_id": "graphify_docker_smoke",
                "project": "graphify",
                "adapter_kind": "docker_cli_graph_report_smoke",
                "evidence_class": status.evidence_class,
                "docker_default": True,
                "host_global_installs_required": False,
                "overall_status": status.overall,
                "setup": {
                    "status": status.setup,
                    "evidence": "The smoke installs graphify in a container-local Python venv and runs with isolated assistant config paths.",
                    "command": "cargo make graphify-docker-graph-report-smoke",
                    "artifact": rel(OUT),
                },
                "run": {
                    "status": status.run,
                    "evidence": "The live path builds graphify graph/report artifacts from a generated public corpus and runs graphify query over graph.json.",
                    "command": "cargo make graphify-docker-graph-report-smoke",
                    "artifact": rel(OUT),
                },
                "result": {
                    "status": status.result,
                    "evidence": status.failure_reason
                    if status.failure_reason
                    else "graphify graph.json, GRAPH_REPORT.md, and query output mapped to generated real_world_job evidence ids.",
                    "artifact": rel(OUT),
                },
                "capabilities": [
                    {
                        "capability": "docker_cli_boundary",
                        "status": status.setup,
                        "evidence": "The runner uses docker-compose.baseline.yml baseline-runner and does not install graphify or assistant hooks on the host.",
                    },
                    {
                        "capability": "graph_report_generation",
                        "status": status.run,
                        "evidence": "The smoke captures graphify-out/graph.json, GRAPH_REPORT.md, cache metadata, and command logs when build succeeds.",
                    },
                    {
                        "capability": "graph_query_evidence_mapping",
                        "status": status.result,
                        "evidence": "Node labels, edge types, confidence tags, source files, source locations, report text, and query output are scanned for generated evidence ids.",
                    },
                    {
                        "capability": "quality_or_scale_claim",
                        "status": "not_encoded",
                        "evidence": "The smoke does not claim multimodal, private corpus, broad codebase-understanding, or large-corpus graph quality.",
                    },
                ],
                "suites": [
                    {
                        "suite_id": "knowledge_compilation",
                        "status": status.result,
                        "evidence": "Only the generated graph/report evidence-mapping job is represented.",
                    },
                    {
                        "suite_id": "retrieval",
                        "status": status.result if status.result in {"pass", "wrong_result"} else status.run,
                        "evidence": "The smoke uses graphify query output only to support source mapping; broad retrieval quality is not scored.",
                    },
                    {
                        "suite_id": "work_resume",
                        "status": "not_encoded",
                        "evidence": "Resume-answer behavior is not encoded by this graph/report smoke.",
                    },
                    {
                        "suite_id": "production_ops",
                        "status": "not_encoded",
                        "evidence": "The smoke records resource bounds but does not encode backup, restore, provider credential, or private corpus operations.",
                    },
                ],
                "evidence": [
                    {"kind": "artifact", "ref": rel(OUT), "status": status.result},
                    {"kind": "artifact", "ref": rel(OUTPUT_CAPTURE_DIR), "status": status.result},
                    {"kind": "manifest", "ref": rel(MANIFEST_OUT), "status": status.overall},
                    {"kind": "source", "ref": "https://github.com/safishamsi/graphify", "status": "real"},
                    {
                        "kind": "source",
                        "ref": "https://github.com/safishamsi/graphify/blob/v3/README.md",
                        "status": "real",
                    },
                ],
                "execution_metadata": {
                    "sources": [
                        {
                            "label": "graphify repository",
                            "url": "https://github.com/safishamsi/graphify",
                            "evidence": "Official source for graphify graph extraction and query workflow.",
                        },
                        {
                            "label": "graphify README",
                            "url": "https://github.com/safishamsi/graphify/blob/v3/README.md",
                            "evidence": "Official CLI, output artifact, query, confidence, and source-location contract.",
                        },
                        {
                            "label": "graphify PyPI package",
                            "url": "https://pypi.org/project/graphifyy/",
                            "evidence": "Official package referenced by the graphify README.",
                        },
                    ],
                    "setup_path": "Run cargo make graphify-docker-graph-report-smoke to install graphify in a container-local venv and build graph/report artifacts over generated public files.",
                    "runtime_boundary": "docker-compose.baseline.yml baseline-runner, isolated HOME/config paths, generated corpus, and artifacts under tmp/real-world-memory/graphify-smoke.",
                    "resource_expectation": f"graphify package {GRAPHIFY_REF}, generated_files=4, timeout_seconds={TIMEOUT_SECONDS}, query_budget={QUERY_BUDGET}.",
                    "retry_guidance": [
                        "Rerun cargo make graphify-docker-graph-report-smoke after dependency or runtime fixes.",
                        "Do not use graphify install hooks, host-global Codex/Claude/Gemini config, or private corpora as proof.",
                        "Score only when graph.json, GRAPH_REPORT.md, and graphify query output map to generated evidence ids.",
                    ],
                    "research_depth": "D1 feasibility plus XY-889 Docker graph/report smoke implementation; generated artifact decides live evidence class.",
                },
                "notes": [
                    "The checked-in manifest record remains research_gate; generated smoke artifacts carry live status.",
                    "graphify output is treated as a derived graph/report adapter, not an authoritative ELF memory store.",
                ],
            }
        ],
    }
    write_json(MANIFEST_OUT, manifest)

    return manifest


def write_summary(materialization: dict[str, Any], manifest: dict[str, Any], report: dict[str, Any]) -> None:
    """Write a small summary artifact."""

    write_json(
        SUMMARY_OUT,
        {
            "schema": "elf.graphify_docker_smoke_summary/v1",
            "generated_at": utc_now(),
            "adapter_id": "graphify_docker_smoke",
            "evidence_class": materialization["evidence_class"],
            "status_boundary": {
                "materialization": "setup/run/evidence-mapping state emitted by the smoke runner",
                "manifest": "external adapter declaration consumed by the scorer",
                "scored_benchmark": "post-score real_world_job outcome; use this for quality status",
            },
            "scored_benchmark": materialization["scored_benchmark"],
            "materialization": materialization,
            "manifest": {
                "json": rel(MANIFEST_OUT),
                "status_source": "external_adapter_manifest_pre_score",
                "summary": manifest["adapters"][0]["overall_status"],
                "suites": manifest["adapters"][0]["suites"],
            },
            "report": report,
        },
    )


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


def main() -> int:
    """Run the smoke and always emit typed artifacts when possible."""

    started_at = time.monotonic()
    mkdirs()
    status = StatusState()
    command_records: list[CommandRecord] = []
    corpus = generated_corpus()
    corpus_csv = write_corpus(corpus)
    mappings = {
        "expected_evidence_ids": expected_ids(corpus),
        "mapped_evidence_ids": [],
        "graph_json": {"artifact": None, "exists": False, "size_bytes": 0},
        "graph_report": {
            "kind": "graph_report",
            "artifact": None,
            "exists": False,
            "size_bytes": 0,
            "evidence_ids": [],
        },
        "query_output": {
            "kind": "query_output",
            "artifact": None,
            "exists": False,
            "command_status": "not_encoded",
            "evidence_ids": [],
        },
        "nodes": [],
        "edges": [],
    }

    if not Path("/.dockerenv").exists() and not ALLOW_HOST:
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "not_running_in_docker"
        status.failure_reason = "graphify smoke must run inside Docker; use cargo make graphify-docker-graph-report-smoke."
    elif not command_available("python3"):
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "python_missing"
        status.failure_reason = "python3 is required for the graphify smoke runner."
    elif not RUN_GRAPHIFY:
        pass
    else:
        graphify = install_graphify(command_records)

        if graphify is None:
            status.setup = "incomplete"
            status.result = "incomplete"
            status.overall = "incomplete"
            status.failure_class = "graphify_setup_failed"
            status.failure_reason = "graphify installation or help command failed inside the Docker runner."
        else:
            status.setup = "pass"
            output_dir = run_graphify(graphify, command_records)

            if output_dir is None:
                status.run = "incomplete"
                status.result = "incomplete"
                status.overall = "incomplete"
                status.failure_class = "graphify_build_failed"
                status.failure_reason = "graphify did not build graph/report artifacts for the generated corpus."
            else:
                status.run = "pass"
                status.evidence_class = "live_real_world"
                mappings = map_artifacts(corpus, command_records)
                result_status, reason = mapping_outcome(mappings, command_records)
                status.result = result_status
                status.overall = result_status

                if result_status == "pass":
                    status.failure_class = ""
                    status.failure_reason = ""
                else:
                    status.failure_class = "graphify_evidence_mapping_failed"
                    status.failure_reason = reason

    fixture_path = write_fixture(corpus, status, mappings["mapped_evidence_ids"])
    materialization = write_materialization(
        status,
        corpus,
        fixture_path,
        corpus_csv,
        command_records,
        mappings,
        started_at,
    )
    manifest = write_manifest(status)
    report = run_scored_report(fixture_path, MANIFEST_OUT, status)
    materialization = write_materialization(
        status,
        corpus,
        fixture_path,
        corpus_csv,
        command_records,
        mappings,
        started_at,
        report,
    )
    write_summary(materialization, manifest, report)
    print(f"graphify smoke artifact: {OUT}")
    print(f"graphify smoke manifest: {MANIFEST_OUT}")
    print(f"graphify smoke summary: {SUMMARY_OUT}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
