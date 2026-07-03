#!/usr/bin/env python3
"""Materialize qmd candidate-replay comparability gates."""

from __future__ import annotations

import argparse
import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


SCHEMA = "elf.qmd_candidate_replay_comparability_gate/v1"
PRODUCT_MANIFEST_SCHEMA = "elf.agent_memory_quantitative_product_manifest/v1"
FRESHNESS_SCHEMA = "elf.quantitative_artifact_freshness_manifest/v1"
QMD_ADAPTER_IDS = {"qmd_live_real_world", "qmd_operator_debug_live"}
SUPPORTED_RESULT_STATES = {
    "pass",
    "wrong_result",
    "incomplete",
    "blocked",
    "not_tested",
    "not_encoded",
    "not_comparable",
    "unsupported_claim",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--product-manifest", required=True, type=Path)
    parser.add_argument("--freshness-manifest", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise SystemExit(f"{path} must contain a JSON object")
    return value


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(value, handle, indent=2, sort_keys=True)
        handle.write("\n")


def qmd_rows(manifest: dict[str, Any]) -> list[dict[str, Any]]:
    rows = manifest.get("rows", [])
    if not isinstance(rows, list):
        return []
    return [row for row in rows if is_qmd_mapping(row)]


def qmd_per_query_rows(manifest: dict[str, Any]) -> list[dict[str, Any]]:
    rows = manifest.get("per_query_rows", [])
    if not isinstance(rows, list):
        return []
    return [row for row in rows if is_qmd_mapping(row)]


def is_qmd_mapping(mapping: Any) -> bool:
    if not isinstance(mapping, dict):
        return False
    product = str(mapping.get("product", "")).strip().lower()
    adapter_id = str(mapping.get("adapter_id", "")).strip()
    return product == "qmd" or adapter_id in QMD_ADAPTER_IDS


def row_key(mapping: dict[str, Any]) -> tuple[str, str]:
    candidate = mapping.get("row", mapping)
    if not isinstance(candidate, dict):
        candidate = mapping
    return (
        str(candidate.get("product", "")).strip().lower(),
        str(candidate.get("adapter_id", "")).strip(),
    )


def freshness_rows(manifest: dict[str, Any]) -> list[dict[str, Any]]:
    rows = []
    top_level_rows = manifest.get("rows", [])
    if isinstance(top_level_rows, list):
        rows.extend(top_level_rows)
    combined_inputs = manifest.get("combined_inputs", [])
    if isinstance(combined_inputs, list):
        for combined_input in combined_inputs:
            if not isinstance(combined_input, dict):
                continue
            input_rows = combined_input.get("rows", [])
            if isinstance(input_rows, list):
                rows.extend(input_rows)
    return [row for row in rows if is_qmd_mapping(row.get("row", row))]


def has_container_digest(row: dict[str, Any]) -> bool:
    reproducibility = row.get("reproducibility", row)
    if not isinstance(reproducibility, dict):
        return False
    digest = reproducibility.get("container_image_digest")
    return (
        isinstance(digest, str)
        and digest.startswith("sha256:")
        and len(digest) == 71
        and all(char in "0123456789abcdefABCDEF" for char in digest.removeprefix("sha256:"))
    )


def has_product_commit(row: dict[str, Any]) -> bool:
    reproducibility = row.get("reproducibility", row)
    if not isinstance(reproducibility, dict):
        return False
    commit = reproducibility.get("product_commit")
    return (
        isinstance(commit, str)
        and len(commit) == 40
        and all(char in "0123456789abcdefABCDEF" for char in commit)
    )


def positive_count(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value > 0


def replay_artifact_rows(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [
        row
        for row in rows
        if row.get("result_state") == "pass"
        and positive_count(row.get("candidate_count"))
        and positive_count(row.get("expected_relevant_count"))
        and row.get("qrel_source") == "explicit_qrels"
    ]


def all_rows_same_corpus(rows: list[dict[str, Any]], corpus_id: Any) -> bool:
    if not isinstance(corpus_id, str) or not corpus_id.strip() or not rows:
        return False
    return all(row.get("source_manifest_corpus_id") == corpus_id for row in rows)


def aggregate_replay_fields_complete(row: dict[str, Any], per_query_count: int) -> bool:
    return (
        per_query_count > 0
        and row.get("ranking_query_count") == per_query_count
        and row.get("explicit_qrel_query_count") == per_query_count
        and row.get("sample_size") == per_query_count
        and row.get("ranking_coverage_state") == "complete"
        and row.get("ranked_candidate_source") == "produced_evidence_order"
        and row.get("qrel_source") == "explicit_qrels"
    )


def gate_result(product_manifest: dict[str, Any], freshness_manifest: dict[str, Any]) -> dict[str, Any]:
    qmd_product_rows = qmd_rows(product_manifest)
    qmd_query_rows = qmd_per_query_rows(product_manifest)
    qmd_freshness_rows = freshness_rows(freshness_manifest)
    replay_rows = replay_artifact_rows(qmd_query_rows)
    qmd_row = qmd_product_rows[0] if qmd_product_rows else {}
    qmd_key = row_key(qmd_row)
    matching_freshness_rows = [row for row in qmd_freshness_rows if row_key(row) == qmd_key]
    source_corpus_ids = sorted(
        {
            row.get("source_manifest_corpus_id")
            for row in [*qmd_product_rows, *qmd_query_rows]
            if row.get("source_manifest_corpus_id")
        }
    )
    gates = {
        "product_manifest_schema": product_manifest.get("schema") == PRODUCT_MANIFEST_SCHEMA,
        "freshness_manifest_schema": freshness_manifest.get("schema") == FRESHNESS_SCHEMA,
        "qmd_row_present": bool(qmd_product_rows),
        "qmd_typed_result_state_present": qmd_row.get("result_state") in SUPPORTED_RESULT_STATES,
        "qmd_result_state_pass": qmd_row.get("result_state") == "pass",
        "qmd_source_id_mapped": all_rows_same_corpus(
            [*qmd_product_rows, *qmd_query_rows],
            product_manifest.get("corpus_id"),
        ),
        "qmd_held_out": qmd_row.get("held_out") is True,
        "qmd_leakage_audited": qmd_row.get("leakage_audited") is True,
        "qmd_audit_manifest_present": bool(qmd_row.get("audit_manifest_id")),
        "qmd_replay_artifact_present": bool(replay_rows),
        "qmd_candidate_replay_complete": len(replay_rows) == len(qmd_query_rows) and bool(qmd_query_rows),
        "qmd_aggregate_replay_fields_complete": aggregate_replay_fields_complete(
            qmd_row,
            len(qmd_query_rows),
        ),
        "qmd_container_digest_present": any(has_container_digest(row) for row in matching_freshness_rows),
        "qmd_product_commit_present": any(has_product_commit(row) for row in matching_freshness_rows),
        "qmd_reproducibility_row_bound": any(
            has_container_digest(row) and has_product_commit(row) for row in matching_freshness_rows
        ),
    }
    missing = [name for name, passed in gates.items() if not passed]
    result_state = "pass" if not missing else "blocked"

    return {
        "schema": SCHEMA,
        "generated_at": datetime.now(UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "product": "qmd",
        "adapter_ids": sorted(
            {str(row.get("adapter_id")) for row in [*qmd_product_rows, *qmd_query_rows] if row.get("adapter_id")}
        ),
        "result_state": result_state,
        "comparable": result_state == "pass",
        "unqualified_leaderboard_claim_allowed": False,
        "claim_boundary": (
            "This gate only permits a qualified qmd candidate-replay comparability claim when "
            "all gates pass. It never permits an unqualified product leaderboard claim."
        ),
        "gates": gates,
        "missing_gates": missing,
        "source_manifest_corpus_ids": source_corpus_ids,
        "row_count": len(qmd_product_rows),
        "per_query_row_count": len(qmd_query_rows),
        "replay_artifact_row_count": len(replay_rows),
        "freshness_row_count": len(qmd_freshness_rows),
        "matching_freshness_row_count": len(matching_freshness_rows),
        "typed_result_state": qmd_row.get("result_state"),
        "quantitative_row": qmd_row,
    }


def main() -> None:
    args = parse_args()
    product_manifest = read_json(args.product_manifest)
    freshness_manifest = read_json(args.freshness_manifest)
    write_json(args.out, gate_result(product_manifest, freshness_manifest))


if __name__ == "__main__":
    main()
