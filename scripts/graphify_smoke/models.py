from __future__ import annotations

from dataclasses import dataclass



@dataclass
class CorpusItem:
    """Generated public corpus item with source mapping metadata."""

    evidence_id: str
    claim_id: str
    title: str
    file_name: str
    text: str
    expected: bool
    kind: str = "document"
    line: int = 1


@dataclass
class StatusState:
    """Typed status for generated graphify smoke artifacts."""

    setup: str = "blocked"
    run: str = "not_encoded"
    result: str = "blocked"
    overall: str = "blocked"
    evidence_class: str = "research_gate"
    failure_class: str = "graphify_live_run_disabled"
    failure_reason: str = (
        "graphify graph/report execution is disabled; set ELF_GRAPHIFY_SMOKE_RUN=1 "
        "to install and run graphify inside Docker."
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
