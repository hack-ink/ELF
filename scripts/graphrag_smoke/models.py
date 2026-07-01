from __future__ import annotations

from dataclasses import dataclass



@dataclass
class StatusState:
    """Typed status for generated GraphRAG smoke artifacts."""

    setup: str = "blocked"
    run: str = "not_encoded"
    result: str = "blocked"
    overall: str = "blocked"
    evidence_class: str = "research_gate"
    failure_class: str = "graphrag_live_run_disabled"
    failure_reason: str = (
        "GraphRAG indexing is model-call intensive; set ELF_GRAPHRAG_SMOKE_RUN=1 "
        "and provide explicit provider configuration to attempt the live Docker smoke."
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
