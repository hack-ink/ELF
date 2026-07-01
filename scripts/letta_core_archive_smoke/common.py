"""Shared filesystem and process helpers for the Letta smoke."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from .context import FIXTURE_DIR, LOG_DIR, MANIFEST_OUT, OUT, REPORT_DIR, REPORT_JSON, REPORT_MD, ROOT_DIR, SUMMARY_OUT, TIMEOUT_SECONDS, WORK_DIR
from .models import CommandRecord

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

    for path in (FIXTURE_DIR, LOG_DIR):
        if path.exists():
            shutil.rmtree(path)

    for path in (REPORT_DIR, WORK_DIR, FIXTURE_DIR, LOG_DIR):
        path.mkdir(parents=True, exist_ok=True)

    for path in (OUT, MANIFEST_OUT, SUMMARY_OUT, REPORT_JSON, REPORT_MD):
        if path.exists():
            path.unlink()

def write_json(path: Path, payload: Any) -> None:
    """Write stable, pretty JSON."""

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")

def command_available(name: str) -> bool:
    """Return whether a command is available."""

    return shutil.which(name) is not None

def run_command(
    label: str,
    command: list[str],
    cwd: Path,
    *,
    extra_env: dict[str, str] | None = None,
) -> CommandRecord:
    """Run a command and capture stdout/stderr artifacts."""

    started = time.monotonic()
    env = os.environ.copy()
    if extra_env:
        env.update(extra_env)

    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            text=True,
            capture_output=True,
            timeout=TIMEOUT_SECONDS,
            check=False,
        )
        elapsed = (time.monotonic() - started) * 1000
        stdout_path = LOG_DIR / f"{label}.stdout.txt"
        stderr_path = LOG_DIR / f"{label}.stderr.txt"
        stdout_path.write_text(result.stdout, encoding="utf-8")
        stderr_path.write_text(result.stderr, encoding="utf-8")
        status = "pass" if result.returncode == 0 else "incomplete"
        reason = "command completed" if result.returncode == 0 else f"exit code {result.returncode}"

        return CommandRecord(
            label=label,
            command=command,
            status=status,
            elapsed_ms=elapsed,
            stdout_artifact=rel(stdout_path),
            stderr_artifact=rel(stderr_path),
            returncode=result.returncode,
            reason=reason,
        )
    except subprocess.TimeoutExpired as exc:
        elapsed = (time.monotonic() - started) * 1000
        stdout_path = LOG_DIR / f"{label}.stdout.txt"
        stderr_path = LOG_DIR / f"{label}.stderr.txt"
        stdout_path.write_text(exc.stdout or "", encoding="utf-8")
        stderr_path.write_text(exc.stderr or "", encoding="utf-8")

        return CommandRecord(
            label=label,
            command=command,
            status="incomplete",
            elapsed_ms=elapsed,
            stdout_artifact=rel(stdout_path),
            stderr_artifact=rel(stderr_path),
            returncode=None,
            reason=f"timed out after {TIMEOUT_SECONDS}s",
        )

def command_to_json(record: CommandRecord) -> dict[str, Any]:
    """Serialize a command record."""

    return {
        "label": record.label,
        "command": record.command,
        "status": record.status,
        "elapsed_ms": round(record.elapsed_ms, 3),
        "stdout_artifact": record.stdout_artifact,
        "stderr_artifact": record.stderr_artifact,
        "returncode": record.returncode,
        "reason": record.reason,
    }
