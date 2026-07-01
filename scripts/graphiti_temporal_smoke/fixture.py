"""Generated fixture writer for the Graphiti/Zep smoke."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from .common import write_json
from .context import FIXTURE_DIR, RUN_ID
from .models import StatusState

def write_fixture(facts: list[dict[str, Any]], status: StatusState, mapping: dict[str, Any]) -> Path:
    """Write a generated memory_evolution fixture for the smoke."""

    fixture_path = FIXTURE_DIR / "memory_evolution" / "graphiti_zep_temporal_validity.json"
    mapped_ids = mapping.get("mapped_evidence_ids", [])
    claims = []

    if status.result == "pass":
        claims = [
            {
                "claim_id": "relation_current_owner",
                "text": "Team Echo currently owns deployment method review.",
                "evidence_ids": [
                    "graphiti-zep-current-owner",
                    "graphiti-zep-old-owner",
                    "graphiti-zep-owner-rationale",
                ],
                "confidence": "derived_from_graphiti_temporal_search",
            },
            {
                "claim_id": "relation_historical_owner",
                "text": "Team Delta owned deployment method review historically.",
                "evidence_ids": ["graphiti-zep-old-owner"],
                "confidence": "derived_from_graphiti_temporal_search",
            },
            {
                "claim_id": "relation_owner_update_rationale",
                "text": "Ownership moved after single-user production runbook scope changed.",
                "evidence_ids": ["graphiti-zep-owner-rationale"],
                "confidence": "derived_from_graphiti_temporal_search",
            },
        ]

    fixture: dict[str, Any] = {
        "schema": "elf.real_world_job/v1",
        "job_id": "graphiti-zep-temporal-validity-001",
        "suite": "memory_evolution",
        "title": "Map Graphiti/Zep temporal validity windows to current and historical relation facts",
        "corpus": {
            "corpus_id": "graphiti-zep-generated-public-smoke",
            "profile": "generated_public",
            "items": [
                {
                    "evidence_id": fact["evidence_id"],
                    "kind": "temporal_fact",
                    "text": fact["fact"],
                    "source_ref": {
                        "schema": "source_ref/v1",
                        "resolver": "graphiti_zep_smoke/v1",
                        "ref": {
                            "run_id": RUN_ID,
                            "evidence_id": fact["evidence_id"],
                            "valid_at": fact["valid_at"],
                            "invalid_at": fact["invalid_at"],
                        },
                    },
                    "created_at": fact["created_at"],
                }
                for fact in facts
            ],
            "adapter_response": {
                "adapter_id": "graphiti_zep_temporal_smoke",
                "answer": {
                    "content": (
                        "Team Echo currently owns deployment method review. Team Delta owned it "
                        "historically, and the move followed the single-user production runbook scope change."
                        if claims
                        else ""
                    ),
                    "claims": claims,
                    "evidence_ids": mapped_ids,
                    "latency_ms": 0.0,
                    "cost": {
                        "currency": "USD",
                        "amount": 0.0,
                        "input_tokens": 0,
                        "output_tokens": 0,
                    },
                },
            },
        },
        "timeline": [
            {
                "event_id": "graphiti-zep-old-owner",
                "ts": "2026-06-05T00:00:00Z",
                "actor": "agent",
                "action": "recorded_relation",
                "evidence_ids": ["graphiti-zep-old-owner"],
                "summary": "Team Delta was the historical owner.",
            },
            {
                "event_id": "graphiti-zep-current-owner",
                "ts": "2026-06-08T00:00:00Z",
                "actor": "agent",
                "action": "updated_memory",
                "evidence_ids": ["graphiti-zep-current-owner", "graphiti-zep-owner-rationale"],
                "summary": "Team Echo became the current owner after the scope changed.",
            },
        ],
        "prompt": {
            "role": "user",
            "content": "Who currently owns deployment method review, and who owned it historically?",
            "job_mode": "answer",
            "constraints": ["cite_evidence", "distinguish_current_from_historical"],
        },
        "expected_answer": {
            "must_include": [
                {
                    "claim_id": "relation_current_owner",
                    "text": "Team Echo currently owns deployment method review.",
                },
                {
                    "claim_id": "relation_historical_owner",
                    "text": "Team Delta owned deployment method review historically.",
                },
            ],
            "must_not_include": ["Team Delta currently owns deployment method review."],
            "evidence_links": {
                "relation_current_owner": [
                    "graphiti-zep-current-owner",
                    "graphiti-zep-old-owner",
                    "graphiti-zep-owner-rationale",
                ],
                "relation_historical_owner": ["graphiti-zep-old-owner"],
                "relation_owner_update_rationale": ["graphiti-zep-owner-rationale"],
            },
            "answer_type": "direct_answer",
            "accepted_alternates": [],
            "requires_caveat": False,
            "requires_refusal": False,
        },
        "required_evidence": [
            {
                "evidence_id": "graphiti-zep-current-owner",
                "claim_id": "relation_current_owner",
                "requirement": "cite",
                "quote": "Team Echo owns deployment method review",
            },
            {
                "evidence_id": "graphiti-zep-old-owner",
                "claim_id": "relation_historical_owner",
                "requirement": "cite",
                "quote": "Team Delta owned deployment method review",
            },
        ],
        "negative_traps": [
            {
                "trap_id": "old-owner-as-current",
                "type": "stale_fact",
                "evidence_ids": ["graphiti-zep-old-owner"],
                "failure_if_used": False,
            }
        ],
        "scoring_rubric": {
            "dimensions": {
                "lifecycle_behavior": {
                    "weight": 0.4,
                    "max_points": 1.0,
                    "criteria": "Requires current-only versus historical temporal validity for relation facts.",
                },
                "answer_correctness": {
                    "weight": 0.25,
                    "max_points": 1.0,
                    "criteria": "Would identify current and historical owners separately.",
                },
                "evidence_grounding": {
                    "weight": 0.2,
                    "max_points": 1.0,
                    "criteria": "Would cite both current and historical relation evidence.",
                },
                "trap_avoidance": {
                    "weight": 0.15,
                    "max_points": 1.0,
                    "criteria": "Would not report the historical owner as current.",
                },
            },
            "pass_threshold": 0.8,
            "hard_fail_rules": [],
        },
        "allowed_uncertainty": {
            "can_answer_unknown": False,
            "acceptable_phrases": ["Graphiti/Zep smoke did not return temporal facts."],
            "fallback_action": "score_temporal_relation_behavior",
        },
        "memory_evolution": {
            "current_evidence_ids": ["graphiti-zep-current-owner"],
            "historical_evidence_ids": ["graphiti-zep-old-owner"],
            "stale_trap_ids": ["old-owner-as-current"],
            "conflicts": [
                {
                    "conflict_id": "relation-owner-current-historical",
                    "claim_id": "relation_current_owner",
                    "current_evidence_id": "graphiti-zep-current-owner",
                    "historical_evidence_id": "graphiti-zep-old-owner",
                    "resolved_by_evidence_id": "graphiti-zep-owner-rationale",
                }
            ],
            "update_rationale": {
                "claim_id": "relation_owner_update_rationale",
                "evidence_ids": ["graphiti-zep-owner-rationale"],
                "available": True,
            },
            "temporal_validity": {"required": True, "encoded": True},
        },
        "tags": ["external_adapter", "generated_public", "memory_evolution", "reference_graphiti_zep_temporal"],
    }

    if status.result in {"blocked", "incomplete", "not_encoded"}:
        fixture["encoding"] = {"status": status.result, "reason": status.failure_reason}

    write_json(fixture_path, fixture)

    return fixture_path
