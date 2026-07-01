from __future__ import annotations

import time
from pathlib import Path
from typing import Any

from .benchmark import scored_benchmark
from .common import dir_size, file_count, rel, utc_now, write_json
from .context import API_BASE, API_KEY, CHAT_MODEL, EMBEDDING_MODEL, FIXTURE_DIR, GRAPH_RAG_PACKAGE, GRAPH_RAG_REF, INDEX_METHOD, MANIFEST_OUT, MAX_DOCS, MAX_INPUT_CHARS, OUT, OUTPUT_CAPTURE_DIR, QUERY_METHOD, REPORT_JSON, REPORT_MD, RUN_ID, RUN_LIVE, SUMMARY_OUT, TIMEOUT_SECONDS, WORK_DIR
from .models import CommandRecord, StatusState
from .runtime import command_to_json



def write_fixture(corpus: list[dict[str, str]], status: StatusState, mapped_ids: list[str]) -> Path:
    """Write a generated real_world_job fixture for the smoke."""

    fixture_path = FIXTURE_DIR / "knowledge" / "graphrag_tiny_corpus.json"
    expected_ids = [item["evidence_id"] for item in corpus if item["evidence_id"] != "graphrag-smoke-stale-trap"]
    used_ids = [item for item in mapped_ids if item in expected_ids]
    stale_trap_ids = [
        item["evidence_id"] for item in corpus if item["evidence_id"] == "graphrag-smoke-stale-trap"
    ]
    response = {
        "adapter_id": "graphrag_docker_smoke",
        "answer": {
            "content": (
                "Nova Observatory and the Aurora Index are connected by calibration "
                "and public skyglow review evidence."
                if used_ids
                else ""
            ),
            "claims": [
                {
                    "claim_id": "nova_aurora_link",
                    "text": (
                        "Nova Observatory and the Aurora Index are connected by "
                        "calibration and public skyglow review evidence."
                    ),
                    "evidence_ids": used_ids,
                    "confidence": "derived_from_graphrag_table_mapping",
                }
            ]
            if used_ids
            else [],
            "evidence_ids": used_ids,
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
        "job_id": "graphrag-tiny-corpus-001",
        "suite": "knowledge_compilation",
        "title": "Map GraphRAG output tables to generated evidence",
        "corpus": {
            "corpus_id": "graphrag-generated-public-smoke",
            "profile": "generated_public",
            "items": [
                {
                    "evidence_id": item["evidence_id"],
                    "kind": "document",
                    "text": item["text"],
                    "source_ref": {
                        "schema": "source_ref/v1",
                        "resolver": "graphrag_smoke/v1",
                        "ref": {
                            "run_id": RUN_ID,
                            "evidence_id": item["evidence_id"],
                            "title": item["title"],
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
                "event_id": "graphrag-smoke-corpus-generated",
                "ts": "2026-06-10T00:00:00Z",
                "actor": "system",
                "action": "generated_public_corpus",
                "evidence_ids": expected_ids,
                "summary": "The GraphRAG smoke generated a tiny public corpus for source mapping.",
            }
        ],
        "prompt": {
            "role": "user",
            "content": "What connects Nova Observatory and the Aurora Index in the generated corpus?",
            "job_mode": "compile",
            "constraints": ["cite_evidence", "avoid_stale_facts"],
        },
        "expected_answer": {
            "must_include": [
                {
                    "claim_id": "nova_aurora_link",
                    "text": (
                        "Nova Observatory and the Aurora Index are connected by "
                        "calibration and public skyglow review evidence."
                    ),
                }
            ],
            "must_not_include": ["Zenith Ledger is the current source."],
            "evidence_links": {"nova_aurora_link": expected_ids},
            "answer_type": "direct_answer",
            "accepted_alternates": [],
            "requires_caveat": False,
            "requires_refusal": False,
        },
        "required_evidence": [
            {
                "evidence_id": evidence_id,
                "claim_id": "nova_aurora_link",
                "requirement": "cite",
                "quote": "Aurora Index",
            }
            for evidence_id in expected_ids
        ],
        "negative_traps": [
            {
                "trap_id": "retired-zenith-ledger",
                "type": "stale_fact",
                "evidence_ids": stale_trap_ids,
                "failure_if_used": True,
            }
        ]
        if stale_trap_ids
        else [],
        "scoring_rubric": {
            "dimensions": {
                "answer_correctness": {
                    "weight": 0.35,
                    "max_points": 1.0,
                    "criteria": "States the Nova Observatory and Aurora Index relationship.",
                },
                "evidence_grounding": {
                    "weight": 0.35,
                    "max_points": 1.0,
                    "criteria": "Maps output table identifiers to generated evidence ids.",
                },
                "trap_avoidance": {
                    "weight": 0.2,
                    "max_points": 1.0,
                    "criteria": "Does not use the retired Zenith Ledger distractor.",
                },
                "uncertainty": {
                    "weight": 0.1,
                    "max_points": 1.0,
                    "criteria": "Does not claim broad GraphRAG quality from the tiny smoke.",
                },
            },
            "pass_threshold": 0.75,
            "hard_fail_rules": [],
        },
        "allowed_uncertainty": {
            "can_answer_unknown": False,
            "acceptable_phrases": ["tiny generated corpus", "smoke only"],
            "fallback_action": "state_blocker",
        },
        "operator_debug": None,
        "encoding": {},
        "memory_evolution": None,
        "tags": ["external_adapter", "generated_public", "no_live_claim"],
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
    corpus: list[dict[str, str]],
    fixture_path: Path,
    corpus_csv: Path,
    command_records: list[CommandRecord],
    mappings: list[dict[str, Any]],
    mapped_ids: list[str],
    started_at: float,
    report: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Write the primary smoke artifact."""

    cache_dir = WORK_DIR / "project" / "cache"
    output_dir = WORK_DIR / "project" / "output"
    elapsed_ms = (time.monotonic() - started_at) * 1000
    expected_ids = [item["evidence_id"] for item in corpus if item["evidence_id"] != "graphrag-smoke-stale-trap"]
    payload = {
        "schema": "elf.graphrag_docker_smoke/v1",
        "generated_at": utc_now(),
        "run_id": RUN_ID,
        "adapter_id": "graphrag_docker_smoke",
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
            "generated_fixture": rel(fixture_path),
            "graph_output_dir": rel(OUTPUT_CAPTURE_DIR),
            "manifest": rel(MANIFEST_OUT),
            "summary": rel(SUMMARY_OUT),
            "scored_report_json": rel(REPORT_JSON),
            "scored_report_markdown": rel(REPORT_MD),
        },
        "docker_boundary": {
            "compose_file": "docker-compose.baseline.yml",
            "runner_service": "baseline-runner",
            "runner": "scripts/graphrag-docker-smoke.py",
            "host_global_installs_required": False,
            "docker_only": True,
        },
        "provider_configuration": {
            "package": GRAPH_RAG_REF,
            "package_spec": GRAPH_RAG_PACKAGE,
            "chat_model": CHAT_MODEL,
            "embedding_model": EMBEDDING_MODEL,
            "api_base_configured": bool(API_BASE),
            "api_key_provided": bool(API_KEY),
            "operator_owned_provider_credentials_used": False,
            "index_method": INDEX_METHOD,
            "query_method": QUERY_METHOD,
            "live_run_enabled": RUN_LIVE,
        },
        "resource_bounds": {
            "max_docs": MAX_DOCS,
            "max_input_chars": MAX_INPUT_CHARS,
            "actual_doc_count": len(corpus),
            "actual_input_chars": sum(len(item["text"]) for item in corpus),
            "timeout_seconds": TIMEOUT_SECONDS,
            "elapsed_ms": round(elapsed_ms, 3),
            "cache_size_bytes": dir_size(cache_dir),
            "cache_file_count": file_count(cache_dir),
            "output_size_bytes": dir_size(output_dir),
            "captured_output_size_bytes": dir_size(OUTPUT_CAPTURE_DIR),
            "model_call_observation": {
                "source": "GraphRAG cache artifact count when available",
                "observed_cache_entries": file_count(cache_dir),
                "raw_provider_usage_tokens_recorded": False,
            },
        },
        "commands": [command_to_json(record) for record in command_records],
        "evidence_mapping": {
            "expected_evidence_ids": expected_ids,
            "mapped_evidence_ids": mapped_ids,
            "tables": mappings,
        },
    }
    write_json(OUT, payload)

    return payload


def write_manifest(status: StatusState) -> dict[str, Any]:
    """Write a generated external adapter manifest for this smoke."""

    manifest = {
        "schema": "elf.real_world_external_adapter_manifest/v1",
        "manifest_id": f"graphrag-docker-smoke-{RUN_ID}",
        "docker_isolation": {
            "default": True,
            "compose_file": "docker-compose.baseline.yml",
            "runner": "scripts/graphrag-docker-smoke.py",
            "artifact_dir": "tmp/real-world-memory/graphrag-smoke",
            "host_global_installs_required": False,
            "notes": [
                f"Generated by the GraphRAG Docker smoke at {utc_now()}.",
                "The smoke uses a generated public corpus and records typed setup/runtime failures.",
            ],
        },
        "adapters": [
            {
                "adapter_id": "graphrag_docker_smoke",
                "project": "GraphRAG",
                "adapter_kind": "docker_python_cli_api_smoke",
                "evidence_class": status.evidence_class,
                "docker_default": True,
                "host_global_installs_required": False,
                "overall_status": status.overall,
                "setup": {
                    "status": status.setup,
                    "evidence": "The smoke runs inside the baseline Docker runner and installs or invokes GraphRAG only in the container-local work directory.",
                    "command": "cargo make smoke-graphrag-docker",
                    "artifact": rel(OUT),
                },
                "run": {
                    "status": status.run,
                    "evidence": "The live path generates a tiny public corpus, initializes GraphRAG, indexes with bounded inputs, and runs local search when provider config is supplied.",
                    "command": "ELF_GRAPHRAG_SMOKE_RUN=1 cargo make smoke-graphrag-docker",
                    "artifact": rel(OUT),
                },
                "result": {
                    "status": status.result,
                    "evidence": status.failure_reason
                    if status.failure_reason
                    else "GraphRAG parquet output tables mapped to generated real_world_job evidence ids.",
                    "artifact": rel(OUT),
                },
                "capabilities": [
                    {
                        "capability": "docker_python_cli_boundary",
                        "status": status.setup,
                        "evidence": "The runner is Python-only inside docker-compose.baseline.yml baseline-runner and does not require host-global GraphRAG installs.",
                    },
                    {
                        "capability": "graphrag_index_query",
                        "status": status.run,
                        "evidence": "The opt-in live path runs GraphRAG index and local query over the generated public corpus.",
                    },
                    {
                        "capability": "parquet_table_evidence_mapping",
                        "status": status.result,
                        "evidence": "documents, text_units, communities, community_reports, entities, and relationships parquet table identifiers are mapped to evidence ids when available.",
                    },
                    {
                        "capability": "quality_or_scale_claim",
                        "status": "not_encoded",
                        "evidence": "The smoke does not claim graph-navigation quality, synthesis quality, private-corpus behavior, or large-corpus indexing.",
                    },
                ],
                "suites": [
                    {
                        "suite_id": "knowledge_compilation",
                        "status": status.result,
                        "evidence": "Only the generated tiny-corpus table-mapping job is represented.",
                    },
                    {
                        "suite_id": "retrieval",
                        "status": status.run if status.run != "pass" else "not_encoded",
                        "evidence": "The smoke may run local search for reachability, but retrieval quality scoring is not encoded.",
                    },
                    {
                        "suite_id": "production_ops",
                        "status": "not_encoded",
                        "evidence": "The smoke records resource bounds but does not encode backup, restore, provider credential, or private corpus production-ops checks.",
                    },
                    {
                        "suite_id": "memory_evolution",
                        "status": "not_encoded",
                        "evidence": "GraphRAG update/delete/current-versus-historical behavior is not encoded by this smoke.",
                    },
                ],
                "evidence": [
                    {"kind": "artifact", "ref": rel(OUT), "status": status.result},
                    {"kind": "artifact", "ref": rel(OUTPUT_CAPTURE_DIR), "status": status.result},
                    {"kind": "manifest", "ref": rel(MANIFEST_OUT), "status": status.overall},
                    {"kind": "source", "ref": "https://github.com/microsoft/graphrag", "status": "real"},
                    {"kind": "source", "ref": "https://microsoft.github.io/graphrag/", "status": "real"},
                    {
                        "kind": "source",
                        "ref": "https://microsoft.github.io/graphrag/index/outputs/",
                        "status": "real",
                    },
                ],
                "execution_metadata": {
                    "sources": [
                        {
                            "label": "GraphRAG repository",
                            "url": "https://github.com/microsoft/graphrag",
                            "evidence": "Official source and package for GraphRAG.",
                        },
                        {
                            "label": "GraphRAG CLI docs",
                            "url": "https://microsoft.github.io/graphrag/cli/",
                            "evidence": "Official index and query command contract.",
                        },
                        {
                            "label": "GraphRAG input docs",
                            "url": "https://microsoft.github.io/graphrag/index/inputs/",
                            "evidence": "Official input formats and document schema.",
                        },
                        {
                            "label": "GraphRAG output tables",
                            "url": "https://microsoft.github.io/graphrag/index/outputs/",
                            "evidence": "Official parquet output table schema for evidence mapping.",
                        },
                        {
                            "label": "GraphRAG local search docs",
                            "url": "https://microsoft.github.io/graphrag/query/local_search/",
                            "evidence": "Official local-search context and graph traversal reference.",
                        },
                    ],
                    "setup_path": "Run cargo make smoke-graphrag-docker for a typed artifact; set ELF_GRAPHRAG_SMOKE_RUN=1 with explicit provider configuration for a live index/query attempt.",
                    "runtime_boundary": "docker-compose.baseline.yml baseline-runner, container-local Python venv, generated public corpus, and report artifacts under tmp/real-world-memory/graphrag-smoke.",
                    "resource_expectation": f"GraphRAG package {GRAPH_RAG_REF}, max_docs={MAX_DOCS}, max_input_chars={MAX_INPUT_CHARS}, timeout_seconds={TIMEOUT_SECONDS}, index_method={INDEX_METHOD}.",
                    "retry_guidance": [
                        "Default command records a typed blocked artifact without model calls.",
                        "Enable the live path only with explicit provider configuration and generated public corpus.",
                        "Treat missing or unmapped documents/text_units as wrong_result, not as pass.",
                    ],
                    "research_depth": "D2 feasibility plus XY-887 cost-bounded Docker smoke implementation; generated artifact decides live evidence class.",
                },
                "notes": [
                    "The checked-in manifest record remains research_gate; generated smoke artifacts carry live status.",
                    "Failure before GraphRAG output remains typed as blocked or incomplete.",
                    "The smoke does not use private corpora or unrecorded provider credentials.",
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
            "schema": "elf.graphrag_docker_smoke_summary/v1",
            "generated_at": utc_now(),
            "adapter_id": "graphrag_docker_smoke",
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
                "status_source": "external_adapter_manifest_pre_score",
                "summary": manifest["adapters"][0]["overall_status"],
                "suites": manifest["adapters"][0]["suites"],
            },
            "report": report,
        },
    )


def scrub_report_secrets(project_dir: Path) -> None:
    """Remove provider secrets from text artifacts before reporting."""

    if not API_KEY:
        return

    for root in (project_dir, LOG_DIR):
        if not root.exists():
            continue

        for path in root.rglob("*"):
            if not path.is_file() or path.suffix not in {".env", ".json", ".log", ".txt", ".yaml", ".yml"}:
                continue

            try:
                content = path.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                continue

            if API_KEY in content:
                path.write_text(content.replace(API_KEY, "<redacted>"), encoding="utf-8")
