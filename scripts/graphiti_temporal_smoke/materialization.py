"""Primary materialization writer for the Graphiti/Zep smoke."""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any

from .benchmark import scored_benchmark
from .common import command_to_json, dir_size, file_count, rel, utc_now, write_json
from .context import *  # noqa: F403
from .models import CommandRecord, StatusState

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
