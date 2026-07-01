"""Typed records for the Graphiti/Zep temporal smoke."""

from __future__ import annotations

from dataclasses import dataclass

class StatusState:
    """Typed status for generated Graphiti/Zep smoke artifacts."""

    setup: str = "blocked"
    run: str = "not_encoded"
    result: str = "blocked"
    overall: str = "blocked"
    evidence_class: str = "research_gate"
    failure_class: str = "graphiti_zep_live_run_disabled"
    failure_reason: str = (
        "Graphiti/Zep temporal graph live run is opt-in; set "
        "ELF_GRAPHITI_ZEP_SMOKE_START=1 ELF_GRAPHITI_ZEP_SMOKE_RUN=1 and provide explicit "
        "provider configuration to attempt the Docker-local FalkorDB smoke."
    )


@dataclass
class CommandRecord:
    """Captured command result without secret-bearing environment values."""

    label: str
    command: list[str]
    status: str
    elapsed_ms: float
    stdout_artifact: str | None
    stderr_artifact: str | None
    returncode: int | None
    reason: str
