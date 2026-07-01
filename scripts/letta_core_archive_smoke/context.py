"""Configuration for the Letta core/archive smoke."""

from __future__ import annotations

import os
from datetime import datetime, timezone
from pathlib import Path

from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent.parent
ROOT_DIR = SCRIPT_DIR.parent
CORE_FIXTURE_DIR = ROOT_DIR / "apps" / "elf-eval" / "fixtures" / "real_world_memory" / "core_archival_memory"
REPORT_DIR = Path(
    os.environ.get(
        "ELF_LETTA_SMOKE_REPORT_DIR",
        ROOT_DIR / "tmp" / "real-world-memory" / "letta-core-archive",
    )
)
WORK_DIR = Path(os.environ.get("ELF_LETTA_SMOKE_WORK_DIR", REPORT_DIR / "work"))
OUT = Path(os.environ.get("ELF_LETTA_SMOKE_OUT", REPORT_DIR / "letta-core-archive-export.json"))
MANIFEST_OUT = Path(
    os.environ.get(
        "ELF_LETTA_SMOKE_MANIFEST_OUT",
        REPORT_DIR / "memory_projects_manifest.letta-core-archive.json",
    )
)
SUMMARY_OUT = Path(os.environ.get("ELF_LETTA_SMOKE_SUMMARY_OUT", REPORT_DIR / "summary.json"))
REPORT_JSON = Path(os.environ.get("ELF_LETTA_SMOKE_REPORT_JSON", REPORT_DIR / "report.json"))
REPORT_MD = Path(os.environ.get("ELF_LETTA_SMOKE_REPORT_MD", REPORT_DIR / "report.md"))
FIXTURE_DIR = REPORT_DIR / "letta-fixtures"
LOG_DIR = REPORT_DIR / "logs"

RUN_ID = os.environ.get(
    "ELF_LETTA_SMOKE_RUN_ID",
    f"letta-core-archive-{datetime.now(timezone.utc).strftime('%Y%m%d%H%M%S')}",
)
RUN_LIVE = os.environ.get("ELF_LETTA_SMOKE_RUN", "0") == "1"
ALLOW_HOST = os.environ.get("ELF_LETTA_SMOKE_ALLOW_HOST", "0") == "1"
INSTALL_CLIENT = os.environ.get("ELF_LETTA_SMOKE_INSTALL_CLIENT", "1") == "1"
LETTA_BASE_URL = os.environ.get("ELF_LETTA_BASE_URL", "http://letta:8283")
LETTA_CLIENT_PACKAGE = os.environ.get("ELF_LETTA_CLIENT_PACKAGE", "letta-client")
LETTA_CLIENT_REF = os.environ.get("ELF_LETTA_CLIENT_REF", f"pypi:{LETTA_CLIENT_PACKAGE}")
LETTA_MODEL = os.environ.get("ELF_LETTA_MODEL", "openai/gpt-4o-mini")
LETTA_EMBEDDING = os.environ.get("ELF_LETTA_EMBEDDING", "openai/text-embedding-3-small")
TIMEOUT_SECONDS = int(os.environ.get("ELF_LETTA_TIMEOUT_SECONDS", "600"))
STARTUP_ATTEMPTS = int(os.environ.get("ELF_LETTA_STARTUP_ATTEMPTS", "30"))
STARTUP_INTERVAL_SECONDS = float(os.environ.get("ELF_LETTA_STARTUP_INTERVAL_SECONDS", "2"))

CORE_KINDS = {"core_block", "core_block_contract", "core_block_event"}

