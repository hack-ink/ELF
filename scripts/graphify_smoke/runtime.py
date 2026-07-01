from __future__ import annotations

import os
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

from .common import rel
from .context import (
    CORPUS_DIR,
    GRAPHIFY_PACKAGE,
    INSTALL_GRAPHIFY,
    LOG_DIR,
    OUTPUT_CAPTURE_DIR,
    QUERY_BUDGET,
    TIMEOUT_SECONDS,
    WORK_DIR,
)
from .models import CommandRecord



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
