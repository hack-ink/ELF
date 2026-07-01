from __future__ import annotations

import os
from datetime import datetime, timezone
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent.parent
ROOT_DIR = SCRIPT_DIR.parent
REPORT_DIR = Path(
    os.environ.get(
        "ELF_GRAPHRAG_SMOKE_REPORT_DIR",
        ROOT_DIR / "tmp" / "real-world-memory" / "graphrag-smoke",
    )
)
WORK_DIR = Path(os.environ.get("ELF_GRAPHRAG_SMOKE_WORK_DIR", REPORT_DIR / "work"))
OUT = Path(os.environ.get("ELF_GRAPHRAG_SMOKE_OUT", REPORT_DIR / "graphrag-smoke.json"))
MANIFEST_OUT = Path(
    os.environ.get(
        "ELF_GRAPHRAG_SMOKE_MANIFEST_OUT",
        REPORT_DIR / "memory_projects_manifest.graphrag-smoke.json",
    )
)
SUMMARY_OUT = Path(os.environ.get("ELF_GRAPHRAG_SMOKE_SUMMARY_OUT", REPORT_DIR / "summary.json"))
REPORT_JSON = Path(os.environ.get("ELF_GRAPHRAG_SMOKE_REPORT_JSON", REPORT_DIR / "graphrag-report.json"))
REPORT_MD = Path(os.environ.get("ELF_GRAPHRAG_SMOKE_REPORT_MD", REPORT_DIR / "graphrag-report.md"))
FIXTURE_DIR = REPORT_DIR / "graphrag-fixtures"
OUTPUT_CAPTURE_DIR = REPORT_DIR / "graphrag-output"
LOG_DIR = REPORT_DIR / "logs"

RUN_ID = os.environ.get(
    "ELF_GRAPHRAG_SMOKE_RUN_ID",
    f"graphrag-docker-smoke-{datetime.now(timezone.utc).strftime('%Y%m%d%H%M%S')}",
)
RUN_LIVE = os.environ.get("ELF_GRAPHRAG_SMOKE_RUN", "0") == "1"
ALLOW_HOST = os.environ.get("ELF_GRAPHRAG_SMOKE_ALLOW_HOST", "0") == "1"
INSTALL_GRAPHRAG = os.environ.get("ELF_GRAPHRAG_SMOKE_INSTALL", "1") == "1"
GRAPH_RAG_VERSION = os.environ.get("ELF_GRAPHRAG_VERSION", "3.1.0")
GRAPH_RAG_PACKAGE = os.environ.get("ELF_GRAPHRAG_PACKAGE", f"graphrag=={GRAPH_RAG_VERSION}")
GRAPH_RAG_REF = os.environ.get("ELF_GRAPHRAG_REF", f"pypi:{GRAPH_RAG_PACKAGE}")
CHAT_MODEL = os.environ.get("ELF_GRAPHRAG_CHAT_MODEL", "gpt-4o-mini")
EMBEDDING_MODEL = os.environ.get("ELF_GRAPHRAG_EMBEDDING_MODEL", "text-embedding-3-small")
API_BASE = os.environ.get("ELF_GRAPHRAG_API_BASE", "")
API_KEY = os.environ.get("ELF_GRAPHRAG_API_KEY", os.environ.get("GRAPHRAG_API_KEY", ""))
INDEX_METHOD = os.environ.get("ELF_GRAPHRAG_INDEX_METHOD", "fast")
QUERY_METHOD = os.environ.get("ELF_GRAPHRAG_QUERY_METHOD", "local")
TIMEOUT_SECONDS = int(os.environ.get("ELF_GRAPHRAG_TIMEOUT_SECONDS", "900"))
MAX_DOCS = max(1, min(int(os.environ.get("ELF_GRAPHRAG_MAX_DOCS", "3")), 3))
MAX_INPUT_CHARS = max(400, min(int(os.environ.get("ELF_GRAPHRAG_MAX_INPUT_CHARS", "2400")), 6000))

TABLES = (
    "documents",
    "text_units",
    "communities",
    "community_reports",
    "entities",
    "relationships",
)
