"""Honcho v3 self-hosted hybrid-message-search benchmark adapter."""

from __future__ import annotations

import hashlib
import json
import os
import time
import urllib.request
from pathlib import Path
from typing import Any


HONCHO_REVISION = "93dcf59c4a4225bb020b20c628799737fefae318"
HONCHO_REPO = Path(os.environ.get("HONCHO_REPO_DIR", "/app"))


class HonchoProductFailure(RuntimeError):
    """A native Honcho service or SDK operation failed."""


class HonchoAdapterFailure(RuntimeError):
    """The adapter input, state, or native identity mapping is invalid."""


def _write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, default=str, ensure_ascii=True, indent=2, sort_keys=True)
        + "\n",
        encoding="utf-8",
    )


def _load_jobs(input_dir: Path) -> list[dict[str, Any]]:
    jobs = [
        json.loads(path.read_text(encoding="utf-8"))
        for path in sorted(input_dir.glob("*.json"))
    ]
    if not jobs:
        raise HonchoAdapterFailure("Honcho received no product fixtures")
    return jobs


def _verify_revision() -> None:
    revision_path = HONCHO_REPO / "REVISION"
    if not revision_path.is_file() or revision_path.read_text(encoding="utf-8").strip() != HONCHO_REVISION:
        raise HonchoAdapterFailure("Honcho checkout does not match the frozen revision")


def _wait_for_api(base_url: str, timeout_seconds: float = 120.0) -> None:
    deadline = time.monotonic() + timeout_seconds
    last_error: Exception | None = None
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(f"{base_url.rstrip('/')}/health", timeout=2) as response:
                if 200 <= response.status < 300:
                    return
        except Exception as error:  # noqa: BLE001 - preserved as product readiness evidence
            last_error = error
        time.sleep(1)
    raise HonchoProductFailure(f"Honcho API did not become ready: {last_error}")


def _message_json(message: Any) -> dict[str, Any]:
    return {
        "id": message.id,
        "content": message.content,
        "peer_id": message.peer_id,
        "session_id": message.session_id,
        "workspace_id": message.workspace_id,
        "metadata": message.metadata,
        "created_at": message.created_at,
        "token_count": message.token_count,
    }


def _search(
    session: Any,
    query: str,
    identity: dict[str, str],
) -> tuple[list[str], list[dict[str, str]], list[dict[str, Any]], float]:
    started = time.monotonic()
    try:
        messages = session.search(query=query, limit=5)
    except Exception as error:  # noqa: BLE001 - converted to typed product failure
        raise HonchoProductFailure(f"Honcho session search failed: {error}") from error
    latency_ms = (time.monotonic() - started) * 1000.0
    evidence_ids: list[str] = []
    contexts: list[dict[str, str]] = []
    native: list[dict[str, Any]] = []
    for message in messages:
        native.append(_message_json(message))
        evidence_id = identity.get(message.id)
        if evidence_id is None:
            continue
        contexts.append({"evidence_id": evidence_id, "text": message.content})
        if evidence_id not in evidence_ids:
            evidence_ids.append(evidence_id)
    return evidence_ids, contexts, native, latency_ms


def run_honcho(input_dir: Path, artifacts: Path, state_dir: Path) -> dict[str, Any]:
    _verify_revision()
    base_url = os.environ.get("HONCHO_URL", "http://honcho-api:8000")
    _wait_for_api(base_url)
    try:
        from honcho import Honcho
    except Exception as error:  # noqa: BLE001
        raise HonchoProductFailure(f"Honcho SDK import failed: {error}") from error

    jobs = _load_jobs(input_dir)
    digest = hashlib.sha256(
        json.dumps(jobs, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    client = Honcho(
        base_url=base_url,
        workspace_id=f"elf-benchmark-{digest[:20]}",
        timeout=180.0,
        max_retries=2,
    )
    state_dir.mkdir(parents=True, exist_ok=True)
    sessions: dict[str, Any] = {}
    identities: dict[str, dict[str, str]] = {}
    phases: dict[str, Any] = {}
    raw_phases: dict[str, Any] = {}
    ingest_duration_ms = 0.0

    for phase in ("cold", "warm"):
        rows: list[dict[str, Any]] = []
        native_rows: list[dict[str, Any]] = []
        for job in jobs:
            job_id = job["job_id"]
            operations: list[dict[str, Any]] = []
            deleted = False
            if phase == "cold":
                ingest_started = time.monotonic()
                try:
                    peer = client.peer(f"peer-{job_id}")
                    session = client.session(f"session-{job_id}", peers=[peer])
                    created = session.add_messages(
                        [
                            peer.message(
                                item["text"],
                                metadata={"evidence_id": item["evidence_id"]},
                            )
                            for item in job["corpus"]["items"]
                        ]
                    )
                except Exception as error:  # noqa: BLE001
                    raise HonchoProductFailure(
                        f"Honcho native ingestion failed for {job_id}: {error}"
                    ) from error
                if len(created) != len(job["corpus"]["items"]):
                    raise HonchoAdapterFailure(
                        f"Honcho returned an incomplete message identity set for {job_id}"
                    )
                identities[job_id] = {
                    message.id: item["evidence_id"]
                    for message, item in zip(created, job["corpus"]["items"], strict=True)
                }
                sessions[job_id] = session
                ingest_duration_ms += (time.monotonic() - ingest_started) * 1000.0
                ingest_native = [_message_json(message) for message in created]
            else:
                session = sessions[job_id]
                identity = identities[job_id]
                peer = client.peer(f"peer-{job_id}")
                ingest_native = []
                for operation in job.get("operations") or []:
                    if operation["type"] == "update":
                        try:
                            created = session.add_messages(
                                peer.message(
                                    operation["text"],
                                    metadata={
                                        "evidence_id": operation["evidence_id"],
                                        "correction": True,
                                    },
                                )
                            )
                        except Exception as error:  # noqa: BLE001
                            raise HonchoProductFailure(
                                f"Honcho append-only correction failed for {job_id}: {error}"
                            ) from error
                        if len(created) != 1:
                            raise HonchoAdapterFailure(
                                f"Honcho correction returned no message identity for {job_id}"
                            )
                        identity[created[0].id] = operation["evidence_id"]
                        ingest_native.extend(_message_json(message) for message in created)
                        operations.append(
                            {
                                "requested_type": "update",
                                "native_type": "append_correction",
                                "classification": "completed",
                                "native_success": True,
                            }
                        )
                    elif operation["type"] == "delete":
                        try:
                            session.delete()
                        except Exception as error:  # noqa: BLE001
                            raise HonchoProductFailure(
                                f"Honcho session deletion failed for {job_id}: {error}"
                            ) from error
                        deleted = True
                        operations.append(
                            {
                                "requested_type": "delete",
                                "native_type": "session_delete",
                                "classification": "completed",
                                "native_success": True,
                            }
                        )
                    else:
                        raise HonchoAdapterFailure(
                            f"unsupported Honcho operation {operation['type']}"
                        )

            if deleted:
                evidence_ids, contexts, native_search, latency_ms = [], [], [], 0.0
            else:
                evidence_ids, contexts, native_search, latency_ms = _search(
                    sessions[job_id], job["prompt"]["content"], identities[job_id]
                )
            rows.append(
                {
                    "job_id": job_id,
                    "classification": "completed",
                    "evidence_ids": evidence_ids,
                    "contexts": contexts,
                    "returned_count": len(native_search),
                    "latency_ms": latency_ms,
                    "native_status": "completed",
                    "failure": None,
                    "operations": operations,
                }
            )
            native_rows.append(
                {
                    "job_id": job_id,
                    "ingest_or_operation_messages": ingest_native,
                    "search": native_search,
                    "operations": operations,
                }
            )
        phases[phase] = {
            "status": "completed",
            "jobs": rows,
            "adapter_metadata": {
                "index_reused": phase == "warm",
                "native_interface": "session.search",
            },
        }
        raw_phases[phase] = native_rows
        _write_json(artifacts / "raw" / f"honcho-{phase}.json", native_rows)

    return {
        "schema": "elf.benchmark_unit_result/v4",
        "target": "honcho",
        "native_mode": "native_hybrid_message_search",
        "score_eligible": True,
        "result_class": "completed",
        "warm_reused_state": True,
        "ingest_count": 1,
        "ingest_duration_ms": round(ingest_duration_ms, 3),
        "source_revision": HONCHO_REVISION,
        "native_deviations": {
            "update": "append-only correction message; no content replacement",
            "delete": "whole-session irreversible deletion",
        },
        "phases": phases,
    }
