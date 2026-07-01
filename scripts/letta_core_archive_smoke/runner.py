"""CLI runner for the Letta core/archive smoke."""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any

from .artifacts import write_manifest, write_materialization, write_summary
from .benchmark import run_scored_report
from .common import command_available, mkdirs
from .context import ALLOW_HOST, MANIFEST_OUT, OUT, RUN_LIVE, SUMMARY_OUT
from .fixtures import evidence_mapping, load_source_fixtures, write_fixture_outputs
from .models import CommandRecord, StatusState
from .runtime import init_letta_client, run_letta, wait_for_letta

def main() -> int:
    """Run the smoke and always emit typed artifacts when possible."""

    started_at = time.monotonic()
    mkdirs()
    status = StatusState()
    command_records: list[CommandRecord] = []
    fixtures = load_source_fixtures()
    live_export: dict[str, Any] | None = None

    if not Path("/.dockerenv").exists() and not ALLOW_HOST:
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "not_running_in_docker"
        status.failure_reason = "Letta smoke must run inside Docker; use cargo make smoke-letta-core-archive-export-readback."
    elif not command_available("python3"):
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "python_missing"
        status.failure_reason = "python3 is required for the Letta smoke runner."
    elif not RUN_LIVE:
        pass
    elif not wait_for_letta(command_records):
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "letta_server_unreachable"
        status.failure_reason = "Docker-local Letta server did not become reachable for export/readback."
    elif not init_letta_client(command_records):
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "letta_client_setup_failed"
        status.failure_reason = "Letta Python client installation or import failed inside the Docker runner."
    else:
        status.setup = "pass"
        live_export = run_letta(fixtures, command_records)
        if live_export is None:
            status.run = "incomplete"
            status.result = "incomplete"
            status.overall = "incomplete"
            status.failure_class = "letta_export_readback_failed"
            status.failure_reason = "Letta did not produce core block export plus archival readback/search output."
        else:
            status.run = "pass"
            status.evidence_class = "live_real_world"
            mapping = evidence_mapping(fixtures, live_export, status)
            if not mapping["missing_evidence_ids"]:
                status.result = "pass"
                status.overall = "pass"
                status.failure_class = ""
                status.failure_reason = ""
            else:
                status.result = "wrong_result"
                status.overall = "wrong_result"
                status.failure_class = "letta_source_id_mapping_failed"
                status.failure_reason = mapping["reason"]

    mapping = evidence_mapping(fixtures, live_export, status)
    fixture_path = write_fixture_outputs(fixtures, status, mapping)
    write_materialization(
        status,
        fixtures,
        fixture_path,
        command_records,
        live_export,
        mapping,
        started_at,
    )
    manifest = write_manifest(status)
    report = run_scored_report(fixture_path, MANIFEST_OUT, status)
    materialization = write_materialization(
        status,
        fixtures,
        fixture_path,
        command_records,
        live_export,
        mapping,
        started_at,
        report,
    )
    write_summary(materialization, manifest, report)
    print(f"Letta core/archive artifact: {OUT}")
    print(f"Letta core/archive manifest: {MANIFEST_OUT}")
    print(f"Letta core/archive summary: {SUMMARY_OUT}")

    return 0
