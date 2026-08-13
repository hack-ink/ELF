#!/usr/bin/env python3
"""Run one cold/warm benchmark unit inside an isolated Compose project."""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

from benchmark_targets.pageindex import PageIndexProductFailure, run_pageindex


ROOT = Path("/benchmark")
INPUT = ROOT / "input"
ARTIFACTS = ROOT / "artifacts"
STATE = ROOT / "state"
ADAPTER = Path("/usr/local/bin/real_world_live_adapter")
QMD_REVISION = "e428df76bc0274d9e93eb7ca3e95673315c42e90"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--target",
        required=True,
        choices=(
            "elf",
            "mem0",
            "qmd",
            "lightrag",
            "openviking",
            "graphiti",
            "graphrag",
            "letta",
            "sag",
            "pageindex",
            "openkb",
            "honcho",
        ),
    )
    return parser.parse_args()


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, default=str, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def wait_port(host: str, port: int, timeout: float = 90.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            with socket.create_connection((host, port), timeout=2):
                return
        except OSError:
            time.sleep(1)
    raise RuntimeError(f"service did not become ready at {host}:{port}")


def sanitized_error(error: BaseException) -> str:
    message = str(error)
    for name in ("EMBEDDING_API_KEY", "LITELLM_API_KEY", "CHAT_API_KEY"):
        value = os.environ.get(name)
        if value:
            message = message.replace(value, "[redacted]")
    return message


def classify_failure(message: str) -> str:
    lowered = message.lower()
    if re.search(r"(?:provider http|error code|http status|status):\s*[45]\d\d", lowered):
        return "provider_failed"
    if any(token in lowered for token in ("401", "403", "429", "rate limit", "quota exhausted")):
        return "provider_failed"
    if any(
        token in lowered
        for token in ("provider api", "provider request", "provider error", "api key")
    ):
        return "provider_failed"
    return "adapter_failed"


def run_command(command: list[str], log_path: Path, env: dict[str, str]) -> float:
    started = time.monotonic()
    completed = subprocess.run(
        command,
        check=False,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.write_text(completed.stdout, encoding="utf-8")
    if completed.returncode:
        output = completed.stdout[-6000:]
        raise RuntimeError(
            f"adapter exited {completed.returncode} after "
            f"{time.monotonic() - started:.3f}s; output follows:\n{output}"
        )
    return (time.monotonic() - started) * 1000.0


def native_operation_receipts(
    target: str, phase: str, fixture: dict[str, Any] | None
) -> list[dict[str, Any]]:
    if phase != "warm" or fixture is None:
        return []
    native_types = {
        "elf": {"update": "update", "delete": "delete"},
        "qmd": {"update": "reindex_update", "delete": "delete"},
    }
    mapping = native_types.get(target)
    if mapping is None:
        return []
    return [
        {
            "requested_type": operation["type"],
            "native_type": mapping[operation["type"]],
            "classification": "completed",
            "native_success": True,
        }
        for operation in fixture.get("operations") or []
    ]


def normalize_rust_phase(
    evidence_path: Path,
    *,
    target: str,
    phase: str,
    fixtures: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
    jobs = []
    phase_status = "completed"
    for job in evidence["jobs"]:
        failure = job.get("failure")
        classification = classify_failure(failure) if failure else "completed"
        if classification != "completed":
            phase_status = classification
        row = {
            "job_id": job["job_id"],
            "classification": classification,
            "evidence_ids": list(job.get("evidence_ids") or []),
            "returned_count": int(job.get("returned_count") or 0),
            "latency_ms": float(job.get("latency_ms") or 0.0),
            "native_status": job.get("status"),
            "failure": failure,
            "operations": native_operation_receipts(
                target, phase, fixtures.get(job["job_id"])
            ) if classification == "completed" else [],
        }
        if "contexts" in job:
            row["contexts"] = job["contexts"]
        jobs.append(row)
    return {
        "status": phase_status,
        "jobs": jobs,
        "adapter_metadata": evidence.get("metadata") or {},
    }


def skipped_warm_phase(cold: dict[str, Any]) -> dict[str, Any]:
    classification = cold["status"]
    return {
        "status": classification,
        "jobs": [
            {
                "job_id": job["job_id"],
                "classification": classification,
                "evidence_ids": [],
                "returned_count": 0,
                "latency_ms": 0.0,
                "native_status": "not_run",
                "failure": f"warm phase not run because cold ended as {classification}",
                "operations": [],
            }
            for job in cold["jobs"]
        ],
        "adapter_metadata": {
            "index_reused": False,
            "skipped_due_to_cold": True,
        },
    }


def run_rust_target(target: str) -> dict[str, Any]:
    if target == "elf":
        wait_port("postgres", 5432)
        wait_port("qdrant", 6334)
    elif target == "lightrag":
        wait_port("lightrag", 9621)
    state_dir = STATE / target
    state_dir.mkdir(parents=True, exist_ok=True)
    phases: dict[str, Any] = {}
    fixtures = {
        job["job_id"]: job
        for job in (
            json.loads(path.read_text(encoding="utf-8")) for path in fixture_paths()
        )
    }
    ingest_duration_ms = 0.0
    env = os.environ.copy()
    if target == "elf":
        env["ELF_REAL_WORLD_EXTERNAL_EMBEDDING"] = "1"

    for phase in ("cold", "warm"):
        if phase == "warm" and phases["cold"]["status"] != "completed":
            phases["warm"] = skipped_warm_phase(phases["cold"])
            break
        raw_dir = ARTIFACTS / "raw" / phase
        evidence_path = raw_dir / "evidence.json"
        command = [
            str(ADAPTER),
            target,
            "--fixtures",
            str(INPUT),
            "--out-fixtures",
            str(raw_dir / "fixtures"),
            "--evidence-out",
            str(evidence_path),
            "--work-dir",
            str(state_dir),
            "--adapter-id",
            f"{target}-competitor-benchmark",
        ]
        if target == "elf":
            command.extend(("--config", "/opt/elf/elf.docker.toml"))
        elif target == "qmd":
            command.extend(
                (
                    "--qmd-dir",
                    "/opt/qmd",
                    "--qmd-revision",
                    QMD_REVISION,
                    "--lexical-only",
                )
            )
        else:
            command.extend(("--api-base", "http://lightrag:9621"))
        if phase == "warm":
            command.append("--reuse-index")
        duration_ms = run_command(command, raw_dir / "adapter.log", env)
        phases[phase] = normalize_rust_phase(
            evidence_path,
            target=target,
            phase=phase,
            fixtures=fixtures,
        )
        if phase == "cold":
            native_query_ms = sum(
                float(row.get("latency_ms") or 0.0) for row in phases[phase]["jobs"]
            )
            ingest_duration_ms = max(0.0, duration_ms - native_query_ms)

    if target == "lightrag" and (state_dir / "native").is_dir():
        shutil.copytree(
            state_dir / "native",
            ARTIFACTS / "raw" / "lightrag-native",
            dirs_exist_ok=True,
        )

    warm_reused = (
        phases["warm"]["status"] == "completed"
        and phases["warm"]["adapter_metadata"].get("index_reused") is True
    )
    return {
        "schema": "elf.benchmark_unit_result/v4",
        "target": target,
        "native_mode": "lexical" if target == "qmd" else "external_embedding",
        "score_eligible": True,
        "result_class": _combined_status(phases),
        "warm_reused_state": warm_reused,
        "ingest_count": 1,
        "ingest_duration_ms": round(ingest_duration_ms, 3),
        "ingest_duration_measurement": "cold adapter wall time minus native query latency",
        "phases": phases,
    }


def fixture_paths() -> list[Path]:
    return sorted(INPUT.glob("*.json"))


def mem0_entries(value: Any) -> list[dict[str, Any]]:
    if isinstance(value, dict):
        for key in ("results", "memories"):
            rows = value.get(key)
            if isinstance(rows, list):
                return [row for row in rows if isinstance(row, dict)]
    if isinstance(value, list):
        return [row for row in value if isinstance(row, dict)]
    return []


def mapped_evidence(entries: list[dict[str, Any]]) -> list[str]:
    output: list[str] = []
    for entry in entries:
        metadata = entry.get("metadata")
        evidence_id = metadata.get("evidence_id") if isinstance(metadata, dict) else None
        if isinstance(evidence_id, str) and evidence_id not in output:
            output.append(evidence_id)
    return output


def mem0_contexts(entries: list[dict[str, Any]]) -> list[dict[str, Any]]:
    output = []
    for entry in entries:
        text = entry.get("memory")
        if not isinstance(text, str):
            continue
        metadata = entry.get("metadata")
        evidence_id = metadata.get("evidence_id") if isinstance(metadata, dict) else None
        output.append({"evidence_id": evidence_id, "text": text})
    return output


def created_memory_ids(value: Any) -> list[str]:
    output: list[str] = []
    for entry in mem0_entries(value):
        memory_id = entry.get("id") or entry.get("memory_id")
        if isinstance(memory_id, str) and memory_id not in output:
            output.append(memory_id)
    return output


def mem0_config(job_dir: Path, collection: str) -> dict[str, Any]:
    return {
        "vector_store": {
            "provider": "qdrant",
            "config": {
                "collection_name": collection,
                "host": "qdrant",
                "port": 6333,
                "embedding_model_dims": 4096,
            },
        },
        "embedder": {
            "provider": "openai",
            "config": {
                "model": os.environ["EMBEDDING_MODEL"],
                "embedding_dims": int(os.environ["EMBEDDING_DIMENSIONS"]),
                "api_key": os.environ["EMBEDDING_API_KEY"],
                "openai_base_url": os.environ["EMBEDDING_API_BASE"],
            },
        },
        "llm": {
            "provider": "openai",
            "config": {
                "model": os.environ["CHAT_MODEL"],
                "api_key": os.environ["CHAT_API_KEY"],
                "openai_base_url": os.environ["CHAT_API_BASE"],
                "max_tokens": 4096,
            },
        },
        "history_db_path": str(job_dir / "history.db"),
        "version": "v1.1",
    }


def run_mem0() -> dict[str, Any]:
    os.environ.setdefault("MEM0_TELEMETRY", "false")
    wait_port("qdrant", 6333)
    from mem0 import Memory

    loaded = [json.loads(path.read_text(encoding="utf-8")) for path in fixture_paths()]
    phases: dict[str, Any] = {}
    cold_ingest_duration_ms = 0.0
    for phase in ("cold", "warm"):
        rows = []
        raw_rows = []
        for job in loaded:
            job_dir = STATE / "mem0" / job["job_id"]
            job_dir.mkdir(parents=True, exist_ok=True)
            receipt = job_dir / "cold-ingest-complete"
            if phase == "warm" and not receipt.is_file():
                raise RuntimeError(f"warm mem0 state is missing for {job['job_id']}")
            memory = Memory.from_config(
                mem0_config(job_dir, f"elf-proof-{job['job_id']}")
            )
            user_id = f"elf-proof-{job['job_id']}"
            adds = []
            operation_results = []
            if phase == "cold":
                ingest_started = time.monotonic()
                memory_ids: dict[str, str] = {}
                for item in job["corpus"]["items"]:
                    add_result = memory.add(
                        item["text"],
                        user_id=user_id,
                        metadata={"evidence_id": item["evidence_id"]},
                        infer=False,
                    )
                    adds.append(add_result)
                    ids = created_memory_ids(add_result)
                    if not ids:
                        raise RuntimeError(
                            f"mem0 add returned no memory id for {item['evidence_id']}"
                        )
                    memory_ids[item["evidence_id"]] = ids[0]
                cold_ingest_duration_ms += (
                    time.monotonic() - ingest_started
                ) * 1000.0
                write_json(receipt, {"memory_ids": memory_ids})
            else:
                receipt_value = json.loads(receipt.read_text(encoding="utf-8"))
                memory_ids = receipt_value.get("memory_ids") or {}
                for operation in job.get("operations") or []:
                    memory_id = memory_ids.get(operation["evidence_id"])
                    if not isinstance(memory_id, str):
                        raise RuntimeError(
                            f"mem0 operation has no native id for {operation['evidence_id']}"
                        )
                    if operation["type"] == "update":
                        native = memory.update(
                            memory_id,
                            operation["text"],
                            metadata={"evidence_id": operation["evidence_id"]},
                        )
                        native_type = "update"
                    elif operation["type"] == "delete":
                        native = memory.delete(memory_id)
                        native_type = "delete"
                    else:
                        raise RuntimeError(
                            f"unsupported mem0 operation {operation['type']}"
                        )
                    operation_results.append(
                        {
                            "requested_type": operation["type"],
                            "native_type": native_type,
                            "classification": "completed",
                            "native_success": True,
                            "native_result": native,
                        }
                    )
            started = time.monotonic()
            search = memory.search(
                job["prompt"]["content"],
                filters={"user_id": user_id},
                top_k=5,
                threshold=0.0,
            )
            latency_ms = (time.monotonic() - started) * 1000.0
            entries = mem0_entries(search)
            evidence_ids = mapped_evidence(entries)
            rows.append(
                {
                    "job_id": job["job_id"],
                    "classification": "completed",
                    "evidence_ids": evidence_ids,
                    "contexts": mem0_contexts(entries),
                    "returned_count": len(entries),
                    "latency_ms": latency_ms,
                    "native_status": "completed",
                    "failure": None,
                    "operations": operation_results,
                }
            )
            raw_rows.append(
                {
                    "job_id": job["job_id"],
                    "add_results": adds,
                    "search": search,
                }
            )
        write_json(ARTIFACTS / "raw" / f"mem0-{phase}.json", raw_rows)
        phases[phase] = {
            "status": "completed",
            "jobs": rows,
            "adapter_metadata": {"index_reused": phase == "warm"},
        }

    return {
        "schema": "elf.benchmark_unit_result/v4",
        "target": "mem0",
        "native_mode": "external_embedding",
        "score_eligible": True,
        "result_class": _combined_status(phases),
        "warm_reused_state": True,
        "ingest_count": 1,
        "ingest_duration_ms": round(cold_ingest_duration_ms, 3),
        "phases": phases,
    }


def _combined_status(phases: dict[str, Any]) -> str:
    for phase in ("cold", "warm"):
        if phases[phase]["status"] != "completed":
            return phases[phase]["status"]
    return "completed"


def failure_result(
    target: str,
    classification: str,
    error: BaseException,
    job_ids: list[str] | None = None,
) -> dict[str, Any]:
    score_eligible = target != "pageindex"
    native_mode = {
        "qmd": "lexical",
        "pageindex": "hierarchical_tree_construction",
        "openviking": "native_resource_find",
        "graphiti": "native_temporal_graph_search",
        "graphrag": "native_local_search",
        "letta": "native_archival_search",
        "sag": "native_multi_search_source_id_trace",
        "openkb": "agent_query_with_native_source_trace",
        "honcho": "native_hybrid_message_search",
    }.get(target, "external_embedding")
    job_ids = job_ids or [path.stem for path in sorted(INPUT.glob("*.json"))]
    phases = {
        phase: {
            "status": classification,
            "jobs": [
                {
                    "job_id": job_id,
                    "classification": classification,
                    "evidence_ids": [],
                    "returned_count": 0,
                    "latency_ms": 0.0,
                    "native_status": "not_run",
                    "failure": sanitized_error(error),
                    "operations": [],
                }
                for job_id in job_ids
            ],
            "adapter_metadata": {"skipped_due_to_failure": True},
        }
        for phase in ("cold", "warm")
    }
    return {
        "schema": "elf.benchmark_unit_result/v4",
        "target": target,
        "native_mode": native_mode,
        "score_eligible": score_eligible,
        "result_class": classification,
        "warm_reused_state": False,
        "ingest_count": 0,
        "failure": {
            "classification": classification,
            "message": sanitized_error(error),
        },
        "phases": phases,
    }


def main() -> int:
    args = parse_args()
    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    STATE.mkdir(parents=True, exist_ok=True)
    job_ids = [path.stem for path in sorted(INPUT.glob("*.json"))]
    try:
        required: tuple[str, ...] = ()
        if args.target in {
            "elf",
            "mem0",
            "lightrag",
            "openviking",
            "graphiti",
            "graphrag",
            "letta",
            "sag",
            "honcho",
        }:
            required += (
                "EMBEDDING_API_BASE",
                "EMBEDDING_API_KEY",
                "EMBEDDING_MODEL",
                "EMBEDDING_DIMENSIONS",
            )
        if args.target in {
            "mem0",
            "lightrag",
            "graphiti",
            "letta",
            "sag",
            "openkb",
            "honcho",
        }:
            required += (
                "CHAT_API_BASE",
                "CHAT_API_KEY",
                "CHAT_MODEL",
                "CHAT_REASONING_EFFORT",
            )
        missing = [name for name in required if not os.environ.get(name)]
        if missing:
            raise KeyError(f"missing required configuration: {', '.join(missing)}")
        exit_code = 0
        if args.target == "mem0":
            result = run_mem0()
        elif args.target == "openviking":
            from benchmark_targets.openviking import (
                OpenVikingAdapterFailure,
                OpenVikingProductFailure,
                run_openviking,
            )

            try:
                result = run_openviking(INPUT, ARTIFACTS, STATE / "openviking")
            except OpenVikingProductFailure as error:
                result = failure_result(args.target, "product_failed", error, job_ids)
                exit_code = 1
            except OpenVikingAdapterFailure as error:
                result = failure_result(args.target, "adapter_failed", error, job_ids)
                exit_code = 1
        elif args.target == "graphiti":
            from benchmark_targets.graphiti import (
                GraphitiAdapterFailure,
                GraphitiProductFailure,
                run_graphiti,
            )

            try:
                result = run_graphiti(INPUT, ARTIFACTS, STATE / "graphiti")
            except GraphitiProductFailure as error:
                result = failure_result(args.target, "product_failed", error, job_ids)
                exit_code = 1
            except GraphitiAdapterFailure as error:
                result = failure_result(args.target, "adapter_failed", error, job_ids)
                exit_code = 1
        elif args.target == "graphrag":
            from benchmark_targets.graphrag import (
                GraphRAGAdapterFailure,
                GraphRAGProductFailure,
                run_graphrag,
            )

            try:
                result = run_graphrag(INPUT, ARTIFACTS, STATE / "graphrag")
            except GraphRAGProductFailure as error:
                result = failure_result(args.target, "product_failed", error, job_ids)
                exit_code = 1
            except GraphRAGAdapterFailure as error:
                result = failure_result(args.target, "adapter_failed", error, job_ids)
                exit_code = 1
        elif args.target == "letta":
            from benchmark_targets.letta import (
                LettaAdapterFailure,
                LettaProductFailure,
                run_letta,
            )

            try:
                result = run_letta(INPUT, ARTIFACTS, STATE / "letta")
            except LettaProductFailure as error:
                result = failure_result(args.target, "product_failed", error, job_ids)
                exit_code = 1
            except LettaAdapterFailure as error:
                result = failure_result(args.target, "adapter_failed", error, job_ids)
                exit_code = 1
        elif args.target == "sag":
            from benchmark_targets.sag import run_sag

            result = run_sag(INPUT, ARTIFACTS, STATE / "sag")
        elif args.target == "pageindex":
            result = run_pageindex(INPUT, ARTIFACTS, STATE / "pageindex")
        elif args.target == "openkb":
            from benchmark_targets.openkb import run_openkb

            result = run_openkb(INPUT, ARTIFACTS, STATE / "openkb")
        elif args.target == "honcho":
            from benchmark_targets.honcho import (
                HonchoAdapterFailure,
                HonchoProductFailure,
                run_honcho,
            )

            try:
                result = run_honcho(INPUT, ARTIFACTS, STATE / "honcho")
            except HonchoProductFailure as error:
                result = failure_result(args.target, "product_failed", error, job_ids)
                exit_code = 1
            except HonchoAdapterFailure as error:
                result = failure_result(args.target, "adapter_failed", error, job_ids)
                exit_code = 1
        else:
            result = run_rust_target(args.target)
    except KeyError as error:
        result = failure_result(args.target, "configuration_failed", error, job_ids)
        exit_code = 2
    except PageIndexProductFailure as error:
        result = failure_result(args.target, "product_failed", error, job_ids)
        exit_code = 1
    except Exception as error:
        classification = classify_failure(sanitized_error(error))
        result = failure_result(args.target, classification, error, job_ids)
        exit_code = 1
    write_json(ARTIFACTS / "unit-result.json", result)
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
