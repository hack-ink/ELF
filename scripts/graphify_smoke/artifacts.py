from __future__ import annotations

import time
from pathlib import Path
from typing import Any

from .benchmark import scored_benchmark
from .common import dir_size, file_count, rel, utc_now, write_json
from .context import (
    CORPUS_DIR,
    FIXTURE_DIR,
    GRAPHIFY_PACKAGE,
    GRAPHIFY_REF,
    MANIFEST_OUT,
    OUT,
    OUTPUT_CAPTURE_DIR,
    QUERY_BUDGET,
    REPORT_JSON,
    REPORT_MD,
    RUN_GRAPHIFY,
    RUN_ID,
    SUMMARY_OUT,
    TIMEOUT_SECONDS,
)
from .corpus import expected_ids
from .models import CommandRecord, CorpusItem, StatusState
from .runtime import command_to_json



def write_fixture(corpus: list[CorpusItem], status: StatusState, mapped_ids: list[str]) -> Path:
    """Write a generated real_world_job fixture for the graphify smoke."""

    fixture_path = FIXTURE_DIR / "knowledge" / "graphify_graph_report.json"
    used_ids = [evidence_id for evidence_id in mapped_ids if evidence_id in expected_ids(corpus)]
    response = {
        "adapter_id": "graphify_docker_smoke",
        "answer": {
            "content": (
                "graphify connected the ELF memory service, Qdrant rebuild, and graph report mapping "
                "through graph/report artifacts that cite generated source evidence."
                if used_ids
                else ""
            ),
            "claims": [
                {
                    "claim_id": "graphify_report_evidence_mapping",
                    "text": (
                        "graphify graph/report artifacts map back to the generated ELF memory service, "
                        "Qdrant rebuild, and report mapping evidence ids."
                    ),
                    "evidence_ids": used_ids,
                    "confidence": "derived_from_graphify_graph_report_mapping",
                }
            ]
            if used_ids
            else [],
            "evidence_ids": used_ids,
            "pages": [
                {
                    "page_id": "graphify:graph-report",
                    "page_type": "concept",
                    "title": "graphify Graph Report",
                    "path": rel(OUTPUT_CAPTURE_DIR / "GRAPH_REPORT.md"),
                    "sections": [
                        {
                            "section_id": "derived-graph-report",
                            "heading": "Derived Graph Report",
                            "role": "summary",
                            "content": "GRAPH_REPORT.md is a derived graphify artifact, not authoritative ELF memory.",
                            "evidence_ids": used_ids,
                            "timeline_event_ids": ["graphify-smoke-built-graph-report"],
                            "unsupported_reason": None if used_ids else "graphify output was not mapped.",
                        }
                    ],
                    "backlinks": used_ids,
                    "lint_findings": [],
                }
            ]
            if (OUTPUT_CAPTURE_DIR / "GRAPH_REPORT.md").exists()
            else [],
            "latency_ms": 0.0,
            "cost": {
                "currency": "USD",
                "amount": 0.0,
                "input_tokens": 0,
                "output_tokens": 0,
            },
        },
    }
    fixture: dict[str, Any] = {
        "schema": "elf.real_world_job/v1",
        "job_id": "graphify-graph-report-001",
        "suite": "knowledge_compilation",
        "title": "Map graphify graph/report output to generated evidence",
        "corpus": {
            "corpus_id": "graphify-generated-public-smoke",
            "profile": "generated_public",
            "items": [
                {
                    "evidence_id": item.evidence_id,
                    "kind": item.kind,
                    "text": item.text,
                    "source_ref": {
                        "schema": "source_ref/v1",
                        "resolver": "graphify_smoke/v1",
                        "ref": {
                            "run_id": RUN_ID,
                            "file": item.file_name,
                            "line": item.line,
                            "evidence_id": item.evidence_id,
                        },
                    },
                    "created_at": "2026-06-10T00:00:00Z",
                }
                for item in corpus
            ],
            "adapter_response": response,
        },
        "timeline": [
            {
                "event_id": "graphify-smoke-corpus-generated",
                "ts": "2026-06-10T00:00:00Z",
                "actor": "system",
                "action": "generated_public_corpus",
                "evidence_ids": expected_ids(corpus),
                "summary": "The graphify smoke generated a tiny public corpus for source mapping.",
            },
            {
                "event_id": "graphify-smoke-built-graph-report",
                "ts": "2026-06-10T00:01:00Z",
                "actor": "system",
                "action": "built_derived_graph_report",
                "evidence_ids": used_ids,
                "summary": "graphify built derived graph/report artifacts when the Docker smoke reached execution.",
            },
        ],
        "prompt": {
            "role": "user",
            "content": "What does graphify connect in the generated ELF graph/report smoke?",
            "job_mode": "compile",
            "constraints": ["cite_evidence", "avoid_stale_facts", "do_not_claim_authoritative_store"],
        },
        "expected_answer": {
            "must_include": [
                {
                    "claim_id": "graphify_report_evidence_mapping",
                    "text": (
                        "graphify connects the ELF memory service, Qdrant rebuild, and graph report "
                        "mapping through derived graph/report artifacts."
                    ),
                }
            ],
            "must_not_include": ["graphify output is an authoritative ELF memory store."],
            "evidence_links": {"graphify_report_evidence_mapping": expected_ids(corpus)},
            "answer_type": "compiled_knowledge",
            "accepted_alternates": [],
            "requires_caveat": True,
            "requires_refusal": False,
        },
        "required_evidence": [
            {
                "evidence_id": item.evidence_id,
                "claim_id": "graphify_report_evidence_mapping",
                "requirement": "cite",
                "quote": item.evidence_id,
            }
            for item in corpus
            if item.expected
        ],
        "negative_traps": [
            {
                "trap_id": "graphify-authoritative-store",
                "type": "unsupported_claim",
                "evidence_ids": ["graphify-smoke-stale-trap"],
                "failure_if_used": True,
            }
        ],
        "scoring_rubric": {
            "dimensions": {
                "answer_correctness": {
                    "weight": 0.25,
                    "max_points": 1.0,
                    "criteria": "States the graph/report connection without broad quality claims.",
                },
                "evidence_grounding": {
                    "weight": 0.4,
                    "max_points": 1.0,
                    "criteria": "Maps graphify output back to generated evidence ids.",
                },
                "trap_avoidance": {
                    "weight": 0.2,
                    "max_points": 1.0,
                    "criteria": "Does not treat graphify output as an authoritative ELF memory store.",
                },
                "latency_resource": {
                    "weight": 0.15,
                    "max_points": 1.0,
                    "criteria": "Records build time, artifact sizes, provider boundary, and retry behavior.",
                },
            },
            "pass_threshold": 0.75,
            "hard_fail_rules": [],
        },
        "allowed_uncertainty": {
            "can_answer_unknown": False,
            "acceptable_phrases": ["tiny generated corpus", "derived graph/report adapter"],
            "fallback_action": "state_blocker",
        },
        "operator_debug": None,
        "encoding": {},
        "memory_evolution": None,
        "tags": ["external_adapter", "generated_public", "graphify", "no_live_claim"],
    }

    if status.result in {"blocked", "incomplete"}:
        fixture["encoding"] = {
            "status": status.result,
            "reason": status.failure_reason,
        }

    write_json(fixture_path, fixture)

    return fixture_path


def write_materialization(
    status: StatusState,
    corpus: list[CorpusItem],
    fixture_path: Path,
    corpus_csv: Path,
    command_records: list[CommandRecord],
    mappings: dict[str, Any],
    started_at: float,
    report: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Write the primary smoke artifact."""

    elapsed_ms = (time.monotonic() - started_at) * 1000
    graph_json = OUTPUT_CAPTURE_DIR / "graph.json"
    graph_report = OUTPUT_CAPTURE_DIR / "GRAPH_REPORT.md"
    cache_dir = OUTPUT_CAPTURE_DIR / "cache"
    query_record = next((record for record in command_records if record.label == "graphify-query"), None)
    payload = {
        "schema": "elf.graphify_docker_graph_report_smoke/v1",
        "generated_at": utc_now(),
        "run_id": RUN_ID,
        "adapter_id": "graphify_docker_smoke",
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
            "generated_corpus_csv": rel(corpus_csv),
            "generated_corpus_dir": rel(CORPUS_DIR),
            "generated_fixture": rel(fixture_path),
            "graph_output_dir": rel(OUTPUT_CAPTURE_DIR),
            "graph_json": rel(graph_json) if graph_json.exists() else None,
            "graph_report": rel(graph_report) if graph_report.exists() else None,
            "query_output": query_record.stdout_artifact if query_record else None,
            "manifest": rel(MANIFEST_OUT),
            "summary": rel(SUMMARY_OUT),
            "scored_report_json": rel(REPORT_JSON),
            "scored_report_markdown": rel(REPORT_MD),
        },
        "docker_boundary": {
            "compose_file": "docker-compose.baseline.yml",
            "runner_service": "baseline-runner",
            "runner": "scripts/graphify-docker-graph-report-smoke.py",
            "host_global_installs_required": False,
            "docker_only": True,
            "assistant_hook_install_used": False,
            "isolated_home": True,
        },
        "model_provider_boundary": {
            "package": GRAPHIFY_REF,
            "package_spec": GRAPHIFY_PACKAGE,
            "assistant_platform_hooks_used": False,
            "host_global_assistant_config_used": False,
            "operator_owned_provider_credentials_used": False,
            "provider_or_model_name": "graphify CLI default; no model configured by this runner",
            "live_run_enabled": RUN_GRAPHIFY,
        },
        "resource_bounds": {
            "generated_file_count": len(corpus),
            "generated_input_chars": sum(len(item.text) for item in corpus),
            "timeout_seconds": TIMEOUT_SECONDS,
            "elapsed_ms": round(elapsed_ms, 3),
            "graph_json_size_bytes": graph_json.stat().st_size if graph_json.exists() else 0,
            "graph_report_size_bytes": graph_report.stat().st_size if graph_report.exists() else 0,
            "graph_output_size_bytes": dir_size(OUTPUT_CAPTURE_DIR),
            "cache_size_bytes": dir_size(cache_dir),
            "cache_file_count": file_count(cache_dir),
        },
        "retry_behavior": {
            "max_attempts": 1,
            "retries_performed": 0,
            "retry_guidance": "Rerun the same Docker command after setup/runtime fixes; do not use host assistant hooks as proof.",
        },
        "commands": [command_to_json(record) for record in command_records],
        "evidence_mapping": mappings,
    }
    write_json(OUT, payload)

    return payload


def write_manifest(status: StatusState) -> dict[str, Any]:
    """Write a generated external adapter manifest for this smoke."""

    manifest = {
        "schema": "elf.real_world_external_adapter_manifest/v1",
        "manifest_id": f"graphify-docker-smoke-{RUN_ID}",
        "docker_isolation": {
            "default": True,
            "compose_file": "docker-compose.baseline.yml",
            "runner": "scripts/graphify-docker-graph-report-smoke.py",
            "artifact_dir": "tmp/real-world-memory/graphify-smoke",
            "host_global_installs_required": False,
            "notes": [
                f"Generated by the graphify Docker graph/report smoke at {utc_now()}.",
                "The smoke uses generated public source files and records typed setup/runtime failures.",
            ],
        },
        "adapters": [
            {
                "adapter_id": "graphify_docker_smoke",
                "project": "graphify",
                "adapter_kind": "docker_cli_graph_report_smoke",
                "evidence_class": status.evidence_class,
                "docker_default": True,
                "host_global_installs_required": False,
                "overall_status": status.overall,
                "setup": {
                    "status": status.setup,
                    "evidence": "The smoke installs graphify in a container-local Python venv and runs with isolated assistant config paths.",
                    "command": "cargo make smoke-graphify-docker-graph-report",
                    "artifact": rel(OUT),
                },
                "run": {
                    "status": status.run,
                    "evidence": "The live path builds graphify graph/report artifacts from a generated public corpus and runs graphify query over graph.json.",
                    "command": "cargo make smoke-graphify-docker-graph-report",
                    "artifact": rel(OUT),
                },
                "result": {
                    "status": status.result,
                    "evidence": status.failure_reason
                    if status.failure_reason
                    else "graphify graph.json, GRAPH_REPORT.md, and query output mapped to generated real_world_job evidence ids.",
                    "artifact": rel(OUT),
                },
                "capabilities": [
                    {
                        "capability": "docker_cli_boundary",
                        "status": status.setup,
                        "evidence": "The runner uses docker-compose.baseline.yml baseline-runner and does not install graphify or assistant hooks on the host.",
                    },
                    {
                        "capability": "graph_report_generation",
                        "status": status.run,
                        "evidence": "The smoke captures graphify-out/graph.json, GRAPH_REPORT.md, cache metadata, and command logs when build succeeds.",
                    },
                    {
                        "capability": "graph_query_evidence_mapping",
                        "status": status.result,
                        "evidence": "Node labels, edge types, confidence tags, source files, source locations, report text, and query output are scanned for generated evidence ids.",
                    },
                    {
                        "capability": "quality_or_scale_claim",
                        "status": "not_encoded",
                        "evidence": "The smoke does not claim multimodal, private corpus, broad codebase-understanding, or large-corpus graph quality.",
                    },
                ],
                "suites": [
                    {
                        "suite_id": "knowledge_compilation",
                        "status": status.result,
                        "evidence": "Only the generated graph/report evidence-mapping job is represented.",
                    },
                    {
                        "suite_id": "retrieval",
                        "status": "blocked",
                        "evidence": "The smoke uses graphify query output only to support source mapping; broad retrieval quality is not scored.",
                    },
                    {
                        "suite_id": "work_resume",
                        "status": "not_encoded",
                        "evidence": "Resume-answer behavior is not encoded by this graph/report smoke.",
                    },
                    {
                        "suite_id": "production_ops",
                        "status": "not_encoded",
                        "evidence": "The smoke records resource bounds but does not encode backup, restore, provider credential, or private corpus operations.",
                    },
                ],
                "evidence": [
                    {"kind": "artifact", "ref": rel(OUT), "status": status.result},
                    {"kind": "artifact", "ref": rel(OUTPUT_CAPTURE_DIR), "status": status.result},
                    {"kind": "manifest", "ref": rel(MANIFEST_OUT), "status": status.overall},
                    {"kind": "source", "ref": "https://github.com/safishamsi/graphify", "status": "real"},
                    {
                        "kind": "source",
                        "ref": "https://github.com/safishamsi/graphify/blob/v3/README.md",
                        "status": "real",
                    },
                ],
                "execution_metadata": {
                    "sources": [
                        {
                            "label": "graphify repository",
                            "url": "https://github.com/safishamsi/graphify",
                            "evidence": "Official source for graphify graph extraction and query workflow.",
                        },
                        {
                            "label": "graphify README",
                            "url": "https://github.com/safishamsi/graphify/blob/v3/README.md",
                            "evidence": "Official CLI, output artifact, query, confidence, and source-location contract.",
                        },
                        {
                            "label": "graphify PyPI package",
                            "url": "https://pypi.org/project/graphifyy/",
                            "evidence": "Official package referenced by the graphify README.",
                        },
                    ],
                    "setup_path": "Run cargo make smoke-graphify-docker-graph-report to install graphify in a container-local venv and build graph/report artifacts over generated public files.",
                    "runtime_boundary": "docker-compose.baseline.yml baseline-runner, isolated HOME/config paths, generated corpus, and artifacts under tmp/real-world-memory/graphify-smoke.",
                    "resource_expectation": f"graphify package {GRAPHIFY_REF}, generated_files=4, timeout_seconds={TIMEOUT_SECONDS}, query_budget={QUERY_BUDGET}.",
                    "retry_guidance": [
                        "Rerun cargo make smoke-graphify-docker-graph-report after dependency or runtime fixes.",
                        "Do not use graphify install hooks, host-global Codex/Claude/Gemini config, or private corpora as proof.",
                        "Score only when graph.json, GRAPH_REPORT.md, and graphify query output map to generated evidence ids.",
                    ],
                    "research_depth": "D1 feasibility plus XY-889 Docker graph/report smoke implementation; generated artifact decides live evidence class.",
                },
                "notes": [
                    "The checked-in manifest carries the current graphify status; generated smoke artifacts carry the run-specific live status.",
                    "graphify output is treated as a derived graph/report adapter, not an authoritative ELF memory store.",
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
            "schema": "elf.graphify_docker_smoke_summary/v1",
            "generated_at": utc_now(),
            "adapter_id": "graphify_docker_smoke",
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
