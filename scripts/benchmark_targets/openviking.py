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
EMBEDDING_DIMENSION_TRANSPORT = "explicit_openai_compatible_dimensions"
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
            "storage": {
                "workspace": str(state_dir / "native"),
                "vectordb": {"backend": "local", "dimension": dimensions},
            },
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


def _install_explicit_dimension_embedder() -> None:
    """Make the pinned custom-base OpenAI client send the configured dimension."""
    try:
        import openviking.models.embedder as embedder_module
    except Exception as error:
        raise OpenVikingProductFailure(
            f"OpenViking embedder import failed: {type(error).__name__}: {error}"
        ) from error
    base = embedder_module.OpenAIDenseEmbedder
    if getattr(base, "_elf_benchmark_explicit_dimensions", False):
        return

    class ExplicitDimensionOpenAIEmbedder(base):
        _elf_benchmark_explicit_dimensions = True

        def _should_send_dimensions(self) -> bool:
            return bool(self.api_base and self.dimension)

    embedder_module.OpenAIDenseEmbedder = ExplicitDimensionOpenAIEmbedder


def _open_client(native_dir: Path) -> Any:
    _install_explicit_dimension_embedder()
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
) -> tuple[str, dict[str, str], dict[str, str]]:
    key = _job_key(job_index, job["job_id"])
    target_root = f"viking://resources/elfbench/{key}"
    source_map: dict[str, str] = {}
    source_paths: dict[str, str] = {}
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
        _assert_add_result_ready(added)
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
        source_paths[item["evidence_id"]] = str(source_path)

    _write_json(raw_dir / f"{key}-add.json", native_json(add_results))
    return target_root, source_map, source_paths


def _assert_add_result_ready(added: Any) -> None:
    if not isinstance(added, dict) or added.get("status") != "success":
        raise OpenVikingProductFailure(
            "OpenViking add_resource did not complete successfully"
        )
    queue_status = added.get("queue_status")
    embedding = queue_status.get("Embedding") if isinstance(queue_status, dict) else None
    if not isinstance(embedding, dict):
        raise OpenVikingAdapterFailure(
            "OpenViking add_resource omitted native embedding queue evidence"
        )
    if int(embedding.get("error_count") or 0) or not int(
        embedding.get("processed") or 0
    ):
        raise OpenVikingProductFailure(
            "OpenViking native embedding queue did not index the resource"
        )


def _not_found_error_type() -> type[Exception]:
    try:
        from openviking_cli.exceptions import NotFoundError
    except Exception as error:
        raise OpenVikingProductFailure(
            f"OpenViking not-found type import failed: {type(error).__name__}: {error}"
        ) from error
    return NotFoundError


def _assert_resource_absent(client: Any, uri: str) -> dict[str, str]:
    """Prove that a successful native rm removed the exact resource identity."""
    not_found_error = _not_found_error_type()
    try:
        client.stat(uri)
    except not_found_error as error:
        return {
            "uri": uri,
            "classification": "absent",
            "native_error_type": type(error).__name__,
        }
    except Exception as error:
        raise OpenVikingProductFailure(
            "OpenViking deletion readback failed: "
            f"{type(error).__name__}: {error}"
        ) from error
    raise OpenVikingProductFailure(
        "OpenViking native resource remained present after rm"
    )


def _evidence_id_for_uri(uri: str, source_map: dict[str, str]) -> str | None:
    matches = [
        (root_uri, evidence_id)
        for root_uri, evidence_id in source_map.items()
        if uri == root_uri or uri.startswith(root_uri.rstrip("/") + "/")
    ]
    if not matches:
        return None
    return max(matches, key=lambda item: len(item[0]))[1]


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
        evidence_id = (
            _evidence_id_for_uri(uri, source_map) if isinstance(uri, str) else None
        )
        if evidence_id and evidence_id not in evidence_ids:
            evidence_ids.append(evidence_id)
    return evidence_ids


def _contexts_from_find(
    client: Any, found: Any, source_map: dict[str, str]
) -> list[dict[str, Any]]:
    resources = getattr(found, "resources", None)
    if resources is None and isinstance(found, dict):
        resources = found.get("resources")
    if not isinstance(resources, list):
        return []
    contexts: list[dict[str, Any]] = []
    for resource in resources:
        uri = getattr(resource, "uri", None)
        if uri is None and isinstance(resource, dict):
            uri = resource.get("uri")
        if not isinstance(uri, str) or not uri:
            raise OpenVikingAdapterFailure(
                "OpenViking ranked resource omitted its native URI"
            )
        text = _native_call("ranked resource read", client.read, uri)
        if not isinstance(text, str):
            raise OpenVikingAdapterFailure(
                "OpenViking ranked resource read returned non-text content"
            )
        contexts.append(
            {"evidence_id": _evidence_id_for_uri(uri, source_map), "text": text}
        )
    return contexts


def _apply_operations(
    client: Any,
    jobs: list[dict[str, Any]],
    source_maps: list[dict[str, str]],
    source_paths: list[dict[str, str]],
    raw_dir: Path,
) -> tuple[dict[str, list[dict[str, Any]]], list[dict[str, Any]]]:
    receipts: dict[str, list[dict[str, Any]]] = {}
    native_jobs: list[dict[str, Any]] = []
    for index, job in enumerate(jobs):
        source_map = source_maps[index]
        job_paths = source_paths[index]
        job_receipts: list[dict[str, Any]] = []
        native_operations: list[dict[str, Any]] = []
        for operation in job.get("operations") or []:
            requested_type = operation.get("type")
            evidence_id = operation.get("evidence_id")
            matching = [
                root_uri
                for root_uri, mapped in source_map.items()
                if mapped == evidence_id
            ]
            source_path_value = job_paths.get(evidence_id)
            if len(matching) != 1 or not isinstance(source_path_value, str):
                raise OpenVikingAdapterFailure(
                    "OpenViking native mutation target did not resolve to one resource"
                )
            root_uri = matching[0]
            native_response: Any = None
            deletion_readback: dict[str, str] | None = None
            native_type = "delete"
            if requested_type == "update":
                replacement_text = operation.get("text")
                if not isinstance(replacement_text, str) or not replacement_text:
                    raise OpenVikingAdapterFailure(
                        "OpenViking update operation has no replacement text"
                    )
                source_path = Path(source_path_value)
                source_path.write_text(replacement_text, encoding="utf-8")
                native_response = _native_call(
                    "resource reindex update",
                    client.add_resource,
                    str(source_path),
                    to=root_uri,
                    wait=True,
                    timeout=300,
                    build_index=True,
                    summarize=False,
                )
                _assert_add_result_ready(native_response)
                returned_uri = (
                    native_response.get("root_uri")
                    if isinstance(native_response, dict)
                    else None
                )
                if returned_uri != root_uri:
                    raise OpenVikingAdapterFailure(
                        "OpenViking reindex update changed native resource identity"
                    )
                native_type = "reindex_update"
            elif requested_type == "delete":
                native_response = _native_call(
                    "resource delete",
                    client.rm,
                    root_uri,
                    recursive=True,
                    wait=True,
                    timeout=300,
                )
                deletion_readback = _assert_resource_absent(client, root_uri)
                del source_map[root_uri]
                del job_paths[evidence_id]
            else:
                raise OpenVikingAdapterFailure(
                    f"OpenViking does not support operation {requested_type!r}"
                )
            job_receipts.append(
                {
                    "requested_type": requested_type,
                    "native_type": native_type,
                    "classification": "completed",
                    "native_success": True,
                }
            )
            native_operations.append(
                {
                    "requested": operation,
                    "native_response": native_response,
                    "deletion_readback": deletion_readback,
                }
            )
        receipts[job["job_id"]] = job_receipts
        native_job = {"job_id": job["job_id"], "operations": native_operations}
        native_jobs.append(native_job)
        _write_json(
            raw_dir / f"{_job_key(index, job['job_id'])}-operations.json",
            native_json(native_job),
        )
    return receipts, native_jobs


def _query_job(
    client: Any,
    job: dict[str, Any],
    *,
    target_root: str,
    source_map: dict[str, str],
    raw_path: Path,
    operations: list[dict[str, Any]] | None = None,
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
        "contexts": _contexts_from_find(client, found, source_map),
        "operations": operations or [],
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
    source_paths: list[dict[str, str]] = []
    cold_rows: list[dict[str, Any]] = []
    ingest_duration_ms = 0.0
    client = _open_client(state_dir / "native")
    try:
        for index, job in enumerate(jobs):
            ingest_started = time.monotonic()
            target_root, source_map, job_source_paths = _ingest_job(
                client,
                job,
                job_index=index,
                source_dir=source_dir,
                raw_dir=raw_cold,
            )
            ingest_duration_ms += (time.monotonic() - ingest_started) * 1000.0
            targets.append(target_root)
            source_maps.append(source_map)
            source_paths.append(job_source_paths)
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
        "historical_source_maps": [
            dict(source_map) for source_map in source_maps
        ],
        "source_paths": source_paths,
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
        operation_receipts, native_operations = _apply_operations(
            client,
            jobs,
            readback["source_maps"],
            readback["source_paths"],
            raw_warm,
        )
        state_readback = [
            {
                uri: _native_call("stat", client.stat, uri)
                for uri in source_map
            }
            for source_map in readback["source_maps"]
        ]
        _write_json(
            raw_warm / "native-state-readback.json",
            native_json(state_readback),
        )
        _write_json(
            raw_warm / "native-operations.json", native_json(native_operations)
        )
        for index, job in enumerate(jobs):
            warm_rows.append(
                _query_job(
                    client,
                    job,
                    target_root=targets[index],
                    source_map=readback["historical_source_maps"][index],
                    raw_path=raw_warm
                    / f"{_job_key(index, job['job_id'])}-find.json",
                    operations=operation_receipts[job["job_id"]],
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
                    "embedding_dimensions": int(
                        os.environ["EMBEDDING_DIMENSIONS"]
                    ),
                    "embedding_dimension_transport": EMBEDDING_DIMENSION_TRANSPORT,
                },
            },
            "warm": {
                "status": "completed",
                "jobs": warm_rows,
                "adapter_metadata": {
                    "index_reused": True,
                    "revision": OPENVIKING_REVISION,
                    "embedding_dimensions": int(
                        os.environ["EMBEDDING_DIMENSIONS"]
                    ),
                    "embedding_dimension_transport": EMBEDDING_DIMENSION_TRANSPORT,
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
