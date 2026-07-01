from __future__ import annotations

import time
from pathlib import Path
from typing import Any

from .artifacts import write_fixture, write_manifest, write_materialization, write_summary
from .benchmark import run_scored_report, status_with_scored_result
from .common import command_available, mkdirs
from .context import ALLOW_HOST, MANIFEST_OUT, OUT, RUN_GRAPHIFY, SUMMARY_OUT
from .corpus import expected_ids, generated_corpus, write_corpus
from .mapping import map_artifacts, mapping_outcome
from .models import CommandRecord, StatusState
from .runtime import install_graphify, run_graphify



def main() -> int:
    """Run the smoke and always emit typed artifacts when possible."""

    started_at = time.monotonic()
    mkdirs()
    status = StatusState()
    command_records: list[CommandRecord] = []
    corpus = generated_corpus()
    corpus_csv = write_corpus(corpus)
    mappings = {
        "expected_evidence_ids": expected_ids(corpus),
        "mapped_evidence_ids": [],
        "graph_json": {"artifact": None, "exists": False, "size_bytes": 0},
        "graph_report": {
            "kind": "graph_report",
            "artifact": None,
            "exists": False,
            "size_bytes": 0,
            "evidence_ids": [],
        },
        "query_output": {
            "kind": "query_output",
            "artifact": None,
            "exists": False,
            "command_status": "not_encoded",
            "evidence_ids": [],
        },
        "nodes": [],
        "edges": [],
    }

    if not Path("/.dockerenv").exists() and not ALLOW_HOST:
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "not_running_in_docker"
        status.failure_reason = "graphify smoke must run inside Docker; use cargo make smoke-graphify-docker-graph-report."
    elif not command_available("python3"):
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "python_missing"
        status.failure_reason = "python3 is required for the graphify smoke runner."
    elif not RUN_GRAPHIFY:
        pass
    else:
        graphify = install_graphify(command_records)

        if graphify is None:
            status.setup = "incomplete"
            status.result = "incomplete"
            status.overall = "incomplete"
            status.failure_class = "graphify_setup_failed"
            status.failure_reason = "graphify installation or help command failed inside the Docker runner."
        else:
            status.setup = "pass"
            output_dir = run_graphify(graphify, command_records)

            if output_dir is None:
                status.run = "incomplete"
                status.result = "incomplete"
                status.overall = "incomplete"
                status.failure_class = "graphify_build_failed"
                status.failure_reason = "graphify did not build graph/report artifacts for the generated corpus."
            else:
                status.run = "pass"
                status.evidence_class = "live_real_world"
                mappings = map_artifacts(corpus, command_records)
                result_status, reason = mapping_outcome(mappings, command_records)
                status.result = result_status
                status.overall = result_status

                if result_status == "pass":
                    status.failure_class = ""
                    status.failure_reason = ""
                else:
                    status.failure_class = "graphify_evidence_mapping_failed"
                    status.failure_reason = reason

    fixture_path = write_fixture(corpus, status, mappings["mapped_evidence_ids"])
    materialization = write_materialization(
        status,
        corpus,
        fixture_path,
        corpus_csv,
        command_records,
        mappings,
        started_at,
    )
    manifest = write_manifest(status)
    report = run_scored_report(fixture_path, MANIFEST_OUT, status)
    manifest_status = status_with_scored_result(status, report)
    if manifest_status.overall != status.overall or manifest_status.result != status.result:
        manifest = write_manifest(manifest_status)
        report = run_scored_report(fixture_path, MANIFEST_OUT, manifest_status)
    materialization = write_materialization(
        status,
        corpus,
        fixture_path,
        corpus_csv,
        command_records,
        mappings,
        started_at,
        report,
    )
    write_summary(materialization, manifest, report)
    print(f"graphify smoke artifact: {OUT}")
    print(f"graphify smoke manifest: {MANIFEST_OUT}")
    print(f"graphify smoke summary: {SUMMARY_OUT}")

    return 0
