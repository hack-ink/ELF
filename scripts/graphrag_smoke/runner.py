from __future__ import annotations

import time
from pathlib import Path
from typing import Any

from .artifacts import scrub_report_secrets, write_fixture, write_manifest, write_materialization, write_summary
from .benchmark import run_scored_report
from .common import command_available, mkdirs
from .context import ALLOW_HOST, API_KEY, MANIFEST_OUT, OUT, RUN_LIVE, SUMMARY_OUT, WORK_DIR
from .corpus import generated_corpus, write_corpus
from .mapping import empty_table_mappings, map_tables, mapping_is_valid
from .models import CommandRecord, StatusState
from .runtime import init_project, run_graphrag



def main() -> int:
    """Run the smoke and always emit typed artifacts when possible."""

    started_at = time.monotonic()
    mkdirs()
    status = StatusState()
    command_records: list[CommandRecord] = []
    mappings: list[dict[str, Any]] = empty_table_mappings("not_encoded")
    mapped_ids: list[str] = []
    corpus = generated_corpus()
    project_dir = WORK_DIR / "project"
    corpus_csv = write_corpus(project_dir, corpus)

    if not Path("/.dockerenv").exists() and not ALLOW_HOST:
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "not_running_in_docker"
        status.failure_reason = "GraphRAG smoke must run inside Docker; use cargo make smoke-graphrag-docker."
    elif not command_available("python3"):
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "python_missing"
        status.failure_reason = "python3 is required for the GraphRAG smoke runner."
    elif not RUN_LIVE:
        pass
    elif not API_KEY:
        status.setup = "blocked"
        status.run = "not_encoded"
        status.result = "blocked"
        status.overall = "blocked"
        status.failure_class = "provider_api_key_missing"
        status.failure_reason = "GraphRAG live indexing requires an explicit provider API key; no private or unrecorded provider credentials were used."
    elif not init_project(project_dir, command_records):
        status.setup = "incomplete"
        status.run = "not_encoded"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "graphrag_setup_failed"
        status.failure_reason = "GraphRAG installation or initialization failed inside the Docker runner."
    else:
        status.setup = "pass"
        output_dir = run_graphrag(project_dir, command_records)

        if output_dir is None:
            status.run = "incomplete"
            status.result = "incomplete"
            status.overall = "incomplete"
            status.failure_class = "graphrag_index_or_query_failed"
            status.failure_reason = "GraphRAG did not complete both index and local query for the generated corpus."
        else:
            status.run = "pass"
            status.evidence_class = "live_real_world"
            mappings, mapped_ids = map_tables(output_dir, corpus)
            expected_ids = [
                item["evidence_id"]
                for item in corpus
                if item["evidence_id"] != "graphrag-smoke-stale-trap"
            ]
            valid, reason = mapping_is_valid(mappings, expected_ids)

            if valid:
                status.result = "pass"
                status.overall = "pass"
                status.failure_class = ""
                status.failure_reason = ""
            else:
                status.result = "wrong_result"
                status.overall = "wrong_result"
                status.failure_class = "graphrag_evidence_mapping_failed"
                status.failure_reason = reason

    scrub_report_secrets(project_dir)
    fixture_path = write_fixture(corpus, status, mapped_ids)
    manifest = write_manifest(status)
    report = run_scored_report(fixture_path, MANIFEST_OUT, status)
    materialization = write_materialization(
        status,
        corpus,
        fixture_path,
        corpus_csv,
        command_records,
        mappings,
        mapped_ids,
        started_at,
        report,
    )
    write_summary(materialization, manifest, report)
    print(f"GraphRAG smoke artifact: {OUT}")
    print(f"GraphRAG smoke manifest: {MANIFEST_OUT}")
    print(f"GraphRAG smoke summary: {SUMMARY_OUT}")

    return 0
