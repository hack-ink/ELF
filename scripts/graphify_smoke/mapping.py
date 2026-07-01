from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from .common import rel, slug
from .context import OUTPUT_CAPTURE_DIR, ROOT_DIR
from .corpus import expected_ids
from .models import CommandRecord, CorpusItem



def map_artifacts(corpus: list[CorpusItem], command_records: list[CommandRecord]) -> dict[str, Any]:
    """Map graphify graph/report/query output to real_world_job evidence ids."""

    graph_json = OUTPUT_CAPTURE_DIR / "graph.json"
    graph_report = OUTPUT_CAPTURE_DIR / "GRAPH_REPORT.md"
    graph_payload = read_json_or_none(graph_json)
    nodes, edges = extract_graph_rows(graph_payload)
    node_mappings = [map_graph_row("node", row, corpus) for row in nodes]
    edge_mappings = [map_graph_row("edge", row, corpus) for row in edges]
    report_mapping = map_text_artifact("graph_report", graph_report, corpus)
    query_mapping = map_query_output(command_records, corpus)
    mapped_ids: list[str] = []

    for section in (node_mappings, edge_mappings):
        for row in section:
            for evidence_id in row["evidence_ids"]:
                append_unique(mapped_ids, evidence_id)

    for row in (report_mapping, query_mapping):
        for evidence_id in row["evidence_ids"]:
            append_unique(mapped_ids, evidence_id)

    return {
        "expected_evidence_ids": expected_ids(corpus),
        "mapped_evidence_ids": mapped_ids,
        "graph_json": {
            "artifact": rel(graph_json) if graph_json.exists() else None,
            "exists": graph_json.exists(),
            "size_bytes": graph_json.stat().st_size if graph_json.exists() else 0,
        },
        "graph_report": report_mapping,
        "query_output": query_mapping,
        "nodes": node_mappings,
        "edges": edge_mappings,
    }


def read_json_or_none(path: Path) -> Any | None:
    """Read JSON and return None on missing or invalid payloads."""

    if not path.exists():
        return None

    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return None


def extract_graph_rows(payload: Any | None) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    """Extract node and edge rows from common graph JSON shapes."""

    if not isinstance(payload, dict):
        return [], []

    nodes = payload.get("nodes")
    edges = payload.get("edges") or payload.get("links") or payload.get("relationships")

    if nodes is None and isinstance(payload.get("elements"), dict):
        elements = payload["elements"]
        nodes = elements.get("nodes")
        edges = elements.get("edges")

    return rows_from_value(nodes), rows_from_value(edges)


def rows_from_value(value: Any) -> list[dict[str, Any]]:
    """Normalize a graph row container into dictionaries."""

    if not isinstance(value, list):
        return []

    rows: list[dict[str, Any]] = []
    for item in value:
        if isinstance(item, dict):
            data = item.get("data")
            rows.append(data if isinstance(data, dict) else item)

    return rows


def map_graph_row(kind: str, row: dict[str, Any], corpus: list[CorpusItem]) -> dict[str, Any]:
    """Map one graph node or edge row to evidence ids."""

    blob = json.dumps(row, sort_keys=True, default=str)
    evidence_ids = evidence_from_text(blob, corpus)
    return {
        "kind": kind,
        "row_id": str(row.get("id") or row.get("key") or row.get("source") or ""),
        "label": first_text(row, ("label", "name", "title", "type", "kind")),
        "edge_type": first_text(row, ("edge_type", "type", "relation", "relationship", "predicate")),
        "confidence": first_text(
            row,
            ("confidence", "confidence_score", "confidence_tag", "extraction_status", "status"),
        ),
        "source_files": source_values(row),
        "source_locations": source_location_values(row),
        "evidence_ids": evidence_ids,
    }


def first_text(row: dict[str, Any], keys: tuple[str, ...]) -> str | None:
    """Return the first scalar text value for a set of keys."""

    for key in keys:
        value = row.get(key)

        if isinstance(value, (str, int, float)):
            return str(value)

    return None


def source_values(value: Any) -> list[str]:
    """Collect source file-ish values from a graph row."""

    values: list[str] = []
    collect_source_values(value, values, ("source", "file", "path"))

    return values[:12]


def source_location_values(value: Any) -> list[str]:
    """Collect source location-ish values from a graph row."""

    values: list[str] = []
    collect_source_values(value, values, ("location", "line", "span", "range"))

    return values[:12]


def collect_source_values(value: Any, out: list[str], key_fragments: tuple[str, ...]) -> None:
    """Recursively collect bounded source-related values."""

    if isinstance(value, dict):
        for key, item in value.items():
            key_lower = key.lower()

            if any(fragment in key_lower for fragment in key_fragments) and isinstance(item, (str, int, float)):
                append_unique(out, str(item))
            else:
                collect_source_values(item, out, key_fragments)
    elif isinstance(value, list):
        for item in value:
            collect_source_values(item, out, key_fragments)


def map_text_artifact(kind: str, path: Path, corpus: list[CorpusItem]) -> dict[str, Any]:
    """Map a text artifact to evidence ids."""

    text = ""
    if path.exists():
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            text = ""

    return {
        "kind": kind,
        "artifact": rel(path) if path.exists() else None,
        "exists": path.exists(),
        "size_bytes": path.stat().st_size if path.exists() else 0,
        "evidence_ids": evidence_from_text(text, corpus),
    }


def map_query_output(command_records: list[CommandRecord], corpus: list[CorpusItem]) -> dict[str, Any]:
    """Map graphify query stdout to evidence ids."""

    query_record = next((record for record in command_records if record.label == "graphify-query"), None)
    text = ""
    artifact = query_record.stdout_artifact if query_record else None

    if artifact:
        path = ROOT_DIR / artifact
        if path.exists():
            text = path.read_text(encoding="utf-8")

    return {
        "kind": "query_output",
        "artifact": artifact,
        "exists": bool(artifact and (ROOT_DIR / artifact).exists()),
        "command_status": query_record.status if query_record else "not_encoded",
        "evidence_ids": evidence_from_text(text, corpus),
    }


def evidence_from_text(text: str, corpus: list[CorpusItem]) -> list[str]:
    """Return evidence ids whose signatures appear in a text blob."""

    evidence_ids: list[str] = []
    haystack = text.lower()

    for item in corpus:
        signatures = (
            item.evidence_id,
            slug(item.evidence_id),
            item.file_name,
            item.title,
            f"{item.file_name}:{item.line}",
        )

        if any(signature.lower() in haystack for signature in signatures):
            append_unique(evidence_ids, item.evidence_id)

    return evidence_ids


def append_unique(values: list[str], value: str) -> None:
    """Append a value if absent."""

    if value not in values:
        values.append(value)


def mapping_outcome(mappings: dict[str, Any], command_records: list[CommandRecord]) -> tuple[str, str]:
    """Return typed result status and explanation for evidence mapping."""

    graph_build = next((record for record in command_records if record.label == "graphify-build"), None)
    graph_query = next((record for record in command_records if record.label == "graphify-query"), None)

    if graph_build is None or graph_build.status != "pass":
        return "incomplete", "graphify did not complete graph/report build for the generated corpus."
    if not mappings["graph_json"]["exists"]:
        return "incomplete", "graphify did not produce graph.json."
    if not mappings["graph_report"]["exists"]:
        return "incomplete", "graphify did not produce GRAPH_REPORT.md."
    if graph_query is None or graph_query.status != "pass":
        return "incomplete", "graphify query output was not available for scoring."

    missing = [
        evidence_id
        for evidence_id in mappings["expected_evidence_ids"]
        if evidence_id not in mappings["mapped_evidence_ids"]
    ]

    if missing:
        return "wrong_result", f"graphify output mappings missed expected evidence ids: {', '.join(missing)}."

    return "pass", "graphify graph/report/query output mapped to expected generated evidence ids."
