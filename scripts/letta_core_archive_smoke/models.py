"""Typed records for the Letta core/archive smoke."""

from __future__ import annotations

from dataclasses import dataclass

class StatusState:
    """Typed status for generated Letta smoke artifacts."""

    setup: str = "blocked"
    run: str = "not_encoded"
    result: str = "blocked"
    overall: str = "blocked"
    evidence_class: str = "research_gate"
    failure_class: str = "letta_live_run_disabled"
    failure_reason: str = (
        "Letta live export/readback is disabled by default; run "
        "ELF_LETTA_SMOKE_START=1 ELF_LETTA_SMOKE_RUN=1 cargo make "
        "smoke-letta-core-archive-export-readback with explicit Docker/provider configuration."
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
