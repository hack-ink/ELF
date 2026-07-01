"""Map Graphiti search results back to benchmark evidence."""

from __future__ import annotations

from typing import Any

def map_observed_facts(results: list[dict[str, Any]], facts: list[dict[str, Any]]) -> dict[str, Any]:
    """Map Graphiti search results back to expected evidence ids."""

    expected_by_id = {fact["evidence_id"]: fact for fact in facts}
    mappings: list[dict[str, Any]] = []
    mapped_ids: list[str] = []

    for fact in facts:
        matched = [
            result
            for result in results
            if isinstance(result.get("fact"), str) and fact["fact"].lower() in result["fact"].lower()
        ]
        if matched:
            result = matched[0]
            mapped_ids.append(fact["evidence_id"])
            mappings.append(
                {
                    "evidence_id": fact["evidence_id"],
                    "claim_id": fact["claim_id"],
                    "status": "pass",
                    "uuid": result.get("uuid"),
                    "fact": result.get("fact"),
                    "valid_at": result.get("valid_at"),
                    "invalid_at": result.get("invalid_at"),
                    "expected_valid_at": fact["valid_at"],
                    "expected_invalid_at": fact["invalid_at"],
                    "current": fact["current"],
                }
            )
        else:
            mappings.append(
                {
                    "evidence_id": fact["evidence_id"],
                    "claim_id": fact["claim_id"],
                    "status": "blocked",
                    "expected_valid_at": fact["valid_at"],
                    "expected_invalid_at": fact["invalid_at"],
                    "current": fact["current"],
                }
            )

    current_ok = any(
        item["evidence_id"] == "graphiti-zep-current-owner"
        and item["status"] == "pass"
        and not item.get("invalid_at")
        for item in mappings
    )
    historical_ok = any(
        item["evidence_id"] == "graphiti-zep-old-owner"
        and item["status"] == "pass"
        and item.get("invalid_at")
        for item in mappings
    )
    rationale_ok = "graphiti-zep-owner-rationale" in mapped_ids
    required_ids = list(expected_by_id)
    missing_ids = [evidence_id for evidence_id in required_ids if evidence_id not in mapped_ids]

    if current_ok and historical_ok and rationale_ok:
        status = "pass"
        reason = "Graphiti/Zep search results mapped current, historical, and rationale facts with validity windows."
    else:
        status = "wrong_result"
        reason = (
            "Graphiti/Zep search results did not map all required temporal facts with expected validity "
            f"windows; missing={', '.join(missing_ids) or 'none'}."
        )

    return {
        "status": status,
        "reason": reason,
        "expected_evidence_ids": required_ids,
        "mapped_evidence_ids": mapped_ids,
        "facts": mappings,
    }
