"""CLI runner for the Graphiti/Zep temporal smoke."""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any

from .fixture import write_fixture
from .manifest import write_manifest
from .materialization import write_materialization
from .summary import write_summary
from .benchmark import run_scored_report
from .common import command_available, mkdirs
from .context import ALLOW_HOST, MANIFEST_OUT, OUT, RUN_LIVE, SUMMARY_OUT
from .corpus import temporal_facts
from .mapping import map_observed_facts
from .models import CommandRecord, StatusState
from .runtime import init_graphiti, run_graphiti, wait_for_falkordb

def main() -> int:
    """Run the smoke and always emit typed artifacts when possible."""

    started_at = time.monotonic()
    mkdirs()
    status = StatusState()
    command_records: list[CommandRecord] = []
    facts = temporal_facts()
    inserted: list[dict[str, Any]] = []
    search_results: list[dict[str, Any]] = []
    mapping: dict[str, Any] = {
        "status": "blocked",
        "reason": status.failure_reason,
        "expected_evidence_ids": [fact["evidence_id"] for fact in facts],
        "mapped_evidence_ids": [],
        "facts": [
            {
                "evidence_id": fact["evidence_id"],
                "claim_id": fact["claim_id"],
                "status": "blocked",
                "expected_valid_at": fact["valid_at"],
                "expected_invalid_at": fact["invalid_at"],
                "current": fact["current"],
            }
            for fact in facts
        ],
    }

    if not Path("/.dockerenv").exists() and not ALLOW_HOST:
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "not_running_in_docker"
        status.failure_reason = "Graphiti/Zep smoke must run inside Docker; use cargo make smoke-graphiti-zep-docker-temporal."
        mapping["status"] = status.result
        mapping["reason"] = status.failure_reason
    elif not command_available("python3"):
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "python_missing"
        status.failure_reason = "python3 is required for the Graphiti/Zep smoke runner."
        mapping["status"] = status.result
        mapping["reason"] = status.failure_reason
    elif not RUN_LIVE:
        pass
    elif not API_KEY:
        status.setup = "blocked"
        status.run = "not_encoded"
        status.result = "blocked"
        status.overall = "blocked"
        status.failure_class = "provider_api_key_missing"
        status.failure_reason = "Graphiti/Zep live temporal search requires an explicit provider API key; no hosted Zep service or unrecorded provider credentials were used."
        mapping["reason"] = status.failure_reason
    elif not wait_for_falkordb(command_records):
        status.setup = "incomplete"
        status.run = "not_encoded"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "falkordb_unreachable"
        status.failure_reason = "Docker-local FalkorDB did not become reachable for the Graphiti/Zep smoke."
        mapping["status"] = status.result
        mapping["reason"] = status.failure_reason
    else:
        installed, python = init_graphiti(command_records)
        if not installed:
            status.setup = "incomplete"
            status.run = "not_encoded"
            status.result = "incomplete"
            status.overall = "incomplete"
            status.failure_class = "graphiti_setup_failed"
            status.failure_reason = "Graphiti installation failed inside the Docker runner."
            mapping["status"] = status.result
            mapping["reason"] = status.failure_reason
        else:
            status.setup = "pass"
            inserted, search_results = run_graphiti(python, command_records)

            if not search_results:
                status.run = "incomplete"
                status.result = "incomplete"
                status.overall = "incomplete"
                status.failure_class = "graphiti_temporal_search_failed"
                status.failure_reason = "Graphiti/Zep did not return temporal search results for the generated fact corpus."
                mapping["status"] = status.result
                mapping["reason"] = status.failure_reason
            else:
                status.run = "pass"
                status.evidence_class = "live_real_world"
                mapping = map_observed_facts(search_results, facts)
                if mapping["status"] == "pass":
                    status.result = "pass"
                    status.overall = "pass"
                    status.failure_class = ""
                    status.failure_reason = ""
                else:
                    status.result = "wrong_result"
                    status.overall = "wrong_result"
                    status.failure_class = "graphiti_temporal_mapping_failed"
                    status.failure_reason = mapping["reason"]

    fixture_path = write_fixture(facts, status, mapping)
    materialization = write_materialization(
        status,
        facts,
        fixture_path,
        command_records,
        inserted,
        search_results,
        mapping,
        started_at,
    )
    manifest = write_manifest(status)
    report = run_scored_report(fixture_path, MANIFEST_OUT, status)
    materialization = write_materialization(
        status,
        facts,
        fixture_path,
        command_records,
        inserted,
        search_results,
        mapping,
        started_at,
        report,
    )
    write_summary(materialization, manifest, report)
    print(f"Graphiti/Zep smoke artifact: {OUT}")
    print(f"Graphiti/Zep smoke manifest: {MANIFEST_OUT}")
    print(f"Graphiti/Zep smoke summary: {SUMMARY_OUT}")

    return 0
