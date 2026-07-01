"""Generated temporal facts used by the Graphiti/Zep smoke."""

from __future__ import annotations

from typing import Any

def temporal_facts() -> list[dict[str, Any]]:
    """Return the generated-public temporal fact corpus."""

    return [
        {
            "evidence_id": "graphiti-zep-old-owner",
            "claim_id": "relation_historical_owner",
            "source": "Team Delta",
            "edge_name": "OWNED_REVIEW",
            "target": "deployment method review",
            "fact": "Team Delta owned deployment method review before 2026-06-06.",
            "valid_at": "2026-06-05T00:00:00Z",
            "invalid_at": "2026-06-08T00:00:00Z",
            "created_at": "2026-06-05T00:00:00Z",
            "current": False,
        },
        {
            "evidence_id": "graphiti-zep-current-owner",
            "claim_id": "relation_current_owner",
            "source": "Team Echo",
            "edge_name": "OWNS_REVIEW",
            "target": "deployment method review",
            "fact": "Team Echo owns deployment method review since 2026-06-08.",
            "valid_at": "2026-06-08T00:00:00Z",
            "invalid_at": None,
            "created_at": "2026-06-08T00:00:00Z",
            "current": True,
        },
        {
            "evidence_id": "graphiti-zep-owner-rationale",
            "claim_id": "relation_owner_update_rationale",
            "source": "single-user production runbook scope",
            "edge_name": "MOVED_OWNERSHIP_TO",
            "target": "Team Echo",
            "fact": "Ownership moved to Team Echo after single-user production runbook scope changed.",
            "valid_at": "2026-06-08T00:05:00Z",
            "invalid_at": None,
            "created_at": "2026-06-08T00:05:00Z",
            "current": True,
        },
    ]
