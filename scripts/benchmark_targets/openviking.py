"""OpenViking native retrieval adapter for the thin benchmark runner."""

from __future__ import annotations

import dataclasses
import hashlib
import json
import os
import subprocess
import time
from enum import Enum
from pathlib import Path
from typing import Any, Callable


OPENVIKING_REVISION = "a0e822a0cd5d02e8ed5330a7f7a6d13279f8308b"
OPENVIKING_REPO = Path(os.environ.get("OPENVIKING_REPO_DIR", "/opt/openviking"))
FORBIDDEN_FIXTURE_KEYS = {
    "expected_answer",
    "required_evidence",
    "negative_traps",
    "qrels",
    "scoring_rubric",
}


class OpenVikingProductFailure(RuntimeError):
    """The native OpenViking SDK failed after the adapter reached it."""


class OpenVikingAdapterFailure(RuntimeError):
    """The adapter could not preserve its input or state contract."""


def _write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def native_json(value: Any) -> Any:
    """Convert native SDK values without dropping their structured fields."""
    if dataclasses.is_dataclass(value) and not isinstance(value, type):
        return {
            field.name: native_json(getattr(value, field.name))
            for field in dataclasses.fields(value)
        }
    if isinstance(value, Enum):
        return native_json(value.value)
    if hasattr(value, "model_dump"):
        return native_json(value.model_dump())
    if hasattr(value, "to_dict"):
        return native_json(value.to_dict())
    if isinstance(value, dict):
        return {str(key): native_json(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [native_json(item) for item in value]
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    return str(value)


def _assert_fixture_is_blind(value: Any, path: str = "$") -> None:
    if isinstance(value, dict):
        leaked = FORBIDDEN_FIXTURE_KEYS.intersection(value)
        if leaked:
            keys = ", ".join(sorted(leaked))
            raise OpenVikingAdapterFailure(
                f"OpenViking fixture leaks evaluator keys at {path}: {keys}"
            )
        for key, child in value.items():
            _assert_fixture_is_blind(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            _assert_fixture_is_blind(child, f"{path}[{index}]")


def _load_jobs(input_dir: Path) -> list[dict[str, Any]]:
    jobs: list[dict[str, Any]] = []
    for path in sorted(input_dir.glob("*.json")):
        try:
            job = json.loads(path.read_text(encoding="utf-8"))
            _assert_fixture_is_blind(job)
            job_id = job["job_id"]
            prompt = job["prompt"]["content"]
            items = job["corpus"]["items"]
            if not isinstance(job_id, str) or not job_id:
                raise TypeError("job_id must be a non-empty string")
            if not isinstance(prompt, str) or not prompt:
                raise TypeError("prompt.content must be a non-empty string")
            if not isinstance(items, list) or not items:
                raise TypeError("corpus.items must be a non-empty list")
            for item in items:
                if not isinstance(item["evidence_id"], str) or not isinstance(
                    item["text"], str
                ):
                    raise TypeError("corpus evidence_id and text must be strings")
        except OpenVikingAdapterFailure:
            raise
        except (KeyError, TypeError, json.JSONDecodeError) as error:
            raise OpenVikingAdapterFailure(
                f"invalid OpenViking fixture {path.name}: {error}"
            ) from error
        jobs.append(job)
    if not jobs:
        raise OpenVikingAdapterFailure("OpenViking received no product fixtures")
    return jobs


def _verify_revision() -> None:
    revision_file = OPENVIKING_REPO / "REVISION"
    if revision_file.is_file():
        if revision_file.read_text(encoding="utf-8").strip() == OPENVIKING_REVISION:
            return
        raise OpenVikingAdapterFailure(
            "OpenViking runtime revision file does not match the frozen revision"
        )
    completed = subprocess.run(
        ["git", "-C", str(OPENVIKING_REPO), "rev-parse", "HEAD"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if completed.returncode or completed.stdout.strip() != OPENVIKING_REVISION:
        raise OpenVikingAdapterFailure(
            "OpenViking checkout does not match the frozen revision"
        )


def _required_environment() -> dict[str, str]:
    names = (
        "EMBEDDING_API_BASE",
        "EMBEDDING_API_KEY",
        "EMBEDDING_MODEL",
        "EMBEDDING_DIMENSIONS",
    )
    missing = [name for name in names if not os.environ.get(name)]
    if missing:
        raise OpenVikingAdapterFailure(
            f"missing OpenViking configuration: {', '.join(missing)}"
        )
    return {name: os.environ[name] for name in names}


def _write_config(state_dir: Path) -> Path:
    environment = _required_environment()
    try:
        dimensions = int(environment["EMBEDDING_DIMENSIONS"])
    except ValueError as error:
        raise OpenVikingAdapterFailure(
            "EMBEDDING_DIMENSIONS must be an integer"
        ) from error
    if dimensions <= 0:
        raise OpenVikingAdapterFailure("EMBEDDING_DIMENSIONS must be positive")

    config_path = state_dir / "ov.conf"
    _write_json(
        config_path,
        {
            "default_account": "elfbench",
            "default_user": "elfbench",
            "storage": {"workspace": str(state_dir / "native")},
            "embedding": {
                "dense": {
                    "provider": "openai",
                    "api_base": environment["EMBEDDING_API_BASE"],
                    "api_key": environment["EMBEDDING_API_KEY"],
                    "model": environment["EMBEDDING_MODEL"],
                    "dimension": dimensions,
                    "encoding_format": "float",
                },
                "text_source": "content_only",
                "max_concurrent": 2,
            },
            "auto_generate_l0": False,
            "auto_generate_l1": False,
            "default_search_mode": "fast",
            "vlm": {},
            "query_planner": {},
            "rerank": {},
        },
    )
    os.environ["OPENVIKING_CONFIG_FILE"] = str(config_path)
    return config_path


def _native_call(operation: str, function: Callable[..., Any], *args: Any, **kwargs: Any) -> Any:
    try:
        return function(*args, **kwargs)
    except Exception as error:
        raise OpenVikingProductFailure(
            f"OpenViking {operation} failed: {type(error).__name__}: {error}"
        ) from error


def _open_client(native_dir: Path) -> Any:
    try:
        from openviking import OpenViking
    except Exception as error:
        raise OpenVikingProductFailure(
            f"OpenViking SDK import failed: {type(error).__name__}: {error}"
        ) from error
    client = OpenViking(path=str(native_dir))
    _native_call("initialize", client.initialize)
    return client


def _job_key(index: int, job_id: str) -> str:
    digest = hashlib.sha256(job_id.encode("utf-8")).hexdigest()[:16]
    return f"job-{index:04d}-{digest}"


def _corpus_hash(jobs: list[dict[str, Any]]) -> str:
    encoded = json.dumps(
        jobs, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _ingest_job(
    client: Any,
    job: dict[str, Any],
    *,
    job_index: int,
    source_dir: Path,
    raw_dir: Path,
) -> tuple[str, dict[str, str]]:
    key = _job_key(job_index, job["job_id"])
    target_root = f"viking://resources/elfbench/{key}"
    source_map: dict[str, str] = {}
    add_results: list[Any] = []
    job_source = source_dir / key
    job_source.mkdir(parents=True, exist_ok=True)
    _native_call("mkdir", client.mkdir, target_root)

    for item_index, item in enumerate(job["corpus"]["items"]):
        name_digest = hashlib.sha256(item["evidence_id"].encode("utf-8")).hexdigest()[:20]
        source_path = job_source / f"source-{item_index:04d}-{name_digest}.txt"
        source_path.write_text(item["text"], encoding="utf-8")
        target_uri = f"{target_root}/{source_path.name}"
        added = _native_call(
            "add_resource",
            client.add_resource,
            str(source_path),
            to=target_uri,
            wait=True,
            timeout=300,
            build_index=True,
            summarize=False,
        )
        add_results.append(added)
        root_uri = added.get("root_uri") if isinstance(added, dict) else None
        if not isinstance(root_uri, str) or not root_uri:
            raise OpenVikingAdapterFailure(
                "OpenViking add_resource returned no native root_uri"
            )
        if root_uri in source_map:
            raise OpenVikingAdapterFailure(
                f"OpenViking returned duplicate native root_uri: {root_uri}"
            )
        source_map[root_uri] = item["evidence_id"]

    _write_json(raw_dir / f"{key}-add.json", native_json(add_results))
    return target_root, source_map


def evidence_ids_from_find(found: Any, source_map: dict[str, str]) -> list[str]:
    """Map results only through the native matched-context URI field."""
    resources = getattr(found, "resources", None)
    if resources is None and isinstance(found, dict):
        resources = found.get("resources")
    if not isinstance(resources, list):
        return []

    evidence_ids: list[str] = []
    for resource in resources:
        uri = getattr(resource, "uri", None)
        if uri is None and isinstance(resource, dict):
            uri = resource.get("uri")
        evidence_id = source_map.get(uri) if isinstance(uri, str) else None
        if evidence_id and evidence_id not in evidence_ids:
            evidence_ids.append(evidence_id)
    return evidence_ids


def _query_job(
    client: Any,
    job: dict[str, Any],
    *,
    target_root: str,
    source_map: dict[str, str],
    raw_path: Path,
) -> dict[str, Any]:
    started = time.monotonic()
    found = _native_call(
        "find",
        client.find,
        job["prompt"]["content"],
        target_uri=target_root,
        limit=5,
        score_threshold=0.0,
        level=[2],
    )
    latency_ms = (time.monotonic() - started) * 1000.0
    _write_json(raw_path, native_json(found))
    resources = getattr(found, "resources", None)
    if resources is None and isinstance(found, dict):
        resources = found.get("resources")
    returned_count = len(resources) if isinstance(resources, list) else 0
    return {
        "job_id": job["job_id"],
        "classification": "completed",
        "evidence_ids": evidence_ids_from_find(found, source_map),
        "returned_count": returned_count,
        "latency_ms": round(latency_ms, 3),
        "native_status": "completed",
        "failure": None,
    }


def _close_client(client: Any) -> None:
    _native_call("close", client.close)


def _run_openviking(
    input_dir: Path, artifacts: Path, state_dir: Path
) -> dict[str, Any]:
    """Run cold ingest/query and warm query against the same native state."""
    _verify_revision()
    jobs = _load_jobs(input_dir)
    state_dir.mkdir(parents=True, exist_ok=True)
    _write_config(state_dir)
    source_dir = state_dir / "sources"
    raw_cold = artifacts / "raw" / "openviking" / "cold"
    raw_warm = artifacts / "raw" / "openviking" / "warm"
    receipt_path = state_dir / "cold-ingest.json"

    targets: list[str] = []
    source_maps: list[dict[str, str]] = []
    cold_rows: list[dict[str, Any]] = []
    ingest_duration_ms = 0.0
    client = _open_client(state_dir / "native")
    try:
        for index, job in enumerate(jobs):
            ingest_started = time.monotonic()
            target_root, source_map = _ingest_job(
                client,
                job,
                job_index=index,
                source_dir=source_dir,
                raw_dir=raw_cold,
            )
            ingest_duration_ms += (time.monotonic() - ingest_started) * 1000.0
            targets.append(target_root)
            source_maps.append(source_map)
            cold_rows.append(
                _query_job(
                    client,
                    job,
                    target_root=target_root,
                    source_map=source_map,
                    raw_path=raw_cold
                    / f"{_job_key(index, job['job_id'])}-find.json",
                )
            )
    finally:
        _close_client(client)

    receipt = {
        "revision": OPENVIKING_REVISION,
        "corpus_sha256": _corpus_hash(jobs),
        "targets": targets,
        "source_maps": source_maps,
    }
    _write_json(receipt_path, receipt)

    readback = json.loads(receipt_path.read_text(encoding="utf-8"))
    if readback != receipt or readback["corpus_sha256"] != _corpus_hash(jobs):
        raise OpenVikingAdapterFailure(
            "OpenViking warm phase did not reuse the cold ingest receipt"
        )

    warm_rows: list[dict[str, Any]] = []
    client = _open_client(state_dir / "native")
    try:
        state_readback = [
            {
                uri: _native_call("stat", client.stat, uri)
                for uri in source_map
            }
            for source_map in source_maps
        ]
        _write_json(
            raw_warm / "native-state-readback.json",
            native_json(state_readback),
        )
        for index, job in enumerate(jobs):
            warm_rows.append(
                _query_job(
                    client,
                    job,
                    target_root=targets[index],
                    source_map=source_maps[index],
                    raw_path=raw_warm
                    / f"{_job_key(index, job['job_id'])}-find.json",
                )
            )
    finally:
        _close_client(client)

    return {
        "schema": "elf.benchmark_unit_result/v4",
        "target": "openviking",
        "native_mode": "external_embedding",
        "score_eligible": True,
        "result_class": "completed",
        "warm_reused_state": True,
        "ingest_count": 1,
        "ingest_duration_ms": round(ingest_duration_ms, 3),
        "phases": {
            "cold": {
                "status": "completed",
                "jobs": cold_rows,
                "adapter_metadata": {
                    "index_reused": False,
                    "revision": OPENVIKING_REVISION,
                },
            },
            "warm": {
                "status": "completed",
                "jobs": warm_rows,
                "adapter_metadata": {
                    "index_reused": True,
                    "revision": OPENVIKING_REVISION,
                },
            },
        },
    }


def run_openviking(
    input_dir: Path, artifacts: Path, state_dir: Path
) -> dict[str, Any]:
    """Return a common result or raise a typed product or adapter failure."""
    try:
        return _run_openviking(input_dir, artifacts, state_dir)
    except (OpenVikingProductFailure, OpenVikingAdapterFailure):
        raise
    except Exception as error:
        raise OpenVikingAdapterFailure(
            f"OpenViking adapter failed: {type(error).__name__}: {error}"
        ) from error
