"""Letta archival-memory benchmark adapter."""

from __future__ import annotations

import hashlib
import json
import os
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid
from pathlib import Path
from typing import Any, Callable, TypeVar


LETTA_IMAGE = (
    "letta/letta@sha256:"
    "aa66c3eeee13d2dfc40c650d709b550237ee31bfc91942a52fa488a13fa8c102"
)
FORBIDDEN_FIXTURE_KEYS = {
    "expected_answer",
    "negative_traps",
    "qrels",
    "required_evidence",
    "scoring_rubric",
}
T = TypeVar("T")


class LettaProductFailure(RuntimeError):
    """The native Letta client or API failed."""


class LettaAdapterFailure(RuntimeError):
    """The adapter input, state, or native identity mapping is invalid."""


def _write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _as_json(value: Any) -> Any:
    if hasattr(value, "model_dump"):
        return value.model_dump(mode="json")
    if isinstance(value, list):
        return [_as_json(item) for item in value]
    if isinstance(value, dict):
        return {key: _as_json(item) for key, item in value.items()}
    return value


def _assert_blind(value: Any, path: str = "$") -> None:
    if isinstance(value, dict):
        leaked = FORBIDDEN_FIXTURE_KEYS.intersection(value)
        if leaked:
            raise LettaAdapterFailure(
                f"Letta fixture leaks evaluator keys at {path}: "
                + ", ".join(sorted(leaked))
            )
        for key, child in value.items():
            _assert_blind(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            _assert_blind(child, f"{path}[{index}]")


def _load_jobs(input_dir: Path) -> list[dict[str, Any]]:
    jobs = [
        json.loads(path.read_text(encoding="utf-8"))
        for path in sorted(input_dir.glob("*.json"))
    ]
    if not jobs:
        raise LettaAdapterFailure("Letta received no product fixtures")
    for job in jobs:
        _assert_blind(job)
    return jobs


def _fixture_digest(jobs: list[dict[str, Any]]) -> str:
    encoded = json.dumps(
        jobs, ensure_ascii=True, separators=(",", ":"), sort_keys=True
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def _product_call(label: str, operation: Callable[[], T]) -> T:
    try:
        return operation()
    except LettaAdapterFailure:
        raise
    except Exception as error:
        raise LettaProductFailure(f"Letta {label} failed: {error}") from error


def _wait_for_server(base_url: str, timeout_seconds: float) -> None:
    deadline = time.monotonic() + timeout_seconds
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(
                f"{base_url.rstrip('/')}/v1/health/", timeout=2.0
            ) as response:
                if 200 <= response.status < 300:
                    return
                last_error = RuntimeError(
                    f"health endpoint returned {response.status}"
                )
        except (OSError, urllib.error.URLError) as error:
            last_error = error
        time.sleep(1.0)
    raise LettaProductFailure(f"Letta server did not become ready: {last_error}")


def _agent_configs() -> tuple[Any, Any]:
    from letta_client.types.embedding_config import EmbeddingConfig
    from letta_client.types.llm_config import LlmConfig

    required = (
        "CHAT_API_BASE",
        "CHAT_MODEL",
        "EMBEDDING_API_BASE",
        "EMBEDDING_MODEL",
        "EMBEDDING_DIMENSIONS",
    )
    missing = [name for name in required if not os.environ.get(name)]
    if missing:
        raise LettaAdapterFailure(
            "missing required Letta configuration: " + ", ".join(missing)
        )
    reasoning = os.environ.get("CHAT_REASONING_EFFORT")
    if reasoning not in {"low", "medium", "high"}:
        reasoning = None
    return (
        LlmConfig(
            model=os.environ["CHAT_MODEL"],
            model_endpoint_type="openai",
            model_endpoint=os.environ["CHAT_API_BASE"],
            context_window=32768,
            max_tokens=4096,
            reasoning_effort=reasoning,
        ),
        EmbeddingConfig(
            embedding_endpoint_type="openai",
            embedding_endpoint=os.environ["EMBEDDING_API_BASE"],
            embedding_model=os.environ["EMBEDDING_MODEL"],
            embedding_dim=int(os.environ["EMBEDDING_DIMENSIONS"]),
            embedding_chunk_size=300,
        ),
    )


def _search(
    base_url: str, agent_id: str, query: str, top_k: int
) -> tuple[dict[str, Any], float]:
    started = time.monotonic()
    try:
        endpoint = (
            f"{base_url.rstrip('/')}/v1/agents/{agent_id}/archival-memory/search?"
            + urllib.parse.urlencode({"query": query, "top_k": top_k})
        )
        with urllib.request.urlopen(endpoint, timeout=120.0) as response:
            native = json.loads(response.read().decode("utf-8"))
    except (OSError, urllib.error.URLError, json.JSONDecodeError) as error:
        raise LettaProductFailure(f"Letta archival search failed: {error}") from error
    latency_ms = (time.monotonic() - started) * 1000.0
    if not isinstance(native, dict) or not isinstance(native.get("results"), list):
        raise LettaAdapterFailure("Letta archival search returned a malformed response")
    return native, latency_ms


def _mapped_evidence(
    native_search: dict[str, Any], passage_identity: dict[str, str]
) -> list[str]:
    evidence_ids: list[str] = []
    for result in native_search["results"]:
        if not isinstance(result, dict) or not isinstance(result.get("id"), str):
            raise LettaAdapterFailure("Letta search result has no native passage id")
        evidence_id = passage_identity.get(result["id"])
        if evidence_id is not None and evidence_id not in evidence_ids:
            evidence_ids.append(evidence_id)
    return evidence_ids


def _cold_ingest(
    client: Any, jobs: list[dict[str, Any]]
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    llm_config, embedding_config = _agent_configs()
    receipt_agents: dict[str, Any] = {}
    native_agents: list[dict[str, Any]] = []
    for job in jobs:
        agent = _product_call(
            "agent creation",
            lambda job=job: client.agents.create(
                name=f"elf-benchmark-{job['job_id']}-{uuid.uuid4().hex[:12]}",
                llm_config=llm_config,
                embedding_config=embedding_config,
                memory_blocks=[
                    {
                        "label": "benchmark",
                        "value": "Retrieve source-backed archival memory.",
                    }
                ],
            ),
        )
        passage_identity: dict[str, str] = {}
        created: list[Any] = []
        for item in job["corpus"]["items"]:
            passages = _product_call(
                "archival passage creation",
                lambda item=item: client.agents.passages.create(
                    agent_id=agent.id, text=item["text"]
                ),
            )
            if not isinstance(passages, list) or not passages:
                raise LettaAdapterFailure(
                    "Letta passage creation returned no native passage identity"
                )
            for passage in passages:
                passage_id = getattr(passage, "id", None)
                if not isinstance(passage_id, str) or passage_id in passage_identity:
                    raise LettaAdapterFailure(
                        "Letta passage creation returned an invalid native identity"
                    )
                passage_identity[passage_id] = item["evidence_id"]
            created.extend(passages)
        receipt_agents[job["job_id"]] = {
            "agent_id": agent.id,
            "passage_identity": passage_identity,
        }
        native_agents.append(
            {
                "job_id": job["job_id"],
                "agent": _as_json(agent),
                "created_passages": _as_json(created),
            }
        )
    return receipt_agents, native_agents


def _phase(
    client: Any,
    base_url: str,
    jobs: list[dict[str, Any]],
    receipt_agents: dict[str, Any],
    *,
    phase: str,
    native_agents: list[dict[str, Any]] | None = None,
) -> tuple[dict[str, Any], dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    native_jobs: list[dict[str, Any]] = []
    for job in jobs:
        state = receipt_agents.get(job["job_id"])
        if not isinstance(state, dict):
            raise LettaAdapterFailure(f"Letta state is missing job {job['job_id']}")
        agent_id = state.get("agent_id")
        identities = state.get("passage_identity")
        if not isinstance(agent_id, str) or not isinstance(identities, dict):
            raise LettaAdapterFailure(f"Letta state is malformed for {job['job_id']}")
        warm_readback: dict[str, Any] | None = None
        if phase == "warm":
            agent = _product_call(
                "warm agent retrieval",
                lambda agent_id=agent_id: client.agents.retrieve(agent_id=agent_id),
            )
            if getattr(agent, "id", None) != agent_id:
                raise LettaAdapterFailure("Letta warm readback changed native agent identity")
            passages = _product_call(
                "warm passage readback",
                lambda agent_id=agent_id: client.agents.passages.list(
                    agent_id=agent_id, limit=1000
                ),
            )
            passage_ids = [getattr(passage, "id", None) for passage in passages]
            if (
                any(not isinstance(passage_id, str) for passage_id in passage_ids)
                or len(passage_ids) != len(set(passage_ids))
                or set(passage_ids) != set(identities)
            ):
                raise LettaAdapterFailure(
                    "Letta warm readback did not preserve cold passage identities"
                )
            warm_readback = {
                "agent": _as_json(agent),
                "passages": _as_json(passages),
            }
        native_search, latency_ms = _search(
            base_url, agent_id, job["prompt"]["content"], top_k=5
        )
        evidence_ids = _mapped_evidence(native_search, identities)
        rows.append(
            {
                "job_id": job["job_id"],
                "classification": "completed",
                "evidence_ids": evidence_ids,
                "returned_count": len(native_search["results"]),
                "latency_ms": latency_ms,
                "native_status": "completed",
                "failure": None,
            }
        )
        native_job = {
            "job_id": job["job_id"],
            "agent_id": agent_id,
            "search": native_search,
        }
        if warm_readback is not None:
            native_job["readback"] = warm_readback
        native_jobs.append(native_job)
    native_output: dict[str, Any] = {"phase": phase, "jobs": native_jobs}
    if native_agents is not None:
        native_output["ingest"] = native_agents
    return (
        {
            "status": "completed",
            "jobs": rows,
            "adapter_metadata": {"index_reused": phase == "warm"},
        },
        native_output,
    )


def run_letta(input_dir: Path, artifacts: Path, state_dir: Path) -> dict[str, Any]:
    """Run cold ingest/search and warm search against one self-hosted Letta state."""

    jobs = _load_jobs(input_dir)
    base_url = os.environ.get("LETTA_BASE_URL", "http://127.0.0.1:8283")
    _wait_for_server(
        base_url, float(os.environ.get("LETTA_STARTUP_TIMEOUT_SECONDS", "120"))
    )
    from letta_client import Letta

    client = Letta(base_url=base_url)
    state_dir.mkdir(parents=True, exist_ok=True)
    receipt_path = state_dir / "cold-ingest.json"
    if receipt_path.exists():
        raise LettaAdapterFailure("Letta cold state already exists before ingest")

    ingest_started = time.monotonic()
    receipt_agents, native_agents = _cold_ingest(client, jobs)
    ingest_duration_ms = (time.monotonic() - ingest_started) * 1000.0
    receipt = {
        "image": LETTA_IMAGE,
        "fixture_sha256": _fixture_digest(jobs),
        "agents": receipt_agents,
    }
    _write_json(receipt_path, receipt)
    cold, cold_native = _phase(
        client,
        base_url,
        jobs,
        receipt_agents,
        phase="cold",
        native_agents=native_agents,
    )
    _write_json(artifacts / "raw" / "letta-cold.json", cold_native)

    warm_receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    if warm_receipt != receipt or warm_receipt["fixture_sha256"] != _fixture_digest(jobs):
        raise LettaAdapterFailure("Letta warm phase did not reuse the cold receipt")
    warm, warm_native = _phase(
        client,
        base_url,
        jobs,
        warm_receipt["agents"],
        phase="warm",
    )
    _write_json(artifacts / "raw" / "letta-warm.json", warm_native)

    return {
        "schema": "elf.benchmark_unit_result/v4",
        "target": "letta",
        "native_mode": "external_embedding",
        "score_eligible": True,
        "result_class": "completed",
        "warm_reused_state": True,
        "ingest_count": 1,
        "ingest_duration_ms": round(ingest_duration_ms, 3),
        "phases": {"cold": cold, "warm": warm},
    }
