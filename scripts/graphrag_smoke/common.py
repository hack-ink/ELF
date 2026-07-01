from __future__ import annotations

import json
import shutil
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from .context import FIXTURE_DIR, LOG_DIR, MANIFEST_OUT, OUT, OUTPUT_CAPTURE_DIR, REPORT_DIR, REPORT_JSON, REPORT_MD, ROOT_DIR, SUMMARY_OUT, WORK_DIR



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
