from __future__ import annotations

import json
import math
import shutil
from pathlib import Path
from typing import Any

from .common import rel, slug
from .context import OUTPUT_CAPTURE_DIR, TABLES



def map_tables(output_dir: Path, corpus: list[dict[str, str]]) -> tuple[list[dict[str, Any]], list[str]]:
    """Map GraphRAG parquet table identifiers to real_world_job evidence ids."""

    try:
        import pandas as pd  # type: ignore[import-not-found]
    except ImportError as err:
        return (
            [
                {
                    "table": table,
                    "mapping_status": "reader_missing",
                    "error": f"pandas/pyarrow unavailable: {err}",
                    "row_count": 0,
                    "mapped_row_count": 0,
                    "rows": [],
                }
                for table in TABLES
            ],
            [],
        )

    table_paths = capture_table_artifacts(output_dir)
    mapped_by_table: dict[str, dict[str, list[str]]] = {}
    mappings: list[dict[str, Any]] = []

    for table in TABLES:
        path = table_paths.get(table)

        if path is None:
            mappings.append(
                {
                    "table": table,
                    "mapping_status": "missing_table",
                    "artifact": None,
                    "row_count": 0,
                    "mapped_row_count": 0,
                    "rows": [],
                }
            )
            mapped_by_table[table] = {}
            continue

        try:
            frame = pd.read_parquet(path)
        except Exception as err:  # noqa: BLE001
            mappings.append(
                {
                    "table": table,
                    "mapping_status": "read_failed",
                    "artifact": rel(path),
                    "error": str(err),
                    "row_count": 0,
                    "mapped_row_count": 0,
                    "rows": [],
                }
            )
            mapped_by_table[table] = {}
            continue

        rows, by_id = map_frame(table, frame, corpus, mapped_by_table)
        mapped_count = sum(1 for row in rows if row["evidence_ids"])
        status = "pass"

        if table in {"documents", "text_units"} and mapped_count < len(rows):
            status = "unmapped_required_rows"
        elif mapped_count == 0 and len(rows) > 0:
            status = "unmapped_rows"

        mappings.append(
            {
                "table": table,
                "mapping_status": status,
                "artifact": rel(path),
                "row_count": len(rows),
                "mapped_row_count": mapped_count,
                "rows": rows,
            }
        )
        mapped_by_table[table] = by_id

    evidence_ids: list[str] = []

    for mapping in mappings:
        for row in mapping["rows"]:
            for evidence_id in row["evidence_ids"]:
                if evidence_id not in evidence_ids:
                    evidence_ids.append(evidence_id)

    return mappings, evidence_ids


def empty_table_mappings(mapping_status: str) -> list[dict[str, Any]]:
    """Return explicit table mapping placeholders for non-live typed outcomes."""

    return [
        {
            "table": table,
            "mapping_status": mapping_status,
            "artifact": None,
            "row_count": 0,
            "mapped_row_count": 0,
            "rows": [],
        }
        for table in TABLES
    ]


def capture_table_artifacts(output_dir: Path) -> dict[str, Path]:
    """Copy known GraphRAG parquet tables into the report artifact directory."""

    table_paths: dict[str, Path] = {}

    if OUTPUT_CAPTURE_DIR.exists():
        shutil.rmtree(OUTPUT_CAPTURE_DIR)
    OUTPUT_CAPTURE_DIR.mkdir(parents=True, exist_ok=True)

    for table in TABLES:
        source = find_table_path(output_dir, table)

        if source is None:
            continue

        destination = OUTPUT_CAPTURE_DIR / f"{table}.parquet"
        shutil.copy2(source, destination)
        table_paths[table] = destination

    return table_paths


def find_table_path(output_dir: Path, table: str) -> Path | None:
    """Find a parquet file for a GraphRAG logical table name."""

    candidates = list(output_dir.rglob("*.parquet"))
    exact_names = {
        f"{table}.parquet",
        f"create_final_{table}.parquet",
        f"final_{table}.parquet",
    }

    for path in candidates:
        if path.name in exact_names:
            return path

    for path in candidates:
        stem = path.stem.lower()

        if stem.endswith(table) or stem == table or f"_{table}" in stem:
            return path

    return None


def map_frame(
    table: str,
    frame: Any,
    corpus: list[dict[str, str]],
    mapped_by_table: dict[str, dict[str, list[str]]],
) -> tuple[list[dict[str, Any]], dict[str, list[str]]]:
    """Map rows for a GraphRAG output table."""

    rows: list[dict[str, Any]] = []
    by_id: dict[str, list[str]] = {}

    for _, row in frame.iterrows():
        row_dict = {key: normalize_cell(value) for key, value in row.to_dict().items()}
        row_id = str(row_dict.get("id") or row_dict.get("human_readable_id") or row_dict.get("community") or "")
        evidence_ids = evidence_from_row(table, row_dict, corpus, mapped_by_table)
        rows.append(
            {
                "row_id": row_id,
                "human_readable_id": row_dict.get("human_readable_id"),
                "document_id": row_dict.get("document_id"),
                "community": row_dict.get("community"),
                "text_unit_ids": row_dict.get("text_unit_ids") or row_dict.get("text_units") or [],
                "evidence_ids": evidence_ids,
            }
        )

        if row_id:
            by_id[row_id] = evidence_ids

    return rows, by_id


def normalize_cell(value: Any) -> Any:
    """Normalize dataframe cell values into JSON-safe values."""

    if value is None:
        return None
    if hasattr(value, "tolist"):
        return normalize_cell(value.tolist())
    if isinstance(value, float) and math.isnan(value):
        return None
    if isinstance(value, (list, tuple, set)):
        return [normalize_cell(item) for item in value]
    if isinstance(value, dict):
        return {str(key): normalize_cell(item) for key, item in value.items()}

    return value


def evidence_from_row(
    table: str,
    row: dict[str, Any],
    corpus: list[dict[str, str]],
    mapped_by_table: dict[str, dict[str, list[str]]],
) -> list[str]:
    """Return mapped evidence ids for one output row."""

    evidence_ids: list[str] = []
    haystack = json.dumps(row, sort_keys=True, default=str)

    for item in corpus:
        evidence_id = item["evidence_id"]
        title = item["title"]
        signature = item["text"].split(".")[0]

        if (
            evidence_id in haystack
            or slug(evidence_id) in haystack
            or title in haystack
            or signature in haystack
        ):
            append_unique(evidence_ids, evidence_id)

    document_id = row.get("document_id")
    if document_id is not None:
        for evidence_id in mapped_by_table.get("documents", {}).get(str(document_id), []):
            append_unique(evidence_ids, evidence_id)

    for text_unit_id in row.get("text_unit_ids") or []:
        for evidence_id in mapped_by_table.get("text_units", {}).get(str(text_unit_id), []):
            append_unique(evidence_ids, evidence_id)

    if table == "community_reports":
        community = row.get("community")

        if community is not None:
            for candidate_id, candidate_evidence in mapped_by_table.get("communities", {}).items():
                if str(candidate_id) == str(community):
                    for evidence_id in candidate_evidence:
                        append_unique(evidence_ids, evidence_id)

    return evidence_ids


def append_unique(values: list[str], value: str) -> None:
    """Append a value if absent."""

    if value not in values:
        values.append(value)


def mapping_is_valid(mappings: list[dict[str, Any]], expected_ids: list[str]) -> tuple[bool, str]:
    """Validate source document/text-unit evidence mapping."""

    mapping_by_table = {mapping["table"]: mapping for mapping in mappings}

    for table in TABLES:
        mapping = mapping_by_table.get(table)

        if mapping is None or mapping["mapping_status"] in {"missing_table", "read_failed", "reader_missing"}:
            return False, f"GraphRAG output table {table} was not available for evidence mapping."

    for table in ("documents", "text_units"):
        mapping = mapping_by_table[table]

        if mapping["mapping_status"] != "pass":
            return False, f"GraphRAG {table} rows include identifiers that did not map to evidence ids."

    seen: list[str] = []
    for mapping in mappings:
        for row in mapping["rows"]:
            for evidence_id in row["evidence_ids"]:
                append_unique(seen, evidence_id)

    missing = [evidence_id for evidence_id in expected_ids if evidence_id not in seen]

    if missing:
        return False, f"GraphRAG output mappings missed expected evidence ids: {', '.join(missing)}."

    return True, "GraphRAG output tables mapped to expected generated evidence ids."
