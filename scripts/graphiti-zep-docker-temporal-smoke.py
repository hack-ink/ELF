#!/usr/bin/env python3
"""Docker-contained Graphiti/Zep temporal fact smoke for real-world adapters."""

from __future__ import annotations

import json
import os
import shutil
import socket
import subprocess
import sys
import textwrap
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent
REPORT_DIR = Path(
    os.environ.get(
        "ELF_GRAPHITI_ZEP_SMOKE_REPORT_DIR",
        ROOT_DIR / "tmp" / "real-world-memory" / "graphiti-zep-smoke",
    )
)
WORK_DIR = Path(os.environ.get("ELF_GRAPHITI_ZEP_SMOKE_WORK_DIR", REPORT_DIR / "work"))
OUT = Path(os.environ.get("ELF_GRAPHITI_ZEP_SMOKE_OUT", REPORT_DIR / "graphiti-zep-smoke.json"))
MANIFEST_OUT = Path(
    os.environ.get(
        "ELF_GRAPHITI_ZEP_SMOKE_MANIFEST_OUT",
        REPORT_DIR / "memory_projects_manifest.graphiti-zep-smoke.json",
    )
)
SUMMARY_OUT = Path(os.environ.get("ELF_GRAPHITI_ZEP_SMOKE_SUMMARY_OUT", REPORT_DIR / "summary.json"))
REPORT_JSON = Path(
    os.environ.get("ELF_GRAPHITI_ZEP_SMOKE_REPORT_JSON", REPORT_DIR / "graphiti-zep-report.json")
)
REPORT_MD = Path(
    os.environ.get("ELF_GRAPHITI_ZEP_SMOKE_REPORT_MD", REPORT_DIR / "graphiti-zep-report.md")
)
FIXTURE_DIR = REPORT_DIR / "graphiti-zep-fixtures"
LOG_DIR = REPORT_DIR / "logs"

RUN_ID = os.environ.get(
    "ELF_GRAPHITI_ZEP_SMOKE_RUN_ID",
    f"graphiti-zep-docker-smoke-{datetime.now(timezone.utc).strftime('%Y%m%d%H%M%S')}",
)
RUN_LIVE = os.environ.get("ELF_GRAPHITI_ZEP_SMOKE_RUN", "0") == "1"
ALLOW_HOST = os.environ.get("ELF_GRAPHITI_ZEP_SMOKE_ALLOW_HOST", "0") == "1"
INSTALL_GRAPHITI = os.environ.get("ELF_GRAPHITI_ZEP_SMOKE_INSTALL", "1") == "1"
GRAPHITI_VERSION = os.environ.get("ELF_GRAPHITI_ZEP_VERSION", "0.21.0")
GRAPHITI_PACKAGE = os.environ.get(
    "ELF_GRAPHITI_ZEP_PACKAGE",
    f"graphiti-core[falkordb]=={GRAPHITI_VERSION}",
)
GRAPHITI_REF = os.environ.get("ELF_GRAPHITI_ZEP_REF", f"pypi:{GRAPHITI_PACKAGE}")
FALKORDB_HOST = os.environ.get("ELF_GRAPHITI_ZEP_FALKORDB_HOST", "graphiti-falkordb")
FALKORDB_PORT = int(os.environ.get("ELF_GRAPHITI_ZEP_FALKORDB_PORT", "6379"))
FALKORDB_DATABASE = os.environ.get("ELF_GRAPHITI_ZEP_FALKORDB_DATABASE", "elf_graphiti_zep_smoke")
FALKORDB_USERNAME = os.environ.get("ELF_GRAPHITI_ZEP_FALKORDB_USERNAME", "")
FALKORDB_PASSWORD = os.environ.get("ELF_GRAPHITI_ZEP_FALKORDB_PASSWORD", "")
API_KEY = os.environ.get(
    "ELF_GRAPHITI_ZEP_API_KEY",
    os.environ.get("GRAPHITI_OPENAI_API_KEY", os.environ.get("OPENAI_API_KEY", "")),
)
API_BASE = os.environ.get("ELF_GRAPHITI_ZEP_API_BASE", os.environ.get("OPENAI_BASE_URL", ""))
LLM_MODEL = os.environ.get("ELF_GRAPHITI_ZEP_LLM_MODEL", "gpt-4o-mini")
EMBEDDING_MODEL = os.environ.get("ELF_GRAPHITI_ZEP_EMBEDDING_MODEL", "text-embedding-3-small")
TIMEOUT_SECONDS = int(os.environ.get("ELF_GRAPHITI_ZEP_TIMEOUT_SECONDS", "900"))
STARTUP_ATTEMPTS = int(os.environ.get("ELF_GRAPHITI_ZEP_STARTUP_ATTEMPTS", "30"))
STARTUP_INTERVAL_SECONDS = float(os.environ.get("ELF_GRAPHITI_ZEP_STARTUP_INTERVAL_SECONDS", "2"))


@dataclass
class StatusState:
    """Typed status for generated Graphiti/Zep smoke artifacts."""

    setup: str = "blocked"
    run: str = "not_encoded"
    result: str = "blocked"
    overall: str = "blocked"
    evidence_class: str = "research_gate"
    failure_class: str = "graphiti_zep_live_run_disabled"
    failure_reason: str = (
        "Graphiti/Zep temporal graph live run is opt-in; set "
        "ELF_GRAPHITI_ZEP_SMOKE_START=1 ELF_GRAPHITI_ZEP_SMOKE_RUN=1 and provide explicit "
        "provider configuration to attempt the Docker-local FalkorDB smoke."
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

    for path in (REPORT_DIR, WORK_DIR, FIXTURE_DIR, LOG_DIR):
        path.mkdir(parents=True, exist_ok=True)


def write_json(path: Path, payload: Any) -> None:
    """Write stable, pretty JSON."""

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def run_scored_report(fixture_path: Path, manifest_path: Path, status: StatusState) -> dict[str, Any]:
    """Score the generated temporal smoke fixture through the real-world job runner."""

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
        "real-world-memory-live-graphiti-zep",
        "--adapter-id",
        "graphiti_zep_temporal_smoke",
        "--adapter-name",
        "Graphiti/Zep Docker temporal smoke adapter",
        "--adapter-behavior",
        "docker_python_falkordb_temporal_smoke",
        "--adapter-storage-status",
        status.setup,
        "--adapter-runtime-status",
        status.overall,
        "--adapter-notes",
        "Generated by the Graphiti/Zep Docker temporal smoke; pass or wrong_result requires current and historical validity-window facts mapped to generated evidence ids, while provider/setup limits remain typed.",
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


def command_available(command: str) -> bool:
    """Return whether a command is on PATH."""

    return shutil.which(command) is not None


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


def temporal_facts() -> list[dict[str, Any]]:
    """Return the generated-public temporal fact corpus."""

    return [
        {
            "evidence_id": "graphiti-zep-old-owner",
            "claim_id": "relation_historical_owner",
            "source": "Team Delta",
            "edge_name": "OWNED_REVIEW",
            "target": "deployment method review",
            "fact": "Team Delta owned deployment method review before 2026-06-06.",
            "valid_at": "2026-06-05T00:00:00Z",
            "invalid_at": "2026-06-08T00:00:00Z",
            "created_at": "2026-06-05T00:00:00Z",
            "current": False,
        },
        {
            "evidence_id": "graphiti-zep-current-owner",
            "claim_id": "relation_current_owner",
            "source": "Team Echo",
            "edge_name": "OWNS_REVIEW",
            "target": "deployment method review",
            "fact": "Team Echo owns deployment method review since 2026-06-08.",
            "valid_at": "2026-06-08T00:00:00Z",
            "invalid_at": None,
            "created_at": "2026-06-08T00:00:00Z",
            "current": True,
        },
        {
            "evidence_id": "graphiti-zep-owner-rationale",
            "claim_id": "relation_owner_update_rationale",
            "source": "single-user production runbook scope",
            "edge_name": "MOVED_OWNERSHIP_TO",
            "target": "Team Echo",
            "fact": "Ownership moved to Team Echo after single-user production runbook scope changed.",
            "valid_at": "2026-06-08T00:05:00Z",
            "invalid_at": None,
            "created_at": "2026-06-08T00:05:00Z",
            "current": True,
        },
    ]


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


def wait_for_falkordb(command_records: list[CommandRecord]) -> bool:
    """Poll the configured FalkorDB TCP endpoint."""

    started = time.monotonic()
    attempts: list[dict[str, Any]] = []

    for attempt in range(1, STARTUP_ATTEMPTS + 1):
        try:
            with socket.create_connection((FALKORDB_HOST, FALKORDB_PORT), timeout=2):
                elapsed_ms = (time.monotonic() - started) * 1000
                attempts.append({"attempt": attempt, "status": "pass", "elapsed_ms": round(elapsed_ms, 3)})
                path = LOG_DIR / "falkordb-startup-attempts.json"
                write_json(path, attempts)
                command_records.append(
                    CommandRecord(
                        label="falkordb-startup",
                        command=["tcp-connect", FALKORDB_HOST, str(FALKORDB_PORT)],
                        status="pass",
                        elapsed_ms=elapsed_ms,
                        stdout_artifact=rel(path),
                        stderr_artifact=None,
                        returncode=0,
                        reason="FalkorDB TCP endpoint accepted a connection.",
                    )
                )
                return True
        except OSError as err:
            attempts.append({"attempt": attempt, "status": "incomplete", "reason": str(err)})
            time.sleep(STARTUP_INTERVAL_SECONDS)

    elapsed_ms = (time.monotonic() - started) * 1000
    path = LOG_DIR / "falkordb-startup-attempts.json"
    write_json(path, attempts)
    command_records.append(
        CommandRecord(
            label="falkordb-startup",
            command=["tcp-connect", FALKORDB_HOST, str(FALKORDB_PORT)],
            status="incomplete",
            elapsed_ms=elapsed_ms,
            stdout_artifact=rel(path),
            stderr_artifact=None,
            returncode=None,
            reason="FalkorDB TCP endpoint did not become reachable.",
        )
    )
    return False


def init_graphiti(command_records: list[CommandRecord]) -> tuple[bool, Path]:
    """Create a venv and install Graphiti with FalkorDB support."""

    venv_dir = WORK_DIR / ".venv"
    python = venv_dir / "bin" / "python"

    if INSTALL_GRAPHITI:
        venv_record = run_command("python-venv", [sys.executable, "-m", "venv", str(venv_dir)], WORK_DIR)
        command_records.append(venv_record)
        if venv_record.status != "pass":
            return False, python

        install_record = run_command(
            "graphiti-install",
            [str(python), "-m", "pip", "install", "--disable-pip-version-check", GRAPHITI_PACKAGE],
            WORK_DIR,
        )
        command_records.append(install_record)
        if install_record.status != "pass":
            return False, python
    elif not python.exists():
        command_records.append(
            CommandRecord(
                label="graphiti-install",
                command=["graphiti-core"],
                status="incomplete",
                elapsed_ms=0.0,
                stdout_artifact=None,
                stderr_artifact=None,
                returncode=None,
                reason="Graphiti install was disabled and no venv python exists.",
            )
        )
        return False, python

    return True, python


def write_live_runner(path: Path) -> None:
    """Write the isolated Graphiti execution script."""

    payload = {
        "run_id": RUN_ID,
        "facts": temporal_facts(),
        "query": "Who currently owns deployment method review, and who owned it historically?",
        "falkordb": {
            "host": FALKORDB_HOST,
            "port": FALKORDB_PORT,
            "database": FALKORDB_DATABASE,
        },
        "models": {
            "llm": LLM_MODEL,
            "embedding": EMBEDDING_MODEL,
            "api_base": API_BASE,
        },
    }
    input_path = WORK_DIR / "graphiti-live-input.json"
    output_path = WORK_DIR / "graphiti-live-output.json"
    write_json(input_path, payload)
    script = f"""
import asyncio
import json
import os
import uuid
from datetime import datetime
from pathlib import Path

from graphiti_core import Graphiti
from graphiti_core.driver.falkordb_driver import FalkorDriver
from graphiti_core.edges import EntityEdge
from graphiti_core.nodes import EntityNode


INPUT = Path({str(input_path)!r})
OUTPUT = Path({str(output_path)!r})


def parse_dt(value):
    if value is None:
        return None
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


async def main():
    data = json.loads(INPUT.read_text(encoding="utf-8"))
    config = data["falkordb"]
    driver = FalkorDriver(
        host=config["host"],
        port=config["port"],
        username=os.environ.get("ELF_GRAPHITI_ZEP_FALKORDB_USERNAME") or None,
        password=os.environ.get("ELF_GRAPHITI_ZEP_FALKORDB_PASSWORD") or None,
        database=config.get("database") or "default_db",
    )
    graphiti = Graphiti(graph_driver=driver)
    try:
        await graphiti.build_indices_and_constraints()
        inserted = []
        for fact in data["facts"]:
            group_id = data["run_id"]
            source_uuid = str(uuid.uuid5(uuid.NAMESPACE_URL, group_id + ":source:" + fact["source"]))
            target_uuid = str(uuid.uuid5(uuid.NAMESPACE_URL, group_id + ":target:" + fact["target"]))
            edge_uuid = str(uuid.uuid5(uuid.NAMESPACE_URL, group_id + ":edge:" + fact["evidence_id"]))
            source_node = EntityNode(uuid=source_uuid, name=fact["source"], group_id=group_id)
            target_node = EntityNode(uuid=target_uuid, name=fact["target"], group_id=group_id)
            edge = EntityEdge(
                uuid=edge_uuid,
                group_id=group_id,
                source_node_uuid=source_uuid,
                target_node_uuid=target_uuid,
                created_at=parse_dt(fact["created_at"]),
                name=fact["edge_name"],
                fact=fact["fact"],
                valid_at=parse_dt(fact["valid_at"]),
                invalid_at=parse_dt(fact.get("invalid_at")),
            )
            await graphiti.add_triplet(source_node, edge, target_node)
            inserted.append({{"evidence_id": fact["evidence_id"], "uuid": edge_uuid}})

        results = await graphiti.search(data["query"])
        serialized = []
        for edge in results:
            serialized.append({{
                "uuid": getattr(edge, "uuid", None),
                "name": getattr(edge, "name", None),
                "fact": getattr(edge, "fact", None),
                "valid_at": str(getattr(edge, "valid_at", "")) if getattr(edge, "valid_at", None) else None,
                "invalid_at": str(getattr(edge, "invalid_at", "")) if getattr(edge, "invalid_at", None) else None,
                "source_node_uuid": getattr(edge, "source_node_uuid", None),
                "target_node_uuid": getattr(edge, "target_node_uuid", None),
            }})

        OUTPUT.write_text(json.dumps({{"inserted": inserted, "results": serialized}}, indent=2, sort_keys=True) + "\\n", encoding="utf-8")
    finally:
        await graphiti.close()


asyncio.run(main())
"""
    path.write_text(textwrap.dedent(script).lstrip(), encoding="utf-8")


def run_graphiti(python: Path, command_records: list[CommandRecord]) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    """Run the Graphiti live worker and return inserted/search result facts."""

    runner = WORK_DIR / "graphiti_live_runner.py"
    write_live_runner(runner)
    env = {
        "OPENAI_API_KEY": API_KEY,
        "MODEL_NAME": LLM_MODEL,
        "LLM_MODEL": LLM_MODEL,
        "EMBEDDING_MODEL": EMBEDDING_MODEL,
    }

    if API_BASE:
        env["OPENAI_BASE_URL"] = API_BASE
    if FALKORDB_USERNAME:
        env["ELF_GRAPHITI_ZEP_FALKORDB_USERNAME"] = FALKORDB_USERNAME
    if FALKORDB_PASSWORD:
        env["ELF_GRAPHITI_ZEP_FALKORDB_PASSWORD"] = FALKORDB_PASSWORD

    record = run_command("graphiti-live-run", [str(python), str(runner)], WORK_DIR, extra_env=env)
    command_records.append(record)

    output_path = WORK_DIR / "graphiti-live-output.json"
    if record.status != "pass" or not output_path.exists():
        return [], []

    payload = json.loads(output_path.read_text(encoding="utf-8"))
    return payload.get("inserted", []), payload.get("results", [])


def map_observed_facts(results: list[dict[str, Any]], facts: list[dict[str, Any]]) -> dict[str, Any]:
    """Map Graphiti search results back to expected evidence ids."""

    expected_by_id = {fact["evidence_id"]: fact for fact in facts}
    mappings: list[dict[str, Any]] = []
    mapped_ids: list[str] = []

    for fact in facts:
        matched = [
            result
            for result in results
            if isinstance(result.get("fact"), str) and fact["fact"].lower() in result["fact"].lower()
        ]
        if matched:
            result = matched[0]
            mapped_ids.append(fact["evidence_id"])
            mappings.append(
                {
                    "evidence_id": fact["evidence_id"],
                    "claim_id": fact["claim_id"],
                    "status": "pass",
                    "uuid": result.get("uuid"),
                    "fact": result.get("fact"),
                    "valid_at": result.get("valid_at"),
                    "invalid_at": result.get("invalid_at"),
                    "expected_valid_at": fact["valid_at"],
                    "expected_invalid_at": fact["invalid_at"],
                    "current": fact["current"],
                }
            )
        else:
            mappings.append(
                {
                    "evidence_id": fact["evidence_id"],
                    "claim_id": fact["claim_id"],
                    "status": "blocked",
                    "expected_valid_at": fact["valid_at"],
                    "expected_invalid_at": fact["invalid_at"],
                    "current": fact["current"],
                }
            )

    current_ok = any(
        item["evidence_id"] == "graphiti-zep-current-owner"
        and item["status"] == "pass"
        and not item.get("invalid_at")
        for item in mappings
    )
    historical_ok = any(
        item["evidence_id"] == "graphiti-zep-old-owner"
        and item["status"] == "pass"
        and item.get("invalid_at")
        for item in mappings
    )
    rationale_ok = "graphiti-zep-owner-rationale" in mapped_ids
    required_ids = list(expected_by_id)
    missing_ids = [evidence_id for evidence_id in required_ids if evidence_id not in mapped_ids]

    if current_ok and historical_ok and rationale_ok:
        status = "pass"
        reason = "Graphiti/Zep search results mapped current, historical, and rationale facts with validity windows."
    else:
        status = "wrong_result"
        reason = (
            "Graphiti/Zep search results did not map all required temporal facts with expected validity "
            f"windows; missing={', '.join(missing_ids) or 'none'}."
        )

    return {
        "status": status,
        "reason": reason,
        "expected_evidence_ids": required_ids,
        "mapped_evidence_ids": mapped_ids,
        "facts": mappings,
    }


def write_fixture(facts: list[dict[str, Any]], status: StatusState, mapping: dict[str, Any]) -> Path:
    """Write a generated memory_evolution fixture for the smoke."""

    fixture_path = FIXTURE_DIR / "memory_evolution" / "graphiti_zep_temporal_validity.json"
    mapped_ids = mapping.get("mapped_evidence_ids", [])
    claims = []

    if status.result == "pass":
        claims = [
            {
                "claim_id": "relation_current_owner",
                "text": "Team Echo currently owns deployment method review.",
                "evidence_ids": [
                    "graphiti-zep-current-owner",
                    "graphiti-zep-old-owner",
                    "graphiti-zep-owner-rationale",
                ],
                "confidence": "derived_from_graphiti_temporal_search",
            },
            {
                "claim_id": "relation_historical_owner",
                "text": "Team Delta owned deployment method review historically.",
                "evidence_ids": ["graphiti-zep-old-owner"],
                "confidence": "derived_from_graphiti_temporal_search",
            },
            {
                "claim_id": "relation_owner_update_rationale",
                "text": "Ownership moved after single-user production runbook scope changed.",
                "evidence_ids": ["graphiti-zep-owner-rationale"],
                "confidence": "derived_from_graphiti_temporal_search",
            },
        ]

    fixture: dict[str, Any] = {
        "schema": "elf.real_world_job/v1",
        "job_id": "graphiti-zep-temporal-validity-001",
        "suite": "memory_evolution",
        "title": "Map Graphiti/Zep temporal validity windows to current and historical relation facts",
        "corpus": {
            "corpus_id": "graphiti-zep-generated-public-smoke",
            "profile": "generated_public",
            "items": [
                {
                    "evidence_id": fact["evidence_id"],
                    "kind": "temporal_fact",
                    "text": fact["fact"],
                    "source_ref": {
                        "schema": "source_ref/v1",
                        "resolver": "graphiti_zep_smoke/v1",
                        "ref": {
                            "run_id": RUN_ID,
                            "evidence_id": fact["evidence_id"],
                            "valid_at": fact["valid_at"],
                            "invalid_at": fact["invalid_at"],
                        },
                    },
                    "created_at": fact["created_at"],
                }
                for fact in facts
            ],
            "adapter_response": {
                "adapter_id": "graphiti_zep_temporal_smoke",
                "answer": {
                    "content": (
                        "Team Echo currently owns deployment method review. Team Delta owned it "
                        "historically, and the move followed the single-user production runbook scope change."
                        if claims
                        else ""
                    ),
                    "claims": claims,
                    "evidence_ids": mapped_ids,
                    "latency_ms": 0.0,
                    "cost": {
                        "currency": "USD",
                        "amount": 0.0,
                        "input_tokens": 0,
                        "output_tokens": 0,
                    },
                },
            },
        },
        "timeline": [
            {
                "event_id": "graphiti-zep-old-owner",
                "ts": "2026-06-05T00:00:00Z",
                "actor": "agent",
                "action": "recorded_relation",
                "evidence_ids": ["graphiti-zep-old-owner"],
                "summary": "Team Delta was the historical owner.",
            },
            {
                "event_id": "graphiti-zep-current-owner",
                "ts": "2026-06-08T00:00:00Z",
                "actor": "agent",
                "action": "updated_memory",
                "evidence_ids": ["graphiti-zep-current-owner", "graphiti-zep-owner-rationale"],
                "summary": "Team Echo became the current owner after the scope changed.",
            },
        ],
        "prompt": {
            "role": "user",
            "content": "Who currently owns deployment method review, and who owned it historically?",
            "job_mode": "answer",
            "constraints": ["cite_evidence", "distinguish_current_from_historical"],
        },
        "expected_answer": {
            "must_include": [
                {
                    "claim_id": "relation_current_owner",
                    "text": "Team Echo currently owns deployment method review.",
                },
                {
                    "claim_id": "relation_historical_owner",
                    "text": "Team Delta owned deployment method review historically.",
                },
            ],
            "must_not_include": ["Team Delta currently owns deployment method review."],
            "evidence_links": {
                "relation_current_owner": [
                    "graphiti-zep-current-owner",
                    "graphiti-zep-old-owner",
                    "graphiti-zep-owner-rationale",
                ],
                "relation_historical_owner": ["graphiti-zep-old-owner"],
                "relation_owner_update_rationale": ["graphiti-zep-owner-rationale"],
            },
            "answer_type": "direct_answer",
            "accepted_alternates": [],
            "requires_caveat": False,
            "requires_refusal": False,
        },
        "required_evidence": [
            {
                "evidence_id": "graphiti-zep-current-owner",
                "claim_id": "relation_current_owner",
                "requirement": "cite",
                "quote": "Team Echo owns deployment method review",
            },
            {
                "evidence_id": "graphiti-zep-old-owner",
                "claim_id": "relation_historical_owner",
                "requirement": "cite",
                "quote": "Team Delta owned deployment method review",
            },
        ],
        "negative_traps": [
            {
                "trap_id": "old-owner-as-current",
                "type": "stale_fact",
                "evidence_ids": ["graphiti-zep-old-owner"],
                "failure_if_used": False,
            }
        ],
        "scoring_rubric": {
            "dimensions": {
                "lifecycle_behavior": {
                    "weight": 0.4,
                    "max_points": 1.0,
                    "criteria": "Requires current-only versus historical temporal validity for relation facts.",
                },
                "answer_correctness": {
                    "weight": 0.25,
                    "max_points": 1.0,
                    "criteria": "Would identify current and historical owners separately.",
                },
                "evidence_grounding": {
                    "weight": 0.2,
                    "max_points": 1.0,
                    "criteria": "Would cite both current and historical relation evidence.",
                },
                "trap_avoidance": {
                    "weight": 0.15,
                    "max_points": 1.0,
                    "criteria": "Would not report the historical owner as current.",
                },
            },
            "pass_threshold": 0.8,
            "hard_fail_rules": [],
        },
        "allowed_uncertainty": {
            "can_answer_unknown": False,
            "acceptable_phrases": ["Graphiti/Zep smoke did not return temporal facts."],
            "fallback_action": "score_temporal_relation_behavior",
        },
        "memory_evolution": {
            "current_evidence_ids": ["graphiti-zep-current-owner"],
            "historical_evidence_ids": ["graphiti-zep-old-owner"],
            "stale_trap_ids": ["old-owner-as-current"],
            "conflicts": [
                {
                    "conflict_id": "relation-owner-current-historical",
                    "claim_id": "relation_current_owner",
                    "current_evidence_id": "graphiti-zep-current-owner",
                    "historical_evidence_id": "graphiti-zep-old-owner",
                    "resolved_by_evidence_id": "graphiti-zep-owner-rationale",
                }
            ],
            "update_rationale": {
                "claim_id": "relation_owner_update_rationale",
                "evidence_ids": ["graphiti-zep-owner-rationale"],
                "available": True,
            },
            "temporal_validity": {"required": True, "encoded": True},
        },
        "tags": ["external_adapter", "generated_public", "memory_evolution", "reference_graphiti_zep_temporal"],
    }

    if status.result in {"blocked", "incomplete", "not_encoded"}:
        fixture["encoding"] = {"status": status.result, "reason": status.failure_reason}

    write_json(fixture_path, fixture)

    return fixture_path


def write_materialization(
    status: StatusState,
    facts: list[dict[str, Any]],
    fixture_path: Path,
    command_records: list[CommandRecord],
    inserted: list[dict[str, Any]],
    search_results: list[dict[str, Any]],
    mapping: dict[str, Any],
    started_at: float,
    report: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Write the primary smoke artifact."""

    elapsed_ms = (time.monotonic() - started_at) * 1000
    payload = {
        "schema": "elf.graphiti_zep_temporal_smoke/v1",
        "generated_at": utc_now(),
        "run_id": RUN_ID,
        "adapter_id": "graphiti_zep_temporal_smoke",
        "project": "Graphiti/Zep",
        "status": status.overall,
        "materialization_status": {
            "source": "smoke_materialization",
            "setup": status.setup,
            "run": status.run,
            "result": status.result,
            "overall": status.overall,
            "failure_class": status.failure_class,
            "failure_reason": status.failure_reason,
        },
        "scored_benchmark": scored_benchmark(report),
        "evidence_class": status.evidence_class,
        "failure": {
            "class": status.failure_class or None,
            "reason": status.failure_reason or None,
        },
        "artifacts": {
            "materialization": rel(OUT),
            "manifest": rel(MANIFEST_OUT),
            "summary": rel(SUMMARY_OUT),
            "fixture": rel(fixture_path),
            "scored_report_json": rel(REPORT_JSON),
            "scored_report_markdown": rel(REPORT_MD),
        },
        "docker_boundary": {
            "compose_file": "docker-compose.baseline.yml",
            "service_profile": "graphiti-zep",
            "graph_store_service": "graphiti-falkordb",
            "runner_service": "baseline-runner",
            "runner": "scripts/graphiti-zep-docker-temporal-smoke.py",
            "host_global_installs_required": False,
            "docker_only": True,
        },
        "provider_configuration": {
            "package": GRAPHITI_REF,
            "package_spec": GRAPHITI_PACKAGE,
            "llm_model": LLM_MODEL,
            "embedding_model": EMBEDDING_MODEL,
            "api_base_configured": bool(API_BASE),
            "api_key_provided": bool(API_KEY),
            "operator_owned_provider_credentials_used": False,
            "live_run_enabled": RUN_LIVE,
            "falkordb": {
                "host": FALKORDB_HOST,
                "port": FALKORDB_PORT,
                "database": FALKORDB_DATABASE,
                "username_configured": bool(FALKORDB_USERNAME),
                "password_configured": bool(FALKORDB_PASSWORD),
            },
        },
        "resource_bounds": {
            "fact_count": len(facts),
            "timeout_seconds": TIMEOUT_SECONDS,
            "elapsed_ms": round(elapsed_ms, 3),
            "work_dir_size_bytes": dir_size(WORK_DIR),
            "work_dir_file_count": file_count(WORK_DIR),
        },
        "commands": [command_to_json(record) for record in command_records],
        "temporal_facts": facts,
        "inserted_facts": inserted,
        "search_results": search_results,
        "evidence_mapping": mapping,
    }
    write_json(OUT, payload)

    return payload


def write_manifest(status: StatusState) -> dict[str, Any]:
    """Write a generated external adapter manifest for this smoke."""

    manifest = {
        "schema": "elf.real_world_external_adapter_manifest/v1",
        "manifest_id": f"graphiti-zep-temporal-smoke-{RUN_ID}",
        "docker_isolation": {
            "default": True,
            "compose_file": "docker-compose.baseline.yml",
            "runner": "scripts/graphiti-zep-docker-temporal-smoke.py",
            "artifact_dir": "tmp/real-world-memory/graphiti-zep-smoke",
            "host_global_installs_required": False,
            "notes": [
                f"Generated by the Graphiti/Zep Docker smoke at {utc_now()}.",
                "The smoke uses generated public temporal facts and records typed setup/runtime failures.",
            ],
        },
        "adapters": [
            {
                "adapter_id": "graphiti_zep_temporal_smoke",
                "project": "Graphiti/Zep",
                "adapter_kind": "docker_python_falkordb_temporal_smoke",
                "evidence_class": status.evidence_class,
                "docker_default": True,
                "host_global_installs_required": False,
                "overall_status": status.overall,
                "setup": {
                    "status": status.setup,
                    "evidence": "The smoke runs inside the baseline Docker runner and uses Docker-local FalkorDB plus a container-local Python venv.",
                    "command": "cargo make smoke-graphiti-zep-docker-temporal",
                    "artifact": rel(OUT),
                },
                "run": {
                    "status": status.run,
                    "evidence": "The live path adds generated temporal fact triples and searches Graphiti/Zep for UUID, fact, valid_at, invalid_at, and source node evidence.",
                    "command": "ELF_GRAPHITI_ZEP_SMOKE_START=1 ELF_GRAPHITI_ZEP_SMOKE_RUN=1 cargo make smoke-graphiti-zep-docker-temporal",
                    "artifact": rel(OUT),
                },
                "result": {
                    "status": status.result,
                    "evidence": status.failure_reason
                    if status.failure_reason
                    else "Graphiti/Zep temporal search mapped current and historical facts to validity windows.",
                    "artifact": rel(OUT),
                },
                "capabilities": [
                    {
                        "capability": "docker_falkordb_setup",
                        "status": status.setup,
                        "evidence": "The task starts a Docker Compose FalkorDB profile only when explicitly requested, and uses no host-global graph database.",
                    },
                    {
                        "capability": "temporal_fact_triple_ingest",
                        "status": status.run,
                        "evidence": "The live worker uses Graphiti fact triples for current, historical, and rationale facts with validity windows.",
                    },
                    {
                        "capability": "validity_window_evidence_mapping",
                        "status": status.result,
                        "evidence": "Search output UUID, fact text, valid_at, invalid_at, and node ids are mapped to memory_evolution expected evidence ids.",
                    },
                    {
                        "capability": "quality_or_scale_claim",
                        "status": "not_encoded",
                        "evidence": "The smoke does not claim broad graph-memory quality, large-corpus behavior, managed Zep service behavior, or private-corpus performance.",
                    },
                ],
                "suites": [
                    {
                        "suite_id": "memory_evolution",
                        "status": status.result,
                        "evidence": "Only generated current-versus-historical temporal relation facts are represented.",
                    },
                    {
                        "suite_id": "retrieval",
                        "status": status.run if status.run != "pass" else "not_encoded",
                        "evidence": "Hybrid retrieval reachability is exercised by the live search, but broad retrieval quality scoring is not encoded.",
                    },
                    {
                        "suite_id": "production_ops",
                        "status": "not_encoded",
                        "evidence": "The smoke records setup and provider boundaries but does not encode backup, restore, private corpus, or hosted-service operations.",
                    },
                ],
                "scenarios": [
                    {
                        "scenario_id": "temporal_validity_window_mapping",
                        "suite_id": "memory_evolution",
                        "status": status.result,
                        "elf_position": "untested",
                        "comparison_outcome": "blocked"
                        if status.result == "blocked"
                        else "not_tested",
                        "evidence": status.failure_reason
                        if status.failure_reason
                        else "Graphiti/Zep temporal search mapped generated current and historical relation facts to validity windows and evidence ids.",
                        "command": "cargo make smoke-graphiti-zep-docker-temporal",
                        "artifact": rel(OUT),
                    }
                ],
                "evidence": [
                    {"kind": "artifact", "ref": rel(OUT), "status": status.result},
                    {"kind": "manifest", "ref": rel(MANIFEST_OUT), "status": status.overall},
                    {"kind": "source", "ref": "https://github.com/getzep/graphiti", "status": "real"},
                    {
                        "kind": "source",
                        "ref": "https://help.getzep.com/graphiti/getting-started/quick-start",
                        "status": "real",
                    },
                    {
                        "kind": "source",
                        "ref": "https://help.getzep.com/graphiti/configuration/falkor-db-configuration",
                        "status": "real",
                    },
                    {
                        "kind": "source",
                        "ref": "https://help.getzep.com/graphiti/working-with-data/adding-fact-triples",
                        "status": "real",
                    },
                ],
                "execution_metadata": {
                    "sources": [
                        {
                            "label": "Graphiti repository",
                            "url": "https://github.com/getzep/graphiti",
                            "evidence": "Official source for the open-source temporal context graph engine.",
                        },
                        {
                            "label": "Graphiti quick start",
                            "url": "https://help.getzep.com/graphiti/getting-started/quick-start",
                            "evidence": "Official search output examples include UUID, fact, valid_at, and invalid_at fields.",
                        },
                        {
                            "label": "Graphiti FalkorDB configuration",
                            "url": "https://help.getzep.com/graphiti/configuration/falkor-db-configuration",
                            "evidence": "Official Docker-local FalkorDB setup and Python driver reference.",
                        },
                        {
                            "label": "Graphiti fact triples",
                            "url": "https://help.getzep.com/graphiti/working-with-data/adding-fact-triples",
                            "evidence": "Official manual fact-triple ingest contract.",
                        },
                    ],
                    "setup_path": "Run cargo make smoke-graphiti-zep-docker-temporal for a typed artifact; set ELF_GRAPHITI_ZEP_SMOKE_START=1 ELF_GRAPHITI_ZEP_SMOKE_RUN=1 with explicit provider configuration for a live attempt.",
                    "runtime_boundary": "docker-compose.baseline.yml baseline-runner plus graphiti-zep FalkorDB profile, container-local Python venv, generated public temporal facts, and report artifacts under tmp/real-world-memory/graphiti-zep-smoke.",
                    "resource_expectation": f"Graphiti package {GRAPHITI_REF}, fact_count=3, timeout_seconds={TIMEOUT_SECONDS}, FalkorDB host={FALKORDB_HOST}:{FALKORDB_PORT}.",
                    "retry_guidance": [
                        "Default command records a typed blocked artifact without model calls.",
                        "Enable the live path only with Docker-local FalkorDB and explicit provider configuration.",
                        "Treat missing validity windows or unmapped current/historical facts as wrong_result, not pass.",
                    ],
                    "research_depth": "D2 feasibility plus XY-888 Docker temporal smoke implementation; generated artifact decides live evidence class.",
                },
                "notes": [
                    "The checked-in manifest record remains research_gate; generated smoke artifacts carry live status.",
                    "Failure before Graphiti search output remains typed as blocked or incomplete.",
                    "The smoke does not use a hosted Zep service, private corpora, or unrecorded provider credentials.",
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
            "schema": "elf.graphiti_zep_temporal_smoke_summary/v1",
            "generated_at": utc_now(),
            "adapter_id": "graphiti_zep_temporal_smoke",
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


def main() -> int:
    """Run the smoke and always emit typed artifacts when possible."""

    started_at = time.monotonic()
    mkdirs()
    status = StatusState()
    command_records: list[CommandRecord] = []
    facts = temporal_facts()
    inserted: list[dict[str, Any]] = []
    search_results: list[dict[str, Any]] = []
    mapping: dict[str, Any] = {
        "status": "blocked",
        "reason": status.failure_reason,
        "expected_evidence_ids": [fact["evidence_id"] for fact in facts],
        "mapped_evidence_ids": [],
        "facts": [
            {
                "evidence_id": fact["evidence_id"],
                "claim_id": fact["claim_id"],
                "status": "blocked",
                "expected_valid_at": fact["valid_at"],
                "expected_invalid_at": fact["invalid_at"],
                "current": fact["current"],
            }
            for fact in facts
        ],
    }

    if not Path("/.dockerenv").exists() and not ALLOW_HOST:
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "not_running_in_docker"
        status.failure_reason = "Graphiti/Zep smoke must run inside Docker; use cargo make smoke-graphiti-zep-docker-temporal."
        mapping["status"] = status.result
        mapping["reason"] = status.failure_reason
    elif not command_available("python3"):
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "python_missing"
        status.failure_reason = "python3 is required for the Graphiti/Zep smoke runner."
        mapping["status"] = status.result
        mapping["reason"] = status.failure_reason
    elif not RUN_LIVE:
        pass
    elif not API_KEY:
        status.setup = "blocked"
        status.run = "not_encoded"
        status.result = "blocked"
        status.overall = "blocked"
        status.failure_class = "provider_api_key_missing"
        status.failure_reason = "Graphiti/Zep live temporal search requires an explicit provider API key; no hosted Zep service or unrecorded provider credentials were used."
        mapping["reason"] = status.failure_reason
    elif not wait_for_falkordb(command_records):
        status.setup = "incomplete"
        status.run = "not_encoded"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "falkordb_unreachable"
        status.failure_reason = "Docker-local FalkorDB did not become reachable for the Graphiti/Zep smoke."
        mapping["status"] = status.result
        mapping["reason"] = status.failure_reason
    else:
        installed, python = init_graphiti(command_records)
        if not installed:
            status.setup = "incomplete"
            status.run = "not_encoded"
            status.result = "incomplete"
            status.overall = "incomplete"
            status.failure_class = "graphiti_setup_failed"
            status.failure_reason = "Graphiti installation failed inside the Docker runner."
            mapping["status"] = status.result
            mapping["reason"] = status.failure_reason
        else:
            status.setup = "pass"
            inserted, search_results = run_graphiti(python, command_records)

            if not search_results:
                status.run = "incomplete"
                status.result = "incomplete"
                status.overall = "incomplete"
                status.failure_class = "graphiti_temporal_search_failed"
                status.failure_reason = "Graphiti/Zep did not return temporal search results for the generated fact corpus."
                mapping["status"] = status.result
                mapping["reason"] = status.failure_reason
            else:
                status.run = "pass"
                status.evidence_class = "live_real_world"
                mapping = map_observed_facts(search_results, facts)
                if mapping["status"] == "pass":
                    status.result = "pass"
                    status.overall = "pass"
                    status.failure_class = ""
                    status.failure_reason = ""
                else:
                    status.result = "wrong_result"
                    status.overall = "wrong_result"
                    status.failure_class = "graphiti_temporal_mapping_failed"
                    status.failure_reason = mapping["reason"]

    fixture_path = write_fixture(facts, status, mapping)
    materialization = write_materialization(
        status,
        facts,
        fixture_path,
        command_records,
        inserted,
        search_results,
        mapping,
        started_at,
    )
    manifest = write_manifest(status)
    report = run_scored_report(fixture_path, MANIFEST_OUT, status)
    materialization = write_materialization(
        status,
        facts,
        fixture_path,
        command_records,
        inserted,
        search_results,
        mapping,
        started_at,
        report,
    )
    write_summary(materialization, manifest, report)
    print(f"Graphiti/Zep smoke artifact: {OUT}")
    print(f"Graphiti/Zep smoke manifest: {MANIFEST_OUT}")
    print(f"Graphiti/Zep smoke summary: {SUMMARY_OUT}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
