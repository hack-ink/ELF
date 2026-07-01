from __future__ import annotations

import os
from datetime import datetime, timezone
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent.parent
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
