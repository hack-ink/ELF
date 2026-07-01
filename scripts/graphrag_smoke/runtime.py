from __future__ import annotations

import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

from .common import rel
from .context import API_BASE, API_KEY, CHAT_MODEL, EMBEDDING_MODEL, GRAPH_RAG_PACKAGE, INDEX_METHOD, INSTALL_GRAPHRAG, LOG_DIR, QUERY_METHOD, TIMEOUT_SECONDS, WORK_DIR
from .models import CommandRecord



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
