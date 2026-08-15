"""GraphRAG local-search benchmark adapter for the pinned 3.1.0 API."""

from __future__ import annotations

import asyncio
import dataclasses
import hashlib
import importlib.metadata
import json
import os
import re
import subprocess
import time
from enum import Enum
from pathlib import Path
from typing import Any


GRAPHRAG_VERSION = "3.1.0"
GRAPHRAG_EXECUTABLE = Path(
    os.environ.get("GRAPHRAG_EXECUTABLE", "/opt/graphrag-venv/bin/graphrag")
)
CHAT_MODEL = "gpt-5.6-luna"
CHAT_REASONING_EFFORT = "high"
EMBEDDING_MODEL = "Qwen3-Embedding-8B"
EMBEDDING_DIMENSIONS = 4096
TOP_K = 5
FORBIDDEN_FIXTURE_KEYS = {
    "expected_answer",
    "negative_traps",
    "qrels",
    "required_evidence",
    "scoring_rubric",
}
REQUIRED_TABLES = (
    "communities",
    "community_reports",
    "documents",
    "entities",
    "relationships",
    "text_units",
)


class GraphRAGProductFailure(RuntimeError):
    """The pinned GraphRAG CLI or API failed."""


class GraphRAGAdapterFailure(RuntimeError):
    """The adapter input, state, or native identity mapping is invalid."""


def _write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def native_json(value: Any) -> Any:
    """Convert native API values without removing structured fields."""
    if dataclasses.is_dataclass(value) and not isinstance(value, type):
        return {
            field.name: native_json(getattr(value, field.name))
            for field in dataclasses.fields(value)
        }
    if isinstance(value, Enum):
        return native_json(value.value)
    if hasattr(value, "model_dump"):
        return native_json(value.model_dump(mode="json"))
    if hasattr(value, "to_dict"):
        try:
            return native_json(value.to_dict(orient="records"))
        except TypeError:
            return native_json(value.to_dict())
    if isinstance(value, dict):
        return {str(key): native_json(child) for key, child in value.items()}
    if isinstance(value, (list, tuple)):
        return [native_json(child) for child in value]
    if isinstance(value, Path):
        return str(value)
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    if hasattr(value, "item"):
        try:
            return native_json(value.item())
        except (TypeError, ValueError):
            pass
    if hasattr(value, "tolist"):
        return native_json(value.tolist())
    return str(value)


def assert_qrel_blind(value: Any, path: str = "$") -> None:
    """Reject evaluator-only fields before GraphRAG observes a fixture."""
    if isinstance(value, dict):
        leaked = FORBIDDEN_FIXTURE_KEYS.intersection(value)
        if leaked:
            raise GraphRAGAdapterFailure(
                f"GraphRAG fixture leaks evaluator keys at {path}: "
                + ", ".join(sorted(leaked))
            )
        for key, child in value.items():
            assert_qrel_blind(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            assert_qrel_blind(child, f"{path}[{index}]")


def _load_jobs(input_dir: Path) -> list[dict[str, Any]]:
    jobs = [
        json.loads(path.read_text(encoding="utf-8"))
        for path in sorted(input_dir.glob("*.json"))
    ]
    if not jobs:
        raise GraphRAGAdapterFailure("GraphRAG received no product fixtures")
    for job in jobs:
        assert_qrel_blind(job)
    return jobs


def _fixture_digest(jobs: list[dict[str, Any]]) -> str:
    encoded = json.dumps(
        jobs, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _job_key(index: int, job_id: str) -> str:
    stem = re.sub(r"[^A-Za-z0-9._-]+", "-", job_id).strip(".-") or "job"
    suffix = hashlib.sha256(job_id.encode("utf-8")).hexdigest()[:12]
    return f"{index:04d}-{stem[:64]}-{suffix}"


def _source_key(evidence_id: str) -> str:
    return "source-" + hashlib.sha256(evidence_id.encode("utf-8")).hexdigest()[:24]


def _required_environment() -> None:
    required = (
        "CHAT_API_BASE",
        "CHAT_API_KEY",
        "CHAT_MODEL",
        "CHAT_REASONING_EFFORT",
        "EMBEDDING_API_BASE",
        "EMBEDDING_API_KEY",
        "EMBEDDING_MODEL",
        "EMBEDDING_DIMENSIONS",
    )
    missing = [name for name in required if not os.environ.get(name)]
    if missing:
        raise GraphRAGAdapterFailure(
            "missing required GraphRAG configuration: " + ", ".join(missing)
        )
    expected = {
        "CHAT_MODEL": CHAT_MODEL,
        "CHAT_REASONING_EFFORT": CHAT_REASONING_EFFORT,
        "EMBEDDING_MODEL": EMBEDDING_MODEL,
        "EMBEDDING_DIMENSIONS": str(EMBEDDING_DIMENSIONS),
    }
    mismatches = [
        f"{name}={os.environ[name]!r}"
        for name, value in expected.items()
        if os.environ[name] != value
    ]
    if mismatches:
        raise GraphRAGAdapterFailure(
            "GraphRAG provider configuration differs from the frozen target: "
            + ", ".join(mismatches)
        )


def _verify_version() -> None:
    try:
        actual = importlib.metadata.version("graphrag")
    except importlib.metadata.PackageNotFoundError as error:
        raise GraphRAGAdapterFailure("the GraphRAG package is not installed") from error
    if actual != GRAPHRAG_VERSION:
        raise GraphRAGAdapterFailure(
            f"GraphRAG {actual} does not match the frozen {GRAPHRAG_VERSION} package"
        )


def _settings(root: Path) -> dict[str, Any]:
    prompts = root / "prompts"
    return {
        "completion_models": {
            "benchmark_chat": {
                "type": "litellm",
                "model_provider": "openai",
                "model": CHAT_MODEL,
                "auth_method": "api_key",
                "api_key": "${CHAT_API_KEY}",
                "api_base": "${CHAT_API_BASE}",
                "call_args": {"reasoning_effort": CHAT_REASONING_EFFORT},
            }
        },
        "embedding_models": {
            "benchmark_embedding": {
                "type": "litellm",
                "model_provider": "openai",
                "model": EMBEDDING_MODEL,
                "auth_method": "api_key",
                "api_key": "${EMBEDDING_API_KEY}",
                "api_base": "${EMBEDDING_API_BASE}",
                "call_args": {
                    "dimensions": EMBEDDING_DIMENSIONS,
                    "allowed_openai_params": ["dimensions"],
                },
            }
        },
        "input": {
            "type": "json",
            "file_pattern": ".*[.]json",
            "id_column": "id",
            "title_column": "title",
            "text_column": "text",
        },
        "chunking": {
            "type": "tokens",
            "size": 1200,
            "overlap": 100,
            "encoding_model": "o200k_base",
        },
        "input_storage": {"type": "file", "base_dir": str(root / "input")},
        "output_storage": {"type": "file", "base_dir": str(root / "output")},
        "reporting": {"type": "file", "base_dir": str(root / "logs")},
        "cache": {
            "type": "json",
            "storage": {"type": "file", "base_dir": str(root / "cache")},
        },
        "vector_store": {
            "type": "lancedb",
            "db_uri": str(root / "output" / "lancedb"),
            "vector_size": EMBEDDING_DIMENSIONS,
        },
        "embed_text": {"embedding_model_id": "benchmark_embedding"},
        "extract_graph": {
            "completion_model_id": "benchmark_chat",
            "prompt": str(prompts / "extract_graph.txt"),
        },
        "summarize_descriptions": {
            "completion_model_id": "benchmark_chat",
            "prompt": str(prompts / "summarize_descriptions.txt"),
        },
        "community_reports": {
            "completion_model_id": "benchmark_chat",
            "graph_prompt": str(prompts / "community_report_graph.txt"),
            "text_prompt": str(prompts / "community_report_text.txt"),
        },
        "local_search": {
            "completion_model_id": "benchmark_chat",
            "embedding_model_id": "benchmark_embedding",
            "prompt": str(prompts / "local_search_system_prompt.txt"),
        },
    }


def _run_cli(
    command: list[str],
    *,
    cwd: Path,
    stdout_path: Path,
    stderr_path: Path,
    timeout: int,
) -> float:
    started = time.monotonic()
    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            env=os.environ.copy(),
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as error:
        stdout_path.parent.mkdir(parents=True, exist_ok=True)
        stdout_path.write_text(error.stdout or "", encoding="utf-8")
        stderr_path.write_text(error.stderr or "", encoding="utf-8")
        raise GraphRAGProductFailure("GraphRAG native operation timed out") from error
    stdout_path.parent.mkdir(parents=True, exist_ok=True)
    stdout_path.write_text(completed.stdout, encoding="utf-8")
    stderr_path.write_text(completed.stderr, encoding="utf-8")
    if completed.returncode:
        raise GraphRAGProductFailure(
            f"GraphRAG native operation exited {completed.returncode}"
        )
    return (time.monotonic() - started) * 1000.0


def _materialize_job(root: Path, job: dict[str, Any]) -> None:
    records = [
        {
            "id": _source_key(item["evidence_id"]),
            "title": "Source document",
            "text": item["text"],
            "source_key": _source_key(item["evidence_id"]),
        }
        for item in job["corpus"]["items"]
    ]
    _write_json(root / "input" / "corpus.json", records)


def _index_job(root: Path, raw_dir: Path, job: dict[str, Any]) -> float:
    root.mkdir(parents=True, exist_ok=True)
    _run_cli(
        [
            str(GRAPHRAG_EXECUTABLE),
            "init",
            "--root",
            str(root),
            "--model",
            CHAT_MODEL,
            "--embedding",
            EMBEDDING_MODEL,
        ],
        cwd=root,
        stdout_path=raw_dir / "init.stdout.log",
        stderr_path=raw_dir / "init.stderr.log",
        timeout=60,
    )
    _write_json(root / "settings.yaml", _settings(root))
    _materialize_job(root, job)
    return _run_cli(
        [
            str(GRAPHRAG_EXECUTABLE),
            "index",
            "--root",
            str(root),
            "--method",
            "standard",
        ],
        cwd=root,
        stdout_path=raw_dir / "index.stdout.log",
        stderr_path=raw_dir / "index.stderr.log",
        timeout=int(os.environ.get("GRAPHRAG_INDEX_TIMEOUT_SECONDS", "1800")),
    )


def _table_records(value: Any) -> list[dict[str, Any]]:
    if value is None:
        return []
    if isinstance(value, list):
        if not all(isinstance(row, dict) for row in value):
            raise GraphRAGAdapterFailure(
                "GraphRAG native table contains non-record rows"
            )
        return value
    if hasattr(value, "to_dict"):
        records = value.to_dict(orient="records")
        if isinstance(records, list):
            return records
    raise GraphRAGAdapterFailure("GraphRAG native table is not record-shaped")


def evidence_ids_from_context(
    context_data: Any,
    text_units: Any,
    documents: Any,
    source_map: dict[str, str],
    *,
    top_k: int = TOP_K,
) -> list[str]:
    """Map only native source, text-unit, and document identity fields."""
    if not isinstance(context_data, dict):
        raise GraphRAGAdapterFailure("GraphRAG local search returned malformed context")
    source_rows = _table_records(context_data.get("sources"))
    text_unit_rows = _table_records(text_units)
    document_rows = _table_records(documents)

    document_evidence: dict[str, str] = {}
    for document in document_rows:
        document_id = document.get("id")
        raw_data = document.get("raw_data")
        source_key = (
            raw_data.get("source_key") if isinstance(raw_data, dict) else None
        )
        if isinstance(document_id, str) and isinstance(source_key, str):
            evidence_id = source_map.get(source_key)
            if evidence_id is not None:
                document_evidence[document_id] = evidence_id

    text_unit_evidence: dict[str, str] = {}
    for index, text_unit in enumerate(text_unit_rows):
        document_id = text_unit.get("document_id")
        evidence_id = document_evidence.get(document_id)
        if evidence_id is not None:
            text_unit_evidence[str(index)] = evidence_id

    evidence_ids: list[str] = []
    for source in source_rows[:top_k]:
        source_id = source.get("id")
        if source_id is None:
            raise GraphRAGAdapterFailure(
                "GraphRAG selected source has no native text-unit identity"
            )
        evidence_id = text_unit_evidence.get(str(source_id))
        if evidence_id is None:
            raise GraphRAGAdapterFailure(
                f"GraphRAG selected source {source_id!r} has no native document mapping"
            )
        if evidence_id not in evidence_ids:
            evidence_ids.append(evidence_id)
    return evidence_ids


def _load_native_tables(root: Path) -> dict[str, Any]:
    try:
        import pandas as pd
    except ImportError as error:
        raise GraphRAGAdapterFailure(
            "pandas is unavailable in the GraphRAG image"
        ) from error

    tables: dict[str, Any] = {}
    for name in REQUIRED_TABLES:
        path = root / "output" / f"{name}.parquet"
        if not path.is_file():
            raise GraphRAGProductFailure(
                f"GraphRAG index completed without native {name}.parquet"
            )
        tables[name] = pd.read_parquet(path)
    covariates_path = root / "output" / "covariates.parquet"
    tables["covariates"] = (
        pd.read_parquet(covariates_path) if covariates_path.is_file() else None
    )
    return tables


def _query_job(
    root: Path, job: dict[str, Any], raw_path: Path, source_map: dict[str, str]
) -> tuple[dict[str, Any], dict[str, Any]]:
    try:
        from graphrag import api
        from graphrag.config.load_config import load_config
    except ImportError as error:
        raise GraphRAGAdapterFailure("the GraphRAG API is unavailable") from error

    tables = _load_native_tables(root)
    config = load_config(root_dir=root)
    started = time.monotonic()
    try:
        response, context_data = asyncio.run(
            api.local_search(
                config=config,
                entities=tables["entities"],
                communities=tables["communities"],
                community_reports=tables["community_reports"],
                text_units=tables["text_units"],
                relationships=tables["relationships"],
                covariates=tables["covariates"],
                community_level=2,
                response_type="Multiple Paragraphs",
                query=job["prompt"]["content"],
            )
        )
    except GraphRAGAdapterFailure:
        raise
    except Exception as error:
        raise GraphRAGProductFailure(
            f"GraphRAG local search failed: {error}"
        ) from error
    latency_ms = (time.monotonic() - started) * 1000.0
    raw = {
        "response": native_json(response),
        "context_data": native_json(context_data),
    }
    _write_json(raw_path, raw)
    source_rows = (
        _table_records(context_data.get("sources"))
        if isinstance(context_data, dict)
        else []
    )
    evidence_ids = evidence_ids_from_context(
        context_data, tables["text_units"], tables["documents"], source_map
    )
    return (
        {
            "job_id": job["job_id"],
            "classification": "completed",
            "evidence_ids": evidence_ids,
            "returned_count": min(len(source_rows), TOP_K),
            "latency_ms": round(latency_ms, 3),
            "native_status": "completed",
            "failure": None,
        },
        raw,
    )


def _state_sha256(root: Path) -> str:
    digest = hashlib.sha256()
    output = root / "output"
    for path in sorted(
        candidate for candidate in output.rglob("*") if candidate.is_file()
    ):
        digest.update(path.relative_to(output).as_posix().encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def _index_readiness(root: Path) -> dict[str, Any]:
    tables = _load_native_tables(root)
    return {
        "output_state_sha256": _state_sha256(root),
        "tables": {
            name: {
                "rows": len(table),
                "columns": [str(column) for column in table.columns],
                "sha256": hashlib.sha256(
                    (root / "output" / f"{name}.parquet").read_bytes()
                ).hexdigest(),
            }
            for name, table in tables.items()
            if table is not None
        },
    }


def run_graphrag(input_dir: Path, artifacts: Path, state_dir: Path) -> dict[str, Any]:
    """Run one cold native index pass and reuse the exact indexes for warm search."""
    _required_environment()
    _verify_version()
    jobs = _load_jobs(input_dir)
    state_dir.mkdir(parents=True, exist_ok=True)
    receipt_path = state_dir / "cold-index.json"
    if receipt_path.exists():
        raise GraphRAGAdapterFailure(
            "GraphRAG cold state already exists before indexing"
        )

    raw_root = artifacts / "raw" / "graphrag"
    roots: list[Path] = []
    cold_rows: list[dict[str, Any]] = []
    readiness: dict[str, Any] = {}
    for index, job in enumerate(jobs):
        key = _job_key(index, job["job_id"])
        root = state_dir / "indexes" / key
        roots.append(root)
        index_latency_ms = _index_job(root, raw_root / "cold" / key, job)
        native_readiness = _index_readiness(root)
        native_readiness["index_latency_ms"] = round(index_latency_ms, 3)
        readiness[job["job_id"]] = native_readiness
        row, _ = _query_job(
            root,
            job,
            raw_root / "cold" / key / "local-search.json",
            {
                _source_key(item["evidence_id"]): item["evidence_id"]
                for item in job["corpus"]["items"]
            },
        )
        native_readiness["reuse_state_sha256"] = _state_sha256(root)
        cold_rows.append(row)

    receipt = {
        "graphrag_version": GRAPHRAG_VERSION,
        "fixture_sha256": _fixture_digest(jobs),
        "indexes": {
            job["job_id"]: {
                "root": str(roots[index]),
                "output_state_sha256": readiness[job["job_id"]][
                    "reuse_state_sha256"
                ],
            }
            for index, job in enumerate(jobs)
        },
    }
    _write_json(receipt_path, receipt)
    _write_json(raw_root / "cold" / "index-readiness.json", readiness)

    warm_receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    if warm_receipt != receipt or receipt["fixture_sha256"] != _fixture_digest(jobs):
        raise GraphRAGAdapterFailure(
            "GraphRAG warm phase did not reuse the cold receipt"
        )

    warm_rows: list[dict[str, Any]] = []
    warm_readiness: dict[str, Any] = {}
    for index, job in enumerate(jobs):
        root = roots[index]
        key = _job_key(index, job["job_id"])
        current_hash = _state_sha256(root)
        expected_hash = receipt["indexes"][job["job_id"]]["output_state_sha256"]
        if current_hash != expected_hash:
            raise GraphRAGAdapterFailure(
                f"GraphRAG warm index changed before reuse for {job['job_id']}"
            )
        warm_readiness[job["job_id"]] = {
            "output_state_sha256": current_hash,
            "reused_cold_index": True,
        }
        row, _ = _query_job(
            root,
            job,
            raw_root / "warm" / key / "local-search.json",
            {
                _source_key(item["evidence_id"]): item["evidence_id"]
                for item in job["corpus"]["items"]
            },
        )
        warm_rows.append(row)
    _write_json(raw_root / "warm" / "index-readiness.json", warm_readiness)

    return {
        "schema": "elf.benchmark_unit_result/v4",
        "target": "graphrag",
        "native_mode": "native_local_search",
        "score_eligible": True,
        "result_class": "completed",
        "warm_reused_state": True,
        "ingest_count": 1,
        "ingest_duration_ms": round(
            sum(float(value["index_latency_ms"]) for value in readiness.values()),
            3,
        ),
        "phases": {
            "cold": {
                "status": "completed",
                "jobs": cold_rows,
                "adapter_metadata": {
                    "index_reused": False,
                    "native_index_count": len(roots),
                    "graphrag_version": GRAPHRAG_VERSION,
                },
            },
            "warm": {
                "status": "completed",
                "jobs": warm_rows,
                "adapter_metadata": {
                    "index_reused": True,
                    "native_index_count": len(roots),
                    "graphrag_version": GRAPHRAG_VERSION,
                },
            },
        },
    }
