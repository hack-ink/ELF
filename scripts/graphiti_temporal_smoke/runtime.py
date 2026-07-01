"""Runtime setup and live Graphiti execution."""

from __future__ import annotations

import json
import socket
import sys
import textwrap
import time
from pathlib import Path
from typing import Any

from .common import run_command, write_json
from .context import *  # noqa: F403
from .corpus import temporal_facts
from .models import CommandRecord

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
