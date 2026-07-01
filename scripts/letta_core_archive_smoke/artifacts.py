"""Artifact writers for the Letta core/archive smoke."""

from __future__ import annotations

import time
from pathlib import Path
from typing import Any

from .benchmark import scored_benchmark
from .common import command_to_json, rel, utc_now, write_json
from .context import (
    LETTA_BASE_URL,
    LETTA_CLIENT_REF,
    LETTA_EMBEDDING,
    LETTA_MODEL,
    MANIFEST_OUT,
    OUT,
    REPORT_JSON,
    REPORT_MD,
    RUN_ID,
    RUN_LIVE,
    SUMMARY_OUT,
    TIMEOUT_SECONDS,
    WORK_DIR,
)
from .fixtures import benchmark_input_contract
from .models import CommandRecord, StatusState

def write_materialization(
    status: StatusState,
    fixtures: list[dict[str, Any]],
    fixture_path: Path,
    command_records: list[CommandRecord],
    live_export: dict[str, Any] | None,
    mapping: dict[str, Any],
    started_at: float,
    report: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Write the primary Letta materialization artifact."""

    elapsed_ms = (time.monotonic() - started_at) * 1000
    payload = {
        "schema": "elf.letta_core_archive_export_readback/v1",
        "generated_at": utc_now(),
        "run_id": RUN_ID,
        "adapter_id": "letta_core_archive_export_readback",
        "project": "Letta",
        "evidence_class": status.evidence_class,
        "status": {
            "source": "smoke_materialization",
            "setup": status.setup,
            "run": status.run,
            "result": status.result,
            "overall": status.overall,
            "failure_class": status.failure_class,
            "failure_reason": status.failure_reason,
        },
        "scored_benchmark": scored_benchmark(report),
        "artifacts": {
            "materialization": rel(OUT),
            "manifest": rel(MANIFEST_OUT),
            "summary": rel(SUMMARY_OUT),
            "generated_fixture_dir": rel(fixture_path),
            "scored_report_json": rel(REPORT_JSON),
            "scored_report_markdown": rel(REPORT_MD),
            "live_output": rel(WORK_DIR / "letta-live-output.json")
            if (WORK_DIR / "letta-live-output.json").exists()
            else None,
        },
        "docker_boundary": {
            "compose_file": "docker-compose.baseline.yml",
            "service_profile": "letta",
            "runner_service": "baseline-runner",
            "runner": "scripts/letta-core-archive-export-readback-smoke.py",
            "host_global_installs_required": False,
            "docker_only": True,
            "host_global_letta_state_used": False,
            "hosted_letta_state_used": False,
        },
        "provider_configuration": {
            "base_url": LETTA_BASE_URL,
            "client_package": LETTA_CLIENT_REF,
            "model": LETTA_MODEL,
            "embedding": LETTA_EMBEDDING,
            "live_run_enabled": RUN_LIVE,
            "operator_owned_provider_credentials_used": False,
        },
        "benchmark_input": benchmark_input_contract(fixtures),
        "letta_export": {
            "core_block_json": live_export.get("core_block_export", []) if live_export else [],
            "archival_readback_json": live_export.get("archival_readback") if live_export else None,
            "archival_search_json": live_export.get("archival_search", []) if live_export else [],
            "agent": live_export.get("agent") if live_export else None,
            "status": "exported" if live_export else status.result,
        },
        "resource_bounds": {
            "source_fixture_count": len(fixtures),
            "core_block_count": len(benchmark_input_contract(fixtures)["core_blocks"]),
            "archival_passage_count": len(benchmark_input_contract(fixtures)["archival_passages"]),
            "timeout_seconds": TIMEOUT_SECONDS,
            "elapsed_ms": round(elapsed_ms, 3),
        },
        "commands": [command_to_json(record) for record in command_records],
        "evidence_mapping": mapping,
        "improvement_regression_readback": {
            "baseline": "XY-955 left Letta core/archive comparison blocked because no contained export/readback artifact existed.",
            "current": (
                "unchanged: the benchmark now has a Docker-contained materialization command and typed report, "
                "but the default run still preserves Letta comparison as blocked until live export/search data maps source ids."
            )
            if status.result != "pass"
            else "improved: Letta export/readback mapped all required core/archive source ids.",
            "judgment": "improved" if status.result == "pass" else "unchanged",
        },
        "claim_boundaries": {
            "allowed": [
                "The Letta comparison now has a reproducible Docker-contained materialization/report command.",
                "The current default report may preserve typed blockers when live Letta/provider setup cannot produce export/readback evidence.",
            ],
            "not_allowed": [
                "Do not claim ELF beats Letta on core-vs-archival memory from fixture-only ELF evidence.",
                "Do not score Letta pass, win, tie, or loss unless exported core block JSON, archival readback/search JSON, and fixture source ids are present.",
            ],
        },
    }
    write_json(OUT, payload)

    return payload

def write_manifest(status: StatusState) -> dict[str, Any]:
    """Write a generated external adapter manifest for this smoke."""

    manifest = {
        "schema": "elf.real_world_external_adapter_manifest/v1",
        "manifest_id": f"letta-core-archive-export-readback-{RUN_ID}",
        "docker_isolation": {
            "default": True,
            "compose_file": "docker-compose.baseline.yml",
            "runner": "scripts/letta-core-archive-export-readback-smoke.py",
            "artifact_dir": "tmp/real-world-memory/letta-core-archive",
            "host_global_installs_required": False,
            "notes": [
                f"Generated by the Letta core/archive export-readback smoke at {utc_now()}.",
                "The smoke uses checked-in core_archival_memory fixtures and records typed setup/runtime failures.",
            ],
        },
        "adapters": [
            {
                "adapter_id": "letta_core_archive_export_readback",
                "project": "Letta",
                "adapter_kind": "docker_core_archive_export_readback",
                "evidence_class": status.evidence_class,
                "docker_default": True,
                "host_global_installs_required": False,
                "overall_status": status.overall,
                "setup": {
                    "status": status.setup,
                    "evidence": "The smoke runs inside the baseline Docker runner and can use a Docker-profile Letta server with explicit model and embedding configuration.",
                    "command": "cargo make smoke-letta-core-archive-export-readback",
                    "artifact": rel(OUT),
                },
                "run": {
                    "status": status.run,
                    "evidence": "The live path creates a benchmark-owned Letta agent, imports fixture source ids into core blocks and archival passages, then exports block/readback/search JSON.",
                    "command": "ELF_LETTA_SMOKE_START=1 ELF_LETTA_SMOKE_RUN=1 cargo make smoke-letta-core-archive-export-readback",
                    "artifact": rel(OUT),
                },
                "result": {
                    "status": status.result,
                    "evidence": status.failure_reason
                    if status.failure_reason
                    else "Letta core block export, archival readback, and archival search mapped required fixture source ids.",
                    "artifact": rel(OUT),
                },
                "capabilities": [
                    {
                        "capability": "docker_letta_server_boundary",
                        "status": status.setup,
                        "evidence": "The runner uses docker-compose.baseline.yml and avoids host-global Letta state or hosted/private agents.",
                    },
                    {
                        "capability": "core_block_export",
                        "status": status.run,
                        "evidence": "Live scoring requires retrieving Letta memory blocks with fixture source ids embedded in block values.",
                    },
                    {
                        "capability": "archival_readback_search_export",
                        "status": status.result,
                        "evidence": "Live scoring requires archival passage list/search JSON to map required source ids.",
                    },
                    {
                        "capability": "broad_letta_quality_claim",
                        "status": "not_encoded",
                        "evidence": "The smoke does not claim broad Letta product quality, private corpus behavior, or hosted-service parity.",
                    },
                ],
                "suites": [
                    {
                        "suite_id": "core_archival_memory",
                        "status": status.result,
                        "evidence": "Only the six checked-in core_archival_memory scenarios are represented.",
                    },
                    {
                        "suite_id": "personalization",
                        "status": "not_encoded",
                        "evidence": "Scoped preference behavior is outside this core/archive export smoke.",
                    },
                    {
                        "suite_id": "project_decisions",
                        "status": status.result,
                        "evidence": "Project-decision recovery is scored only through the core_archival_memory fixture that requires core routing plus archival rationale source ids.",
                    },
                    {
                        "suite_id": "work_resume",
                        "status": "not_encoded",
                        "evidence": "Agent resumption across sessions is not encoded by this export/readback smoke.",
                    },
                ],
                "evidence": [
                    {"kind": "artifact", "ref": rel(OUT), "status": status.result},
                    {"kind": "manifest", "ref": rel(MANIFEST_OUT), "status": status.overall},
                    {"kind": "source", "ref": "https://docs.letta.com/guides/docker", "status": "real"},
                    {"kind": "source", "ref": "https://docs.letta.com/api/python", "status": "real"},
                    {
                        "kind": "source",
                        "ref": "https://docs.letta.com/api/resources/agents/subresources/passages/methods/search",
                        "status": "real",
                    },
                ],
                "execution_metadata": {
                    "sources": [
                        {
                            "label": "Letta Docker docs",
                            "url": "https://docs.letta.com/guides/docker",
                            "evidence": "Official Docker setup and explicit embedding configuration boundary.",
                        },
                        {
                            "label": "Letta Python API",
                            "url": "https://docs.letta.com/api/python",
                            "evidence": "Official Python SDK memory block creation and retrieval examples.",
                        },
                        {
                            "label": "Letta archival search API",
                            "url": "https://docs.letta.com/api/resources/agents/subresources/passages/methods/search",
                            "evidence": "Official archival-memory search endpoint contract.",
                        },
                    ],
                    "setup_path": "Run cargo make smoke-letta-core-archive-export-readback for a typed artifact; set ELF_LETTA_SMOKE_START=1 ELF_LETTA_SMOKE_RUN=1 with explicit model/provider configuration for a live export attempt.",
                    "runtime_boundary": "docker-compose.baseline.yml baseline-runner plus optional Letta server profile, benchmark-created agent, benchmark-owned fixture corpus, and artifacts under tmp/real-world-memory/letta-core-archive.",
                    "resource_expectation": f"Letta client {LETTA_CLIENT_REF}, model={LETTA_MODEL}, embedding={LETTA_EMBEDDING}, source fixture count=6, timeout_seconds={TIMEOUT_SECONDS}.",
                    "retry_guidance": [
                        "Default command records a typed blocked artifact without model calls.",
                        "Enable the live path only with a Docker-local Letta server and explicit provider or local model configuration.",
                        "Score only when core block export and archival list/search output map to required fixture source ids.",
                    ],
                    "research_depth": "XY-984 materialization contract; generated artifact decides live evidence class.",
                },
                "notes": [
                    "Failure before Letta export/readback remains typed as blocked or incomplete.",
                    "The smoke does not use hosted/private Letta state or operator-owned data.",
                ],
            }
        ],
    }
    write_json(MANIFEST_OUT, manifest)

    return manifest

def write_summary(materialization: dict[str, Any], manifest: dict[str, Any], report: dict[str, Any]) -> None:
    """Write a small summary artifact."""

    write_json(
        SUMMARY_OUT,
        {
            "schema": "elf.letta_core_archive_export_readback_summary/v1",
            "generated_at": utc_now(),
            "adapter_id": "letta_core_archive_export_readback",
            "evidence_class": materialization["evidence_class"],
            "status_boundary": {
                "materialization": "setup/run/evidence-mapping state emitted by the smoke runner",
                "manifest": "external adapter declaration consumed by the scorer",
                "scored_benchmark": "post-score real_world_job outcome; use this for quality status",
            },
            "scored_benchmark": materialization["scored_benchmark"],
            "materialization": materialization,
            "manifest": {
                "json": rel(MANIFEST_OUT),
                "status_source": "external_adapter_manifest_score_aligned",
                "summary": manifest["adapters"][0]["overall_status"],
                "suites": manifest["adapters"][0]["suites"],
            },
            "report": report,
        },
    )
