"""PageIndex native hierarchy-construction capability adapter."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import time
from pathlib import Path
from typing import Any


PAGEINDEX_REVISION = "f413c66fee0bfbb7291c389333f9cc1adac68d57"
PAGEINDEX_REPO = Path(os.environ.get("PAGEINDEX_REPO_DIR", "/opt/pageindex"))
PAGEINDEX_PYTHON = Path(
    os.environ.get("PAGEINDEX_PYTHON", "/opt/pageindex-venv/bin/python")
)


class PageIndexProductFailure(RuntimeError):
    """The native PageIndex process failed after the adapter reached it."""


def _write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _fixture_paths(input_dir: Path) -> list[Path]:
    return sorted(input_dir.glob("*.json"))


def _load_jobs(input_dir: Path) -> list[dict[str, Any]]:
    return [json.loads(path.read_text(encoding="utf-8")) for path in _fixture_paths(input_dir)]


def _render_corpus(jobs: list[dict[str, Any]]) -> str:
    parts = ["# Benchmark source corpus", ""]
    for job in jobs:
        parts.extend((f"# Job {job['job_id']}", ""))
        for item in job["corpus"]["items"]:
            parts.extend(
                (
                    f"## Evidence {item['evidence_id']}",
                    "",
                    f"Evidence id: {item['evidence_id']}",
                    "",
                    item["text"],
                    "",
                )
            )
    return "\n".join(parts)


def flatten_nodes(nodes: Any) -> list[dict[str, str]]:
    if isinstance(nodes, dict):
        nodes = [nodes]
    if not isinstance(nodes, list):
        return []
    flattened: list[dict[str, str]] = []
    for node in nodes:
        if not isinstance(node, dict):
            continue
        flattened.append(
            {
                "title": str(node.get("title") or ""),
                "text": str(node.get("text") or ""),
                "node_id": str(node.get("node_id") or ""),
            }
        )
        flattened.extend(flatten_nodes(node.get("nodes")))
    return flattened


def _verify_revision() -> None:
    completed = subprocess.run(
        ["git", "-C", str(PAGEINDEX_REPO), "rev-parse", "HEAD"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if completed.returncode or completed.stdout.strip() != PAGEINDEX_REVISION:
        raise RuntimeError("PageIndex checkout does not match the frozen revision")


def _build_tree(markdown_path: Path, state_dir: Path, log_path: Path) -> float:
    command = [
        str(PAGEINDEX_PYTHON),
        str(PAGEINDEX_REPO / "run_pageindex.py"),
        "--md_path",
        str(markdown_path),
        "--if-add-node-summary",
        "no",
        "--if-add-doc-description",
        "no",
        "--if-add-node-text",
        "yes",
        "--if-add-node-id",
        "yes",
    ]
    env = os.environ.copy()
    env["PYTHONPATH"] = str(PAGEINDEX_REPO)
    env.setdefault("LITELLM_LOCAL_MODEL_COST_MAP", "True")
    started = time.monotonic()
    try:
        completed = subprocess.run(
            command,
            cwd=state_dir,
            env=env,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=180,
        )
    except subprocess.TimeoutExpired as error:
        log_path.write_text(error.stdout or "", encoding="utf-8")
        raise PageIndexProductFailure("PageIndex tree construction timed out") from error
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.write_text(completed.stdout, encoding="utf-8")
    if completed.returncode:
        raise PageIndexProductFailure(
            f"PageIndex tree construction exited {completed.returncode}"
        )
    return (time.monotonic() - started) * 1000.0


def _phase(
    jobs: list[dict[str, Any]], *, latency_ms: float, node_count: int, reused: bool
) -> dict[str, Any]:
    return {
        "status": "completed",
        "jobs": [
            {
                "job_id": job["job_id"],
                "classification": "completed",
                "evidence_ids": [],
                "returned_count": 0,
                "latency_ms": 0.0,
                "native_status": "completed",
                "failure": None,
                "operations": [],
            }
            for job in jobs
        ],
        "adapter_metadata": {
            "index_reused": reused,
            "tree_node_count": node_count,
            "tree_operation_latency_ms": round(latency_ms, 3),
        },
    }


def run_pageindex(input_dir: Path, artifacts: Path, state_dir: Path) -> dict[str, Any]:
    _verify_revision()
    jobs = _load_jobs(input_dir)
    if not jobs:
        raise RuntimeError("PageIndex received no product fixtures")
    state_dir.mkdir(parents=True, exist_ok=True)
    markdown_path = state_dir / "combined-corpus.md"
    tree_path = state_dir / "results" / "combined-corpus_structure.json"
    receipt_path = state_dir / "cold-ingest.json"
    rendered = _render_corpus(jobs)
    markdown_path.write_text(rendered, encoding="utf-8")

    cold_latency_ms = _build_tree(
        markdown_path, state_dir, artifacts / "raw" / "pageindex-cold.log"
    )
    if not tree_path.is_file():
        raise RuntimeError("PageIndex completed without its native tree artifact")
    tree = json.loads(tree_path.read_text(encoding="utf-8"))
    nodes = flatten_nodes(tree.get("structure"))
    if not nodes:
        raise RuntimeError("PageIndex returned an empty or malformed tree")
    receipt = {
        "revision": PAGEINDEX_REVISION,
        "corpus_sha256": hashlib.sha256(rendered.encode("utf-8")).hexdigest(),
        "tree_sha256": hashlib.sha256(tree_path.read_bytes()).hexdigest(),
    }
    _write_json(receipt_path, receipt)
    _write_json(artifacts / "raw" / "pageindex-tree.json", tree)

    warm_started = time.monotonic()
    warm_receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    warm_tree = json.loads(tree_path.read_text(encoding="utf-8"))
    if warm_receipt != receipt or hashlib.sha256(tree_path.read_bytes()).hexdigest() != receipt[
        "tree_sha256"
    ]:
        raise RuntimeError("PageIndex warm readback did not reuse the cold tree")
    warm_nodes = flatten_nodes(warm_tree.get("structure"))
    warm_latency_ms = (time.monotonic() - warm_started) * 1000.0

    return {
        "schema": "elf.benchmark_unit_result/v4",
        "target": "pageindex",
        "native_mode": "hierarchical_tree_construction",
        "score_eligible": False,
        "result_class": "completed",
        "native_deviation": (
            "The pinned PageIndex revision completes native hierarchy construction "
            "but exposes no native ranked retrieval operation."
        ),
        "warm_reused_state": True,
        "ingest_count": 1,
        "ingest_duration_ms": round(cold_latency_ms, 3),
        "phases": {
            "cold": _phase(
                jobs, latency_ms=cold_latency_ms, node_count=len(nodes), reused=False
            ),
            "warm": _phase(
                jobs,
                latency_ms=warm_latency_ms,
                node_count=len(warm_nodes),
                reused=True,
            ),
        },
    }
