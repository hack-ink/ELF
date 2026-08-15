"""Graphiti FalkorDB adapter for the thin benchmark runner."""

from __future__ import annotations

import asyncio
import dataclasses
import hashlib
import json
import os
import socket
import time
from datetime import datetime, timedelta, timezone
from enum import Enum
from pathlib import Path
from typing import Any, Awaitable, TypeVar


GRAPHITI_PACKAGE = "graphiti-core[falkordb]==0.21.0"
EMBEDDING_MODEL = "Qwen3-Embedding-8B"
EMBEDDING_DIMENSIONS = 4096
CHAT_MODEL = "gpt-5.6-luna"
CHAT_REASONING_EFFORT = "high"
SCHEMA_INSTANCE_INSTRUCTION = (
    "Return one JSON instance that validates against the requested schema. "
    "Do not return or describe the schema itself."
)
FORBIDDEN_FIXTURE_KEYS = {
    "expected_answer",
    "negative_traps",
    "qrels",
    "required_evidence",
    "scoring_rubric",
}
T = TypeVar("T")


class GraphitiProductFailure(RuntimeError):
    """A native Graphiti, FalkorDB, or model-provider operation failed."""


class GraphitiAdapterFailure(RuntimeError):
    """The adapter input, configuration, state, or identity mapping is invalid."""


def _write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(
            native_json(value), ensure_ascii=True, indent=2, sort_keys=True
        )
        + "\n",
        encoding="utf-8",
    )


def native_json(value: Any) -> Any:
    """Preserve fields from native Graphiti values in JSON-compatible form."""
    if dataclasses.is_dataclass(value) and not isinstance(value, type):
        return {
            field.name: native_json(getattr(value, field.name))
            for field in dataclasses.fields(value)
        }
    if isinstance(value, Enum):
        return native_json(value.value)
    if hasattr(value, "model_dump"):
        return native_json(value.model_dump(mode="json"))
    if isinstance(value, dict):
        return {str(key): native_json(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [native_json(item) for item in value]
    if isinstance(value, datetime):
        return value.isoformat()
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    return str(value)


def _assert_fixture_is_blind(value: Any, path: str = "$") -> None:
    if isinstance(value, dict):
        leaked = FORBIDDEN_FIXTURE_KEYS.intersection(value)
        if leaked:
            raise GraphitiAdapterFailure(
                f"Graphiti fixture leaks evaluator keys at {path}: "
                + ", ".join(sorted(leaked))
            )
        for key, child in value.items():
            _assert_fixture_is_blind(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            _assert_fixture_is_blind(child, f"{path}[{index}]")


def _load_jobs(input_dir: Path) -> list[dict[str, Any]]:
    jobs: list[dict[str, Any]] = []
    job_ids: set[str] = set()
    for path in sorted(input_dir.glob("*.json")):
        try:
            job = json.loads(path.read_text(encoding="utf-8"))
            _assert_fixture_is_blind(job)
            job_id = job["job_id"]
            prompt = job["prompt"]["content"]
            items = job["corpus"]["items"]
            if not isinstance(job_id, str) or not job_id:
                raise TypeError("job_id must be a non-empty string")
            if job_id in job_ids:
                raise TypeError(f"duplicate job_id: {job_id}")
            if not isinstance(prompt, str) or not prompt:
                raise TypeError("prompt.content must be a non-empty string")
            if not isinstance(items, list) or not items:
                raise TypeError("corpus.items must be a non-empty list")
            evidence_ids: set[str] = set()
            for item in items:
                evidence_id = item["evidence_id"]
                text = item["text"]
                if not isinstance(evidence_id, str) or not evidence_id:
                    raise TypeError("corpus evidence_id must be a non-empty string")
                if evidence_id in evidence_ids:
                    raise TypeError(f"duplicate evidence_id in {job_id}: {evidence_id}")
                if not isinstance(text, str) or not text:
                    raise TypeError("corpus text must be a non-empty string")
                evidence_ids.add(evidence_id)
        except GraphitiAdapterFailure:
            raise
        except (KeyError, TypeError, json.JSONDecodeError) as error:
            raise GraphitiAdapterFailure(
                f"invalid Graphiti fixture {path.name}: {error}"
            ) from error
        jobs.append(job)
        job_ids.add(job_id)
    if not jobs:
        raise GraphitiAdapterFailure("Graphiti received no product fixtures")
    return jobs


def _corpus_digest(jobs: list[dict[str, Any]]) -> str:
    encoded = json.dumps(
        jobs, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _required_environment() -> dict[str, str]:
    names = (
        "CHAT_API_BASE",
        "CHAT_API_KEY",
        "CHAT_MODEL",
        "CHAT_REASONING_EFFORT",
        "EMBEDDING_API_BASE",
        "EMBEDDING_API_KEY",
        "EMBEDDING_MODEL",
        "EMBEDDING_DIMENSIONS",
    )
    missing = [name for name in names if not os.environ.get(name)]
    if missing:
        raise GraphitiAdapterFailure(
            "missing Graphiti configuration: " + ", ".join(missing)
        )
    environment = {name: os.environ[name] for name in names}
    expected = {
        "CHAT_MODEL": CHAT_MODEL,
        "CHAT_REASONING_EFFORT": CHAT_REASONING_EFFORT,
        "EMBEDDING_MODEL": EMBEDDING_MODEL,
        "EMBEDDING_DIMENSIONS": str(EMBEDDING_DIMENSIONS),
    }
    mismatched = [
        f"{name}={environment[name]!r}, expected {value!r}"
        for name, value in expected.items()
        if environment[name] != value
    ]
    if mismatched:
        raise GraphitiAdapterFailure(
            "Graphiti model configuration is not frozen: " + "; ".join(mismatched)
        )
    return environment


def _chat_response_format(response_model: Any) -> dict[str, Any]:
    """Preserve Graphiti's response schema on the Luna chat-completions route."""
    if response_model is None:
        return {"type": "json_object"}
    return {
        "type": "json_schema",
        "json_schema": {
            "name": "graphiti_response",
            "strict": False,
            "schema": response_model.model_json_schema(),
        },
    }


def _validate_response_instance(response_model: Any, value: Any) -> None:
    """Validate one model response without adding product-specific fields."""
    if response_model is None:
        return
    validator = getattr(response_model, "model_validate", None)
    if callable(validator):
        validator(value)
        return
    response_model(**value)


def _wait_for_falkordb(host: str, port: int, timeout_seconds: float) -> None:
    deadline = time.monotonic() + timeout_seconds
    last_error: OSError | None = None
    while time.monotonic() < deadline:
        try:
            with socket.create_connection((host, port), timeout=2.0):
                return
        except OSError as error:
            last_error = error
            time.sleep(1.0)
    raise GraphitiProductFailure(
        f"FalkorDB did not become ready at {host}:{port}: {last_error}"
    )


async def _product_call(label: str, operation: Awaitable[T]) -> T:
    try:
        return await operation
    except GraphitiAdapterFailure:
        raise
    except Exception as error:
        raise GraphitiProductFailure(
            f"Graphiti {label} failed: {type(error).__name__}: {error}"
        ) from error


def _new_graphiti(environment: dict[str, str]) -> tuple[Any, Any, Any]:
    try:
        from graphiti_core import Graphiti
        from graphiti_core.cross_encoder.openai_reranker_client import (
            OpenAIRerankerClient,
        )
        from graphiti_core.driver.falkordb_driver import FalkorDriver
        from graphiti_core.embedder.openai import (
            OpenAIEmbedder,
            OpenAIEmbedderConfig,
        )
        from graphiti_core.llm_client.config import LLMConfig
        from graphiti_core.llm_client.openai_generic_client import (
            OpenAIGenericClient,
        )
        from graphiti_core.nodes import EpisodeType, EpisodicNode
    except Exception as error:
        raise GraphitiProductFailure(
            f"Graphiti native API import failed: {type(error).__name__}: {error}"
        ) from error

    class FixedDimensionEmbedder(OpenAIEmbedder):
        async def _vectors(self, input_data: Any) -> list[list[float]]:
            response = await self.client.embeddings.create(
                input=input_data,
                model=self.config.embedding_model,
                dimensions=self.config.embedding_dim,
            )
            vectors = [row.embedding for row in response.data]
            if any(len(vector) != EMBEDDING_DIMENSIONS for vector in vectors):
                sizes = [len(vector) for vector in vectors]
                raise RuntimeError(
                    f"embedding route returned dimensions {sizes}; "
                    f"expected {EMBEDDING_DIMENSIONS}"
                )
            return vectors

        async def create(self, input_data: Any) -> list[float]:
            return (await self._vectors(input_data))[0]

        async def create_batch(self, input_data_list: list[str]) -> list[list[float]]:
            return await self._vectors(input_data_list)

    class HighReasoningChatClient(OpenAIGenericClient):
        async def _generate_response(
            self,
            messages: list[Any],
            response_model: Any = None,
            max_tokens: int = 8192,
            model_size: Any = None,
        ) -> dict[str, Any]:
            del model_size
            native_messages = [
                {"role": message.role, "content": self._clean_input(message.content)}
                for message in messages
                if message.role in {"system", "user"}
            ]
            if response_model is not None:
                native_messages[-1]["content"] += f"\n\n{SCHEMA_INSTANCE_INSTRUCTION}"
            for attempt in range(2):
                response = await self.client.chat.completions.create(
                    model=CHAT_MODEL,
                    messages=native_messages,
                    max_tokens=max_tokens,
                    reasoning_effort=CHAT_REASONING_EFFORT,
                    response_format=_chat_response_format(response_model),
                )
                content = response.choices[0].message.content or "{}"
                try:
                    parsed = json.loads(content)
                    _validate_response_instance(response_model, parsed)
                    return parsed
                except Exception:
                    if attempt:
                        raise
                    native_messages.append(
                        {
                            "role": "user",
                            "content": (
                                "The previous response did not validate. "
                                + SCHEMA_INSTANCE_INSTRUCTION
                            ),
                        }
                    )
            raise RuntimeError("Graphiti structured response retry was exhausted")

    try:
        driver = FalkorDriver(
            host=os.environ.get("FALKORDB_HOST", "falkordb"),
            port=int(os.environ.get("FALKORDB_PORT", "6379")),
            username=os.environ.get("FALKORDB_USERNAME") or None,
            password=os.environ.get("FALKORDB_PASSWORD") or None,
            database=os.environ.get("FALKORDB_DATABASE", "elf_benchmark"),
        )
        llm_client = HighReasoningChatClient(
            LLMConfig(
                api_key=environment["CHAT_API_KEY"],
                base_url=environment["CHAT_API_BASE"],
                model=CHAT_MODEL,
                small_model=CHAT_MODEL,
                max_tokens=8192,
            )
        )
        embedder = FixedDimensionEmbedder(
            OpenAIEmbedderConfig(
                api_key=environment["EMBEDDING_API_KEY"],
                base_url=environment["EMBEDDING_API_BASE"],
                embedding_model=EMBEDDING_MODEL,
                embedding_dim=EMBEDDING_DIMENSIONS,
            )
        )
        reranker = OpenAIRerankerClient(
            config=LLMConfig(
                api_key=environment["CHAT_API_KEY"],
                base_url=environment["CHAT_API_BASE"],
                model=CHAT_MODEL,
                small_model=CHAT_MODEL,
                max_tokens=8192,
            ),
            client=llm_client.client,
        )
        return (
            Graphiti(
                graph_driver=driver,
                llm_client=llm_client,
                embedder=embedder,
                cross_encoder=reranker,
            ),
            EpisodeType,
            EpisodicNode,
        )
    except Exception as error:
        raise GraphitiProductFailure(
            f"Graphiti initialization failed: {type(error).__name__}: {error}"
        ) from error


def _group_id(job_id: str) -> str:
    return "elf-benchmark-" + hashlib.sha256(job_id.encode("utf-8")).hexdigest()[:24]


async def _cold_ingest(
    graphiti: Any, jobs: list[dict[str, Any]], episode_type: Any
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    identities: dict[str, Any] = {}
    native_jobs: list[dict[str, Any]] = []
    reference_base = datetime.now(timezone.utc)
    episode_offset = 0
    for job in jobs:
        group_id = _group_id(job["job_id"])
        episode_identity: dict[str, str] = {}
        native_adds: list[Any] = []
        for item_index, item in enumerate(job["corpus"]["items"]):
            added = await _product_call(
                "add_episode",
                graphiti.add_episode(
                    name=f"benchmark source {item_index + 1}",
                    episode_body=item["text"],
                    source_description=f"benchmark job {job['job_id']}",
                    reference_time=reference_base + timedelta(seconds=episode_offset),
                    source=episode_type.text,
                    group_id=group_id,
                ),
            )
            episode_offset += 1
            episode = getattr(added, "episode", None)
            episode_uuid = getattr(episode, "uuid", None)
            if not isinstance(episode_uuid, str) or not episode_uuid:
                raise GraphitiAdapterFailure(
                    "Graphiti add_episode returned no native episode UUID"
                )
            if episode_uuid in episode_identity:
                raise GraphitiAdapterFailure(
                    f"Graphiti returned duplicate episode UUID: {episode_uuid}"
                )
            episode_identity[episode_uuid] = item["evidence_id"]
            native_adds.append(added)
        identities[job["job_id"]] = {
            "group_id": group_id,
            "episode_identity": episode_identity,
            "historical_episode_identity": dict(episode_identity),
        }
        native_jobs.append(
            {
                "job_id": job["job_id"],
                "group_id": group_id,
                "add_episode_results": native_adds,
            }
        )
    return identities, native_jobs


def evidence_ids_from_edges(
    edges: Any, episode_identity: dict[str, str]
) -> list[str]:
    """Map native search edges only through their native episode UUIDs."""
    if not isinstance(edges, list):
        raise GraphitiAdapterFailure("Graphiti search did not return a list")
    evidence_ids: list[str] = []
    for edge in edges:
        episodes = getattr(edge, "episodes", None)
        if episodes is None and isinstance(edge, dict):
            episodes = edge.get("episodes")
        if not isinstance(episodes, list) or not all(
            isinstance(episode_uuid, str) for episode_uuid in episodes
        ):
            raise GraphitiAdapterFailure(
                "Graphiti search edge has no native episode UUID list"
            )
        for episode_uuid in episodes:
            evidence_id = episode_identity.get(episode_uuid)
            if evidence_id is None:
                raise GraphitiAdapterFailure(
                    f"Graphiti search returned unknown episode UUID: {episode_uuid}"
                )
            if evidence_id not in evidence_ids:
                evidence_ids.append(evidence_id)
    return evidence_ids


def _episode_uuids_from_edges(edges: Any) -> list[str]:
    """Return native episode UUIDs in the order exposed by ranked edges."""
    episode_uuids: list[str] = []
    for edge in edges:
        episodes = getattr(edge, "episodes", None)
        if episodes is None and isinstance(edge, dict):
            episodes = edge.get("episodes")
        if not isinstance(episodes, list):
            raise GraphitiAdapterFailure(
                "Graphiti search edge has no native episode UUID list"
            )
        for episode_uuid in episodes:
            if not isinstance(episode_uuid, str):
                raise GraphitiAdapterFailure(
                    "Graphiti search edge returned a malformed episode UUID"
                )
            if episode_uuid not in episode_uuids:
                episode_uuids.append(episode_uuid)
    return episode_uuids


async def _native_contexts_from_edges(
    episodic_node: Any,
    driver: Any,
    edges: Any,
    episode_identity: dict[str, str],
) -> tuple[list[dict[str, str]], list[Any]]:
    episode_uuids = _episode_uuids_from_edges(edges)
    if not episode_uuids:
        return [], []
    episodes = await _product_call(
        "ranked episodic-node readback",
        episodic_node.get_by_uuids(driver, episode_uuids),
    )
    native_by_uuid = {
        getattr(episode, "uuid", None): episode for episode in episodes
    }
    fallback_by_uuid: dict[str, str] = {}
    for edge in edges:
        edge_episodes = getattr(edge, "episodes", None)
        fact = getattr(edge, "fact", None)
        if isinstance(edge, dict):
            edge_episodes = edge.get("episodes", edge_episodes)
            fact = edge.get("fact", fact)
        if isinstance(edge_episodes, list) and isinstance(fact, str):
            for episode_uuid in edge_episodes:
                if isinstance(episode_uuid, str):
                    fallback_by_uuid.setdefault(episode_uuid, fact)
    contexts: list[dict[str, str]] = []
    for episode_uuid in episode_uuids:
        episode = native_by_uuid.get(episode_uuid)
        evidence_id = episode_identity.get(episode_uuid)
        content = (
            getattr(episode, "content", None)
            if episode is not None
            else fallback_by_uuid.get(episode_uuid)
        )
        if evidence_id is None or not isinstance(content, str):
            raise GraphitiAdapterFailure(
                "Graphiti ranked episode could not be read through its native identity"
            )
        contexts.append({"evidence_id": evidence_id, "text": content})
    return contexts, episodes


async def _apply_operations(
    graphiti: Any,
    episodic_node: Any,
    episode_type: Any,
    jobs: list[dict[str, Any]],
    identities: dict[str, Any],
) -> tuple[dict[str, list[dict[str, Any]]], list[dict[str, Any]], list[str]]:
    """Execute frozen native replace/delete operations before warm retrieval."""
    receipts: dict[str, list[dict[str, Any]]] = {}
    native_jobs: list[dict[str, Any]] = []
    removed_episode_uuids: list[str] = []
    reference_time = datetime.now(timezone.utc)
    for job_index, job in enumerate(jobs):
        state = identities.get(job["job_id"])
        if not isinstance(state, dict):
            raise GraphitiAdapterFailure(
                f"Graphiti state is missing job {job['job_id']}"
            )
        group_id = state.get("group_id")
        episode_identity = state.get("episode_identity")
        historical_identity = state.get("historical_episode_identity")
        if (
            not isinstance(group_id, str)
            or not isinstance(episode_identity, dict)
            or not isinstance(historical_identity, dict)
        ):
            raise GraphitiAdapterFailure(
                f"Graphiti state is malformed for {job['job_id']}"
            )
        job_receipts: list[dict[str, Any]] = []
        native_operations: list[dict[str, Any]] = []
        for operation_index, operation in enumerate(job.get("operations") or []):
            requested_type = operation.get("type")
            evidence_id = operation.get("evidence_id")
            matching = [
                episode_uuid
                for episode_uuid, mapped in episode_identity.items()
                if mapped == evidence_id
            ]
            if len(matching) != 1:
                raise GraphitiAdapterFailure(
                    "Graphiti native mutation target did not resolve to one episode"
                )
            old_uuid = matching[0]
            before = await _product_call(
                "mutation episodic-node readback",
                episodic_node.get_by_uuids(graphiti.driver, [old_uuid]),
            )
            if len(before) != 1:
                raise GraphitiAdapterFailure(
                    "Graphiti native mutation target was absent before mutation"
                )
            await _product_call("remove_episode", graphiti.remove_episode(old_uuid))
            removed_episode_uuids.append(old_uuid)
            historical_identity[old_uuid] = evidence_id
            del episode_identity[old_uuid]
            native_type = "delete"
            replacement = None
            if requested_type == "update":
                replacement_text = operation.get("text")
                if not isinstance(replacement_text, str) or not replacement_text:
                    raise GraphitiAdapterFailure(
                        "Graphiti update operation has no replacement text"
                    )
                replacement = await _product_call(
                    "replacement add_episode",
                    graphiti.add_episode(
                        name=f"benchmark replacement {operation_index + 1}",
                        episode_body=replacement_text,
                        source_description=f"benchmark job {job['job_id']} update",
                        reference_time=reference_time
                        + timedelta(seconds=job_index * 10 + operation_index),
                        source=episode_type.text,
                        group_id=group_id,
                    ),
                )
                replacement_episode = getattr(replacement, "episode", None)
                replacement_uuid = getattr(replacement_episode, "uuid", None)
                if not isinstance(replacement_uuid, str) or not replacement_uuid:
                    raise GraphitiAdapterFailure(
                        "Graphiti replacement add_episode returned no native UUID"
                    )
                episode_identity[replacement_uuid] = evidence_id
                historical_identity[replacement_uuid] = evidence_id
                native_type = "replace"
            elif requested_type != "delete":
                raise GraphitiAdapterFailure(
                    f"Graphiti does not support operation {requested_type!r}"
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
                    "removed_episode": before[0],
                    "replacement_add_episode": replacement,
                }
            )
        receipts[job["job_id"]] = job_receipts
        native_jobs.append(
            {"job_id": job["job_id"], "operations": native_operations}
        )
    return receipts, native_jobs, removed_episode_uuids


async def _query_phase(
    graphiti: Any,
    episodic_node: Any,
    jobs: list[dict[str, Any]],
    identities: dict[str, Any],
    *,
    phase: str,
    ingest_output: list[dict[str, Any]] | None = None,
    operations: dict[str, list[dict[str, Any]]] | None = None,
) -> tuple[dict[str, Any], dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    native_jobs: list[dict[str, Any]] = []
    for job in jobs:
        state = identities.get(job["job_id"])
        if not isinstance(state, dict):
            raise GraphitiAdapterFailure(
                f"Graphiti state is missing job {job['job_id']}"
            )
        group_id = state.get("group_id")
        episode_identity = state.get("episode_identity")
        historical_identity = state.get("historical_episode_identity")
        if (
            not isinstance(group_id, str)
            or not isinstance(episode_identity, dict)
            or not isinstance(historical_identity, dict)
        ):
            raise GraphitiAdapterFailure(
                f"Graphiti state is malformed for {job['job_id']}"
            )
        started = time.monotonic()
        edges = await _product_call(
            "search",
            graphiti.search(
                job["prompt"]["content"],
                group_ids=[group_id],
                num_results=5,
            ),
        )
        latency_ms = (time.monotonic() - started) * 1000.0
        evidence_ids = evidence_ids_from_edges(edges, historical_identity)
        contexts, native_episodes = await _native_contexts_from_edges(
            episodic_node, graphiti.driver, edges, historical_identity
        )
        rows.append(
            {
                "job_id": job["job_id"],
                "classification": "completed",
                "evidence_ids": evidence_ids,
                "contexts": contexts,
                "operations": (operations or {}).get(job["job_id"], []),
                "returned_count": len(edges),
                "latency_ms": round(latency_ms, 3),
                "native_status": "completed",
                "failure": None,
            }
        )
        native_jobs.append(
            {
                "job_id": job["job_id"],
                "group_id": group_id,
                "search_results": edges,
                "ranked_episode_readback": native_episodes,
            }
        )
    native_output: dict[str, Any] = {"phase": phase, "jobs": native_jobs}
    if ingest_output is not None:
        native_output["ingest"] = ingest_output
    return (
        {
            "status": "completed",
            "jobs": rows,
            "adapter_metadata": {
                "index_reused": phase == "warm",
                "package": GRAPHITI_PACKAGE,
                "driver": "falkordb",
                "embedding_model": EMBEDDING_MODEL,
                "embedding_dimensions": EMBEDDING_DIMENSIONS,
                "chat_model": CHAT_MODEL,
                "chat_reasoning_effort": CHAT_REASONING_EFFORT,
                "structured_response": (
                    "chat_completions_json_schema_with_instance_validation"
                ),
            },
        },
        native_output,
    )


async def _verify_warm_state(
    episodic_node: Any,
    driver: Any,
    identities: dict[str, Any],
    removed_episode_uuids: list[str],
) -> list[Any]:
    expected: dict[str, str] = {}
    for state in identities.values():
        for episode_uuid in state["episode_identity"]:
            expected[episode_uuid] = state["group_id"]
    episodes = await _product_call(
        "warm episodic-node query",
        episodic_node.get_by_uuids(
            driver, list(expected) + removed_episode_uuids
        ),
    )
    actual = {
        getattr(episode, "uuid", None): getattr(episode, "group_id", None)
        for episode in episodes
    }
    if actual != expected:
        raise GraphitiAdapterFailure(
            "Graphiti warm native episode state does not match post-operation state"
        )
    return episodes


async def _run_graphiti_async(
    input_dir: Path, artifacts: Path, state_dir: Path
) -> dict[str, Any]:
    jobs = _load_jobs(input_dir)
    environment = _required_environment()
    state_dir.mkdir(parents=True, exist_ok=True)
    receipt_path = state_dir / "cold-ingest.json"
    if receipt_path.exists():
        raise GraphitiAdapterFailure(
            "Graphiti cold state already exists before ingest"
        )

    host = os.environ.get("FALKORDB_HOST", "falkordb")
    try:
        port = int(os.environ.get("FALKORDB_PORT", "6379"))
        timeout_seconds = float(
            os.environ.get("FALKORDB_STARTUP_TIMEOUT_SECONDS", "120")
        )
    except ValueError as error:
        raise GraphitiAdapterFailure(
            "Graphiti FalkorDB port and timeout must be numeric"
        ) from error
    _wait_for_falkordb(host, port, timeout_seconds)

    graphiti, episode_type, episodic_node = _new_graphiti(environment)
    try:
        ingest_started = time.monotonic()
        await _product_call(
            "index and constraint creation",
            graphiti.build_indices_and_constraints(),
        )
        identities, ingest_output = await _cold_ingest(
            graphiti, jobs, episode_type
        )
        ingest_duration_ms = (time.monotonic() - ingest_started) * 1000.0
        receipt = {
            "package": GRAPHITI_PACKAGE,
            "corpus_sha256": _corpus_digest(jobs),
            "identities": identities,
        }
        _write_json(receipt_path, receipt)

        cold, cold_native = await _query_phase(
            graphiti,
            episodic_node,
            jobs,
            identities,
            phase="cold",
            ingest_output=ingest_output,
        )
        _write_json(artifacts / "raw" / "graphiti-cold.json", cold_native)

        warm_receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        if (
            warm_receipt != receipt
            or warm_receipt["corpus_sha256"] != _corpus_digest(jobs)
        ):
            raise GraphitiAdapterFailure(
                "Graphiti warm phase did not reuse the cold ingest receipt"
            )
        operation_receipts, native_operations, removed_episode_uuids = (
            await _apply_operations(
                graphiti,
                episodic_node,
                episode_type,
                jobs,
                warm_receipt["identities"],
            )
        )
        _write_json(
            artifacts / "raw" / "graphiti-operations.json",
            native_operations,
        )
        native_episodes = await _verify_warm_state(
            episodic_node,
            graphiti.driver,
            warm_receipt["identities"],
            removed_episode_uuids,
        )
        warm, warm_native = await _query_phase(
            graphiti,
            episodic_node,
            jobs,
            warm_receipt["identities"],
            phase="warm",
            operations=operation_receipts,
        )
        warm_native["native_episode_readback"] = native_episodes
        _write_json(artifacts / "raw" / "graphiti-warm.json", warm_native)
    finally:
        await _product_call("close", graphiti.close())

    return {
        "schema": "elf.benchmark_unit_result/v4",
        "target": "graphiti",
        "native_mode": "external_embedding",
        "score_eligible": True,
        "result_class": "completed",
        "warm_reused_state": True,
        "ingest_count": 1,
        "ingest_duration_ms": round(ingest_duration_ms, 3),
        "phases": {"cold": cold, "warm": warm},
    }


def run_graphiti(
    input_dir: Path, artifacts: Path, state_dir: Path
) -> dict[str, Any]:
    """Return a common result or raise a typed product or adapter failure."""
    try:
        return asyncio.run(_run_graphiti_async(input_dir, artifacts, state_dir))
    except (GraphitiProductFailure, GraphitiAdapterFailure):
        raise
    except Exception as error:
        raise GraphitiAdapterFailure(
            f"Graphiti adapter failed: {type(error).__name__}: {error}"
        ) from error
