"""Configuration for the Graphiti/Zep temporal smoke."""

from __future__ import annotations

import os
from datetime import datetime, timezone
from pathlib import Path

from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent.parent
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
