"""Fixture loading, evidence mapping, and generated fixture output."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from .common import rel, write_json
from .context import CORE_FIXTURE_DIR, CORE_KINDS, FIXTURE_DIR
from .models import StatusState

def load_source_fixtures() -> list[dict[str, Any]]:
    """Load the checked-in core_archival_memory fixture corpus."""

    fixtures = []
    for path in sorted(CORE_FIXTURE_DIR.glob("*.json")):
        payload = json.loads(path.read_text(encoding="utf-8"))
        payload["_source_path"] = rel(path)
        fixtures.append(payload)

    return fixtures

def evidence_ids_for_fixture(fixture: dict[str, Any]) -> list[str]:
    """Return required evidence ids for one fixture."""

    return [
        item["evidence_id"]
        for item in fixture.get("required_evidence", [])
        if isinstance(item, dict) and item.get("evidence_id")
    ]

def all_required_evidence_ids(fixtures: list[dict[str, Any]]) -> list[str]:
    """Return de-duplicated required evidence ids."""

    ids: list[str] = []
    for fixture in fixtures:
        for evidence_id in evidence_ids_for_fixture(fixture):
            if evidence_id not in ids:
                ids.append(evidence_id)

    return ids

def source_items(fixtures: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Flatten fixture corpus items with job metadata."""

    items = []
    for fixture in fixtures:
        for item in fixture.get("corpus", {}).get("items", []):
            item_copy = dict(item)
            item_copy["job_id"] = fixture["job_id"]
            item_copy["fixture_source"] = fixture["_source_path"]
            items.append(item_copy)

    return items

def benchmark_input_contract(fixtures: list[dict[str, Any]]) -> dict[str, Any]:
    """Return the benchmark-owned Letta input contract."""

    core_blocks = []
    archival_passages = []
    for item in source_items(fixtures):
        record = {
            "source_id": item["evidence_id"],
            "job_id": item["job_id"],
            "kind": item.get("kind"),
            "text": item.get("text", ""),
            "fixture_source": item["fixture_source"],
        }
        if item.get("kind") in CORE_KINDS:
            core_blocks.append(
                {
                    "label": slug(item["evidence_id"])[:48],
                    "value": f"Source ID: {item['evidence_id']}\n{item.get('text', '')}",
                    **record,
                }
            )
        elif item.get("kind") not in {"stale_claim", "unsupported_claim"}:
            archival_passages.append(
                {
                    "text": f"Source ID: {item['evidence_id']}\n{item.get('text', '')}",
                    **record,
                }
            )

    return {
        "core_blocks": core_blocks,
        "archival_passages": archival_passages,
        "source_id_count": len({item["evidence_id"] for item in source_items(fixtures)}),
        "required_evidence_ids": all_required_evidence_ids(fixtures),
    }

def slug(value: str) -> str:
    """Return a small ASCII slug."""

    out: list[str] = []
    last_dash = False

    for char in value.lower():
        if char.isascii() and char.isalnum():
            out.append(char)
            last_dash = False
        elif not last_dash and out:
            out.append("-")
            last_dash = True

    while out and out[-1] == "-":
        out.pop()

    return "".join(out) or "item"

def ids_in_payload(payload: Any, evidence_ids: list[str]) -> list[str]:
    """Return evidence ids present anywhere in a JSON-compatible payload."""

    haystack = json.dumps(payload, sort_keys=True, default=str)
    return [evidence_id for evidence_id in evidence_ids if evidence_id in haystack]

def evidence_mapping(
    fixtures: list[dict[str, Any]],
    live_export: dict[str, Any] | None,
    status: StatusState,
) -> dict[str, Any]:
    """Map observed Letta export/readback data to fixture source ids."""

    required_ids = all_required_evidence_ids(fixtures)
    if live_export is None:
        mapped_ids: list[str] = []
    else:
        mapped_ids = ids_in_payload(live_export, required_ids)

    missing_ids = [evidence_id for evidence_id in required_ids if evidence_id not in mapped_ids]
    jobs = []
    for fixture in fixtures:
        expected = evidence_ids_for_fixture(fixture)
        mapped = [evidence_id for evidence_id in expected if evidence_id in mapped_ids]
        if status.result in {"blocked", "incomplete", "not_encoded"}:
            job_status = status.result
            reason = status.failure_reason
        elif len(mapped) == len(expected):
            job_status = "pass"
            reason = "Letta core block export and archival readback/search mapped all required source ids."
        else:
            job_status = "wrong_result"
            missing = [evidence_id for evidence_id in expected if evidence_id not in mapped]
            reason = f"Letta export/readback missed required evidence ids: {', '.join(missing)}."

        jobs.append(
            {
                "job_id": fixture["job_id"],
                "source_fixture": fixture["_source_path"],
                "expected_evidence_ids": expected,
                "mapped_evidence_ids": mapped,
                "missing_evidence_ids": [evidence_id for evidence_id in expected if evidence_id not in mapped],
                "status": job_status,
                "reason": reason,
            }
        )

    return {
        "status": status.result if missing_ids or live_export is None else "pass",
        "reason": status.failure_reason
        if live_export is None
        else (
            "Letta export/readback mapped all required fixture source ids."
            if not missing_ids
            else f"Letta export/readback missed required evidence ids: {', '.join(missing_ids)}."
        ),
        "expected_evidence_ids": required_ids,
        "mapped_evidence_ids": mapped_ids,
        "missing_evidence_ids": missing_ids,
        "jobs": jobs,
    }

def write_fixture_outputs(
    fixtures: list[dict[str, Any]],
    status: StatusState,
    mapping: dict[str, Any],
) -> Path:
    """Write generated Letta real_world_job fixtures."""

    for fixture in fixtures:
        generated = json.loads(json.dumps({k: v for k, v in fixture.items() if k != "_source_path"}))
        generated["corpus"]["profile"] = "external_adapter"
        generated["corpus"]["corpus_id"] = "letta-core-archive-export-readback-2026-06-19"
        job_mapping = next(item for item in mapping["jobs"] if item["job_id"] == fixture["job_id"])
        source_answer = fixture.get("corpus", {}).get("adapter_response", {}).get("answer", {})
        generated["corpus"]["adapter_response"] = {
            "adapter_id": "letta_core_archive_export_readback",
            "answer": {
                "content": source_answer.get("content", ""),
                "claims": source_answer.get("claims", []),
                "evidence_ids": evidence_ids_for_fixture(fixture),
                "latency_ms": 0.0,
                "cost": {
                    "currency": "USD",
                    "amount": 0.0,
                    "input_tokens": 0,
                    "output_tokens": 0,
                },
            },
        }
        generated["tags"] = sorted(set(generated.get("tags", []) + ["external_adapter", "letta_export_readback"]))
        generated["encoding"] = {}
        if job_mapping["status"] in {"blocked", "incomplete", "not_encoded"}:
            generated["encoding"] = {
                "status": job_mapping["status"],
                "reason": job_mapping["reason"],
                "follow_up": {
                    "title": "Produce Letta core/archive export-readback evidence",
                    "reason": (
                        "The benchmark must export Letta core block JSON, archival readback/search JSON, "
                        "and fixture source ids before this scenario can be scored as pass or wrong_result."
                    ),
                },
            }

        if job_mapping["status"] == "wrong_result":
            generated["corpus"]["adapter_response"]["answer"]["evidence_ids"] = job_mapping[
                "mapped_evidence_ids"
            ]

        fixture_path = FIXTURE_DIR / "core_archival_memory" / Path(fixture["_source_path"]).name
        write_json(fixture_path, generated)

    return FIXTURE_DIR / "core_archival_memory"
