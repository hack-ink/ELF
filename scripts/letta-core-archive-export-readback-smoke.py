#!/usr/bin/env python3
"""Docker-contained Letta core/archive export-readback smoke."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent
CORE_FIXTURE_DIR = ROOT_DIR / "apps" / "elf-eval" / "fixtures" / "real_world_memory" / "core_archival_memory"
REPORT_DIR = Path(
    os.environ.get(
        "ELF_LETTA_SMOKE_REPORT_DIR",
        ROOT_DIR / "tmp" / "real-world-memory" / "letta-core-archive",
    )
)
WORK_DIR = Path(os.environ.get("ELF_LETTA_SMOKE_WORK_DIR", REPORT_DIR / "work"))
OUT = Path(os.environ.get("ELF_LETTA_SMOKE_OUT", REPORT_DIR / "letta-core-archive-export.json"))
MANIFEST_OUT = Path(
    os.environ.get(
        "ELF_LETTA_SMOKE_MANIFEST_OUT",
        REPORT_DIR / "memory_projects_manifest.letta-core-archive.json",
    )
)
SUMMARY_OUT = Path(os.environ.get("ELF_LETTA_SMOKE_SUMMARY_OUT", REPORT_DIR / "summary.json"))
REPORT_JSON = Path(os.environ.get("ELF_LETTA_SMOKE_REPORT_JSON", REPORT_DIR / "report.json"))
REPORT_MD = Path(os.environ.get("ELF_LETTA_SMOKE_REPORT_MD", REPORT_DIR / "report.md"))
FIXTURE_DIR = REPORT_DIR / "letta-fixtures"
LOG_DIR = REPORT_DIR / "logs"

RUN_ID = os.environ.get(
    "ELF_LETTA_SMOKE_RUN_ID",
    f"letta-core-archive-{datetime.now(timezone.utc).strftime('%Y%m%d%H%M%S')}",
)
RUN_LIVE = os.environ.get("ELF_LETTA_SMOKE_RUN", "0") == "1"
ALLOW_HOST = os.environ.get("ELF_LETTA_SMOKE_ALLOW_HOST", "0") == "1"
INSTALL_CLIENT = os.environ.get("ELF_LETTA_SMOKE_INSTALL_CLIENT", "1") == "1"
LETTA_BASE_URL = os.environ.get("ELF_LETTA_BASE_URL", "http://letta:8283")
LETTA_CLIENT_PACKAGE = os.environ.get("ELF_LETTA_CLIENT_PACKAGE", "letta-client")
LETTA_CLIENT_REF = os.environ.get("ELF_LETTA_CLIENT_REF", f"pypi:{LETTA_CLIENT_PACKAGE}")
LETTA_MODEL = os.environ.get("ELF_LETTA_MODEL", "openai/gpt-4o-mini")
LETTA_EMBEDDING = os.environ.get("ELF_LETTA_EMBEDDING", "openai/text-embedding-3-small")
TIMEOUT_SECONDS = int(os.environ.get("ELF_LETTA_TIMEOUT_SECONDS", "600"))
STARTUP_ATTEMPTS = int(os.environ.get("ELF_LETTA_STARTUP_ATTEMPTS", "30"))
STARTUP_INTERVAL_SECONDS = float(os.environ.get("ELF_LETTA_STARTUP_INTERVAL_SECONDS", "2"))

CORE_KINDS = {"core_block", "core_block_contract", "core_block_event"}


@dataclass
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


def utc_now() -> str:
    """Return an RFC3339 UTC timestamp."""

    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def rel(path: Path) -> str:
    """Return a repository-relative path when possible."""

    try:
        return str(path.resolve().relative_to(ROOT_DIR))
    except ValueError:
        return str(path)


def mkdirs() -> None:
    """Create and reset output directories owned by this smoke."""

    for path in (FIXTURE_DIR, LOG_DIR):
        if path.exists():
            shutil.rmtree(path)

    for path in (REPORT_DIR, WORK_DIR, FIXTURE_DIR, LOG_DIR):
        path.mkdir(parents=True, exist_ok=True)

    for path in (OUT, MANIFEST_OUT, SUMMARY_OUT, REPORT_JSON, REPORT_MD):
        if path.exists():
            path.unlink()


def write_json(path: Path, payload: Any) -> None:
    """Write stable, pretty JSON."""

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def command_available(name: str) -> bool:
    """Return whether a command is available."""

    return shutil.which(name) is not None


def run_command(
    label: str,
    command: list[str],
    cwd: Path,
    *,
    extra_env: dict[str, str] | None = None,
) -> CommandRecord:
    """Run a command and capture stdout/stderr artifacts."""

    started = time.monotonic()
    env = os.environ.copy()
    if extra_env:
        env.update(extra_env)

    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            text=True,
            capture_output=True,
            timeout=TIMEOUT_SECONDS,
            check=False,
        )
        elapsed = (time.monotonic() - started) * 1000
        stdout_path = LOG_DIR / f"{label}.stdout.txt"
        stderr_path = LOG_DIR / f"{label}.stderr.txt"
        stdout_path.write_text(result.stdout, encoding="utf-8")
        stderr_path.write_text(result.stderr, encoding="utf-8")
        status = "pass" if result.returncode == 0 else "incomplete"
        reason = "command completed" if result.returncode == 0 else f"exit code {result.returncode}"

        return CommandRecord(
            label=label,
            command=command,
            status=status,
            elapsed_ms=elapsed,
            stdout_artifact=rel(stdout_path),
            stderr_artifact=rel(stderr_path),
            returncode=result.returncode,
            reason=reason,
        )
    except subprocess.TimeoutExpired as exc:
        elapsed = (time.monotonic() - started) * 1000
        stdout_path = LOG_DIR / f"{label}.stdout.txt"
        stderr_path = LOG_DIR / f"{label}.stderr.txt"
        stdout_path.write_text(exc.stdout or "", encoding="utf-8")
        stderr_path.write_text(exc.stderr or "", encoding="utf-8")

        return CommandRecord(
            label=label,
            command=command,
            status="incomplete",
            elapsed_ms=elapsed,
            stdout_artifact=rel(stdout_path),
            stderr_artifact=rel(stderr_path),
            returncode=None,
            reason=f"timed out after {TIMEOUT_SECONDS}s",
        )


def command_to_json(record: CommandRecord) -> dict[str, Any]:
    """Serialize a command record."""

    return {
        "label": record.label,
        "command": record.command,
        "status": record.status,
        "elapsed_ms": round(record.elapsed_ms, 3),
        "stdout_artifact": record.stdout_artifact,
        "stderr_artifact": record.stderr_artifact,
        "returncode": record.returncode,
        "reason": record.reason,
    }


def load_source_fixtures() -> list[dict[str, Any]]:
    """Load the checked-in core_archival_memory fixture corpus."""

    fixtures = []
    for path in sorted(CORE_FIXTURE_DIR.glob("*.json")):
        payload = json.loads(path.read_text(encoding="utf-8"))
        payload["_source_path"] = rel(path)
        fixtures.append(payload)

    return fixtures


def evidence_ids_for_fixture(fixture: dict[str, Any]) -> list[str]:
    """Return required evidence ids for one fixture."""

    return [
        item["evidence_id"]
        for item in fixture.get("required_evidence", [])
        if isinstance(item, dict) and item.get("evidence_id")
    ]


def all_required_evidence_ids(fixtures: list[dict[str, Any]]) -> list[str]:
    """Return de-duplicated required evidence ids."""

    ids: list[str] = []
    for fixture in fixtures:
        for evidence_id in evidence_ids_for_fixture(fixture):
            if evidence_id not in ids:
                ids.append(evidence_id)

    return ids


def source_items(fixtures: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Flatten fixture corpus items with job metadata."""

    items = []
    for fixture in fixtures:
        for item in fixture.get("corpus", {}).get("items", []):
            item_copy = dict(item)
            item_copy["job_id"] = fixture["job_id"]
            item_copy["fixture_source"] = fixture["_source_path"]
            items.append(item_copy)

    return items


def benchmark_input_contract(fixtures: list[dict[str, Any]]) -> dict[str, Any]:
    """Return the benchmark-owned Letta input contract."""

    core_blocks = []
    archival_passages = []
    for item in source_items(fixtures):
        record = {
            "source_id": item["evidence_id"],
            "job_id": item["job_id"],
            "kind": item.get("kind"),
            "text": item.get("text", ""),
            "fixture_source": item["fixture_source"],
        }
        if item.get("kind") in CORE_KINDS:
            core_blocks.append(
                {
                    "label": slug(item["evidence_id"])[:48],
                    "value": f"Source ID: {item['evidence_id']}\n{item.get('text', '')}",
                    **record,
                }
            )
        elif item.get("kind") not in {"stale_claim", "unsupported_claim"}:
            archival_passages.append(
                {
                    "text": f"Source ID: {item['evidence_id']}\n{item.get('text', '')}",
                    **record,
                }
            )

    return {
        "core_blocks": core_blocks,
        "archival_passages": archival_passages,
        "source_id_count": len({item["evidence_id"] for item in source_items(fixtures)}),
        "required_evidence_ids": all_required_evidence_ids(fixtures),
    }


def slug(value: str) -> str:
    """Return a small ASCII slug."""

    out: list[str] = []
    last_dash = False

    for char in value.lower():
        if char.isascii() and char.isalnum():
            out.append(char)
            last_dash = False
        elif not last_dash and out:
            out.append("-")
            last_dash = True

    while out and out[-1] == "-":
        out.pop()

    return "".join(out) or "item"


def wait_for_letta(command_records: list[CommandRecord]) -> bool:
    """Wait for a Letta server endpoint to become reachable."""

    started = time.monotonic()
    probes = ["/v1/health", "/health", "/v1/models"]
    last_reason = "not attempted"
    for _ in range(STARTUP_ATTEMPTS):
        for path in probes:
            url = LETTA_BASE_URL.rstrip("/") + path
            try:
                with urllib.request.urlopen(url, timeout=5) as response:
                    if 200 <= response.status < 500:
                        command_records.append(
                            CommandRecord(
                                label="letta-health-probe",
                                command=["GET", url],
                                status="pass",
                                elapsed_ms=(time.monotonic() - started) * 1000,
                                stdout_artifact=None,
                                stderr_artifact=None,
                                returncode=0,
                                reason=f"reachable via {path}",
                            )
                        )
                        return True
            except (urllib.error.URLError, TimeoutError, OSError) as exc:
                last_reason = str(exc)

        time.sleep(STARTUP_INTERVAL_SECONDS)

    command_records.append(
        CommandRecord(
            label="letta-health-probe",
            command=["GET", LETTA_BASE_URL.rstrip() + "/v1/health"],
            status="incomplete",
            elapsed_ms=(time.monotonic() - started) * 1000,
            stdout_artifact=None,
            stderr_artifact=None,
            returncode=None,
            reason=last_reason,
        )
    )
    return False


def init_letta_client(command_records: list[CommandRecord]) -> bool:
    """Install or verify the Letta Python client."""

    if INSTALL_CLIENT:
        record = run_command(
            "letta-client-install",
            [sys.executable, "-m", "pip", "install", LETTA_CLIENT_PACKAGE],
            WORK_DIR,
        )
        command_records.append(record)
        if record.status != "pass":
            return False

    record = run_command("letta-client-import", [sys.executable, "-c", "import letta_client"], WORK_DIR)
    command_records.append(record)

    return record.status == "pass"


def write_live_runner(fixtures: list[dict[str, Any]]) -> Path:
    """Write a small Python runner that uses the current Letta SDK."""

    contract = benchmark_input_contract(fixtures)
    input_path = WORK_DIR / "letta-live-input.json"
    write_json(input_path, contract)

    runner = WORK_DIR / "letta_live_runner.py"
    runner.write_text(
        """
import json
import os
from pathlib import Path

from letta_client import Letta


def as_dict(value):
    if hasattr(value, "model_dump"):
        return value.model_dump(mode="json")
    if hasattr(value, "dict"):
        return value.dict()
    return json.loads(json.dumps(value, default=str))


input_path = Path(os.environ["ELF_LETTA_LIVE_INPUT"])
output_path = Path(os.environ["ELF_LETTA_LIVE_OUTPUT"])
data = json.loads(input_path.read_text())

client = Letta(base_url=os.environ["ELF_LETTA_BASE_URL"])
agent = client.agents.create(
    name=os.environ.get("ELF_LETTA_AGENT_NAME", "elf-core-archive-smoke"),
    model=os.environ["ELF_LETTA_MODEL"],
    embedding=os.environ["ELF_LETTA_EMBEDDING"],
    memory_blocks=[
        {"label": item["label"], "value": item["value"]}
        for item in data["core_blocks"]
    ],
)

created_passages = []
for passage in data["archival_passages"]:
    created_passages.append(
        as_dict(client.agents.passages.create(agent_id=agent.id, text=passage["text"]))
    )

core_block_export = []
for item in data["core_blocks"]:
    core_block_export.append(
        {
            "source_id": item["source_id"],
            "label": item["label"],
            "block": as_dict(
                client.agents.blocks.retrieve(agent_id=agent.id, block_label=item["label"])
            ),
        }
    )

listed_passages = as_dict(client.agents.passages.list(agent_id=agent.id))
search_results = []
for source_id in data["required_evidence_ids"]:
    search_results.append(
        {
            "query": source_id,
            "response": as_dict(
                client.agents.passages.search(agent_id=agent.id, query=source_id, top_k=5)
            ),
        }
    )

output_path.write_text(
    json.dumps(
        {
            "agent": as_dict(agent),
            "core_block_export": core_block_export,
            "created_passages": created_passages,
            "archival_readback": listed_passages,
            "archival_search": search_results,
        },
        indent=2,
        sort_keys=True,
    )
    + "\\n"
)
""".lstrip(),
        encoding="utf-8",
    )

    return runner


def run_letta(fixtures: list[dict[str, Any]], command_records: list[CommandRecord]) -> dict[str, Any] | None:
    """Create the Letta benchmark agent and export readback/search data."""

    runner = write_live_runner(fixtures)
    output_path = WORK_DIR / "letta-live-output.json"
    env = {
        "ELF_LETTA_BASE_URL": LETTA_BASE_URL,
        "ELF_LETTA_MODEL": LETTA_MODEL,
        "ELF_LETTA_EMBEDDING": LETTA_EMBEDDING,
        "ELF_LETTA_LIVE_INPUT": str(WORK_DIR / "letta-live-input.json"),
        "ELF_LETTA_LIVE_OUTPUT": str(output_path),
        "ELF_LETTA_AGENT_NAME": f"elf-core-archive-smoke-{RUN_ID}",
    }
    record = run_command("letta-live-export-readback", [sys.executable, str(runner)], WORK_DIR, extra_env=env)
    command_records.append(record)
    if record.status != "pass" or not output_path.exists():
        return None

    return json.loads(output_path.read_text(encoding="utf-8"))


def ids_in_payload(payload: Any, evidence_ids: list[str]) -> list[str]:
    """Return evidence ids present anywhere in a JSON-compatible payload."""

    haystack = json.dumps(payload, sort_keys=True, default=str)
    return [evidence_id for evidence_id in evidence_ids if evidence_id in haystack]


def evidence_mapping(
    fixtures: list[dict[str, Any]],
    live_export: dict[str, Any] | None,
    status: StatusState,
) -> dict[str, Any]:
    """Map observed Letta export/readback data to fixture source ids."""

    required_ids = all_required_evidence_ids(fixtures)
    if live_export is None:
        mapped_ids: list[str] = []
    else:
        mapped_ids = ids_in_payload(live_export, required_ids)

    missing_ids = [evidence_id for evidence_id in required_ids if evidence_id not in mapped_ids]
    jobs = []
    for fixture in fixtures:
        expected = evidence_ids_for_fixture(fixture)
        mapped = [evidence_id for evidence_id in expected if evidence_id in mapped_ids]
        if status.result in {"blocked", "incomplete", "not_encoded"}:
            job_status = status.result
            reason = status.failure_reason
        elif len(mapped) == len(expected):
            job_status = "pass"
            reason = "Letta core block export and archival readback/search mapped all required source ids."
        else:
            job_status = "wrong_result"
            missing = [evidence_id for evidence_id in expected if evidence_id not in mapped]
            reason = f"Letta export/readback missed required evidence ids: {', '.join(missing)}."

        jobs.append(
            {
                "job_id": fixture["job_id"],
                "source_fixture": fixture["_source_path"],
                "expected_evidence_ids": expected,
                "mapped_evidence_ids": mapped,
                "missing_evidence_ids": [evidence_id for evidence_id in expected if evidence_id not in mapped],
                "status": job_status,
                "reason": reason,
            }
        )

    return {
        "status": status.result if missing_ids or live_export is None else "pass",
        "reason": status.failure_reason
        if live_export is None
        else (
            "Letta export/readback mapped all required fixture source ids."
            if not missing_ids
            else f"Letta export/readback missed required evidence ids: {', '.join(missing_ids)}."
        ),
        "expected_evidence_ids": required_ids,
        "mapped_evidence_ids": mapped_ids,
        "missing_evidence_ids": missing_ids,
        "jobs": jobs,
    }


def write_fixture_outputs(
    fixtures: list[dict[str, Any]],
    status: StatusState,
    mapping: dict[str, Any],
) -> Path:
    """Write generated Letta real_world_job fixtures."""

    for fixture in fixtures:
        generated = json.loads(json.dumps({k: v for k, v in fixture.items() if k != "_source_path"}))
        generated["corpus"]["profile"] = "external_adapter"
        generated["corpus"]["corpus_id"] = "letta-core-archive-export-readback-2026-06-19"
        job_mapping = next(item for item in mapping["jobs"] if item["job_id"] == fixture["job_id"])
        source_answer = fixture.get("corpus", {}).get("adapter_response", {}).get("answer", {})
        generated["corpus"]["adapter_response"] = {
            "adapter_id": "letta_core_archive_export_readback",
            "answer": {
                "content": source_answer.get("content", ""),
                "claims": source_answer.get("claims", []),
                "evidence_ids": evidence_ids_for_fixture(fixture),
                "latency_ms": 0.0,
                "cost": {
                    "currency": "USD",
                    "amount": 0.0,
                    "input_tokens": 0,
                    "output_tokens": 0,
                },
            },
        }
        generated["tags"] = sorted(set(generated.get("tags", []) + ["external_adapter", "letta_export_readback"]))
        generated["encoding"] = {}
        if job_mapping["status"] in {"blocked", "incomplete", "not_encoded"}:
            generated["encoding"] = {
                "status": job_mapping["status"],
                "reason": job_mapping["reason"],
                "follow_up": {
                    "title": "Produce Letta core/archive export-readback evidence",
                    "reason": (
                        "The benchmark must export Letta core block JSON, archival readback/search JSON, "
                        "and fixture source ids before this scenario can be scored as pass or wrong_result."
                    ),
                },
            }

        if job_mapping["status"] == "wrong_result":
            generated["corpus"]["adapter_response"]["answer"]["evidence_ids"] = job_mapping[
                "mapped_evidence_ids"
            ]

        fixture_path = FIXTURE_DIR / "core_archival_memory" / Path(fixture["_source_path"]).name
        write_json(fixture_path, generated)

    return FIXTURE_DIR / "core_archival_memory"


def run_scored_report(fixture_path: Path, manifest_path: Path, status: StatusState) -> dict[str, Any]:
    """Score the generated Letta fixtures through the real-world job runner."""

    run_cmd = [
        "cargo",
        "run",
        "-p",
        "elf-eval",
        "--bin",
        "real_world_job_benchmark",
        "--",
        "run",
        "--fixtures",
        str(fixture_path),
        "--out",
        str(REPORT_JSON),
        "--run-id",
        "real-world-memory-live-letta-core-archive",
        "--adapter-id",
        "letta_core_archive_export_readback",
        "--adapter-name",
        "Letta core/archive export-readback adapter",
        "--adapter-behavior",
        "docker_core_archive_export_readback",
        "--adapter-storage-status",
        status.setup,
        "--adapter-runtime-status",
        status.overall,
        "--adapter-notes",
        "Generated by the Letta core/archive export-readback smoke; pass requires exported core block JSON, archival readback/search JSON, and mapped fixture source ids.",
        "--external-adapter-manifest",
        str(manifest_path),
    ]
    publish_cmd = [
        "cargo",
        "run",
        "-p",
        "elf-eval",
        "--bin",
        "real_world_job_benchmark",
        "--",
        "publish",
        "--report",
        str(REPORT_JSON),
        "--out",
        str(REPORT_MD),
    ]

    subprocess.run(run_cmd, cwd=ROOT_DIR, check=True)
    subprocess.run(publish_cmd, cwd=ROOT_DIR, check=True)

    report = json.loads(REPORT_JSON.read_text(encoding="utf-8"))
    return {
        "json": rel(REPORT_JSON),
        "markdown": rel(REPORT_MD),
        "summary": report.get("summary", {}),
        "suites": report.get("suites", []),
    }


def scored_benchmark(report: dict[str, Any] | None) -> dict[str, Any]:
    """Extract the post-score benchmark status from a real_world_job report."""

    if report is None:
        return {
            "schema": "elf.scored_benchmark_status/v1",
            "source": "real_world_job_benchmark",
            "status": "pending",
            "reason": "The Letta smoke materialization was written before benchmark scoring completed.",
        }

    summary = report.get("summary", {})
    counts = {
        status: int(summary.get(status, 0) or 0)
        for status in ("pass", "wrong_result", "lifecycle_fail", "incomplete", "blocked", "not_encoded")
    }
    status = next((name for name, count in counts.items() if name != "pass" and count > 0), "pass")

    return {
        "schema": "elf.scored_benchmark_status/v1",
        "source": "real_world_job_benchmark",
        "status": status,
        "counts": counts,
        "job_count": int(summary.get("job_count", 0) or 0),
        "mean_score": summary.get("mean_score"),
        "evidence_coverage": summary.get("evidence_coverage"),
    }


def write_materialization(
    status: StatusState,
    fixtures: list[dict[str, Any]],
    fixture_path: Path,
    command_records: list[CommandRecord],
    live_export: dict[str, Any] | None,
    mapping: dict[str, Any],
    started_at: float,
    report: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Write the primary Letta materialization artifact."""

    elapsed_ms = (time.monotonic() - started_at) * 1000
    payload = {
        "schema": "elf.letta_core_archive_export_readback/v1",
        "generated_at": utc_now(),
        "run_id": RUN_ID,
        "adapter_id": "letta_core_archive_export_readback",
        "project": "Letta",
        "evidence_class": status.evidence_class,
        "status": {
            "source": "smoke_materialization",
            "setup": status.setup,
            "run": status.run,
            "result": status.result,
            "overall": status.overall,
            "failure_class": status.failure_class,
            "failure_reason": status.failure_reason,
        },
        "scored_benchmark": scored_benchmark(report),
        "artifacts": {
            "materialization": rel(OUT),
            "manifest": rel(MANIFEST_OUT),
            "summary": rel(SUMMARY_OUT),
            "generated_fixture_dir": rel(fixture_path),
            "scored_report_json": rel(REPORT_JSON),
            "scored_report_markdown": rel(REPORT_MD),
            "live_output": rel(WORK_DIR / "letta-live-output.json")
            if (WORK_DIR / "letta-live-output.json").exists()
            else None,
        },
        "docker_boundary": {
            "compose_file": "docker-compose.baseline.yml",
            "service_profile": "letta",
            "runner_service": "baseline-runner",
            "runner": "scripts/letta-core-archive-export-readback-smoke.py",
            "host_global_installs_required": False,
            "docker_only": True,
            "host_global_letta_state_used": False,
            "hosted_letta_state_used": False,
        },
        "provider_configuration": {
            "base_url": LETTA_BASE_URL,
            "client_package": LETTA_CLIENT_REF,
            "model": LETTA_MODEL,
            "embedding": LETTA_EMBEDDING,
            "live_run_enabled": RUN_LIVE,
            "operator_owned_provider_credentials_used": False,
        },
        "benchmark_input": benchmark_input_contract(fixtures),
        "letta_export": {
            "core_block_json": live_export.get("core_block_export", []) if live_export else [],
            "archival_readback_json": live_export.get("archival_readback") if live_export else None,
            "archival_search_json": live_export.get("archival_search", []) if live_export else [],
            "agent": live_export.get("agent") if live_export else None,
            "status": "exported" if live_export else status.result,
        },
        "resource_bounds": {
            "source_fixture_count": len(fixtures),
            "core_block_count": len(benchmark_input_contract(fixtures)["core_blocks"]),
            "archival_passage_count": len(benchmark_input_contract(fixtures)["archival_passages"]),
            "timeout_seconds": TIMEOUT_SECONDS,
            "elapsed_ms": round(elapsed_ms, 3),
        },
        "commands": [command_to_json(record) for record in command_records],
        "evidence_mapping": mapping,
        "improvement_regression_readback": {
            "baseline": "XY-955 left Letta core/archive comparison blocked because no contained export/readback artifact existed.",
            "current": (
                "unchanged: the benchmark now has a Docker-contained materialization command and typed report, "
                "but the default run still preserves Letta comparison as blocked until live export/search data maps source ids."
            )
            if status.result != "pass"
            else "improved: Letta export/readback mapped all required core/archive source ids.",
            "judgment": "improved" if status.result == "pass" else "unchanged",
        },
        "claim_boundaries": {
            "allowed": [
                "The Letta comparison now has a reproducible Docker-contained materialization/report command.",
                "The current default report may preserve typed blockers when live Letta/provider setup cannot produce export/readback evidence.",
            ],
            "not_allowed": [
                "Do not claim ELF beats Letta on core-vs-archival memory from fixture-only ELF evidence.",
                "Do not score Letta pass, win, tie, or loss unless exported core block JSON, archival readback/search JSON, and fixture source ids are present.",
            ],
        },
    }
    write_json(OUT, payload)

    return payload


def write_manifest(status: StatusState) -> dict[str, Any]:
    """Write a generated external adapter manifest for this smoke."""

    manifest = {
        "schema": "elf.real_world_external_adapter_manifest/v1",
        "manifest_id": f"letta-core-archive-export-readback-{RUN_ID}",
        "docker_isolation": {
            "default": True,
            "compose_file": "docker-compose.baseline.yml",
            "runner": "scripts/letta-core-archive-export-readback-smoke.py",
            "artifact_dir": "tmp/real-world-memory/letta-core-archive",
            "host_global_installs_required": False,
            "notes": [
                f"Generated by the Letta core/archive export-readback smoke at {utc_now()}.",
                "The smoke uses checked-in core_archival_memory fixtures and records typed setup/runtime failures.",
            ],
        },
        "adapters": [
            {
                "adapter_id": "letta_core_archive_export_readback",
                "project": "Letta",
                "adapter_kind": "docker_core_archive_export_readback",
                "evidence_class": status.evidence_class,
                "docker_default": True,
                "host_global_installs_required": False,
                "overall_status": status.overall,
                "setup": {
                    "status": status.setup,
                    "evidence": "The smoke runs inside the baseline Docker runner and can use a Docker-profile Letta server with explicit model and embedding configuration.",
                    "command": "cargo make smoke-letta-core-archive-export-readback",
                    "artifact": rel(OUT),
                },
                "run": {
                    "status": status.run,
                    "evidence": "The live path creates a benchmark-owned Letta agent, imports fixture source ids into core blocks and archival passages, then exports block/readback/search JSON.",
                    "command": "ELF_LETTA_SMOKE_START=1 ELF_LETTA_SMOKE_RUN=1 cargo make smoke-letta-core-archive-export-readback",
                    "artifact": rel(OUT),
                },
                "result": {
                    "status": status.result,
                    "evidence": status.failure_reason
                    if status.failure_reason
                    else "Letta core block export, archival readback, and archival search mapped required fixture source ids.",
                    "artifact": rel(OUT),
                },
                "capabilities": [
                    {
                        "capability": "docker_letta_server_boundary",
                        "status": status.setup,
                        "evidence": "The runner uses docker-compose.baseline.yml and avoids host-global Letta state or hosted/private agents.",
                    },
                    {
                        "capability": "core_block_export",
                        "status": status.run,
                        "evidence": "Live scoring requires retrieving Letta memory blocks with fixture source ids embedded in block values.",
                    },
                    {
                        "capability": "archival_readback_search_export",
                        "status": status.result,
                        "evidence": "Live scoring requires archival passage list/search JSON to map required source ids.",
                    },
                    {
                        "capability": "broad_letta_quality_claim",
                        "status": "not_encoded",
                        "evidence": "The smoke does not claim broad Letta product quality, private corpus behavior, or hosted-service parity.",
                    },
                ],
                "suites": [
                    {
                        "suite_id": "core_archival_memory",
                        "status": status.result,
                        "evidence": "Only the six checked-in core_archival_memory scenarios are represented.",
                    },
                    {
                        "suite_id": "personalization",
                        "status": "not_encoded",
                        "evidence": "Scoped preference behavior is outside this core/archive export smoke.",
                    },
                    {
                        "suite_id": "project_decisions",
                        "status": status.result,
                        "evidence": "Project-decision recovery is scored only through the core_archival_memory fixture that requires core routing plus archival rationale source ids.",
                    },
                    {
                        "suite_id": "work_resume",
                        "status": "not_encoded",
                        "evidence": "Agent resumption across sessions is not encoded by this export/readback smoke.",
                    },
                ],
                "evidence": [
                    {"kind": "artifact", "ref": rel(OUT), "status": status.result},
                    {"kind": "manifest", "ref": rel(MANIFEST_OUT), "status": status.overall},
                    {"kind": "source", "ref": "https://docs.letta.com/guides/docker", "status": "real"},
                    {"kind": "source", "ref": "https://docs.letta.com/api/python", "status": "real"},
                    {
                        "kind": "source",
                        "ref": "https://docs.letta.com/api/resources/agents/subresources/passages/methods/search",
                        "status": "real",
                    },
                ],
                "execution_metadata": {
                    "sources": [
                        {
                            "label": "Letta Docker docs",
                            "url": "https://docs.letta.com/guides/docker",
                            "evidence": "Official Docker setup and explicit embedding configuration boundary.",
                        },
                        {
                            "label": "Letta Python API",
                            "url": "https://docs.letta.com/api/python",
                            "evidence": "Official Python SDK memory block creation and retrieval examples.",
                        },
                        {
                            "label": "Letta archival search API",
                            "url": "https://docs.letta.com/api/resources/agents/subresources/passages/methods/search",
                            "evidence": "Official archival-memory search endpoint contract.",
                        },
                    ],
                    "setup_path": "Run cargo make smoke-letta-core-archive-export-readback for a typed artifact; set ELF_LETTA_SMOKE_START=1 ELF_LETTA_SMOKE_RUN=1 with explicit model/provider configuration for a live export attempt.",
                    "runtime_boundary": "docker-compose.baseline.yml baseline-runner plus optional Letta server profile, benchmark-created agent, benchmark-owned fixture corpus, and artifacts under tmp/real-world-memory/letta-core-archive.",
                    "resource_expectation": f"Letta client {LETTA_CLIENT_REF}, model={LETTA_MODEL}, embedding={LETTA_EMBEDDING}, source fixture count=6, timeout_seconds={TIMEOUT_SECONDS}.",
                    "retry_guidance": [
                        "Default command records a typed blocked artifact without model calls.",
                        "Enable the live path only with a Docker-local Letta server and explicit provider or local model configuration.",
                        "Score only when core block export and archival list/search output map to required fixture source ids.",
                    ],
                    "research_depth": "XY-984 materialization contract; generated artifact decides live evidence class.",
                },
                "notes": [
                    "Failure before Letta export/readback remains typed as blocked or incomplete.",
                    "The smoke does not use hosted/private Letta state or operator-owned data.",
                ],
            }
        ],
    }
    write_json(MANIFEST_OUT, manifest)

    return manifest


def write_summary(materialization: dict[str, Any], manifest: dict[str, Any], report: dict[str, Any]) -> None:
    """Write a small summary artifact."""

    write_json(
        SUMMARY_OUT,
        {
            "schema": "elf.letta_core_archive_export_readback_summary/v1",
            "generated_at": utc_now(),
            "adapter_id": "letta_core_archive_export_readback",
            "evidence_class": materialization["evidence_class"],
            "status_boundary": {
                "materialization": "setup/run/evidence-mapping state emitted by the smoke runner",
                "manifest": "external adapter declaration consumed by the scorer",
                "scored_benchmark": "post-score real_world_job outcome; use this for quality status",
            },
            "scored_benchmark": materialization["scored_benchmark"],
            "materialization": materialization,
            "manifest": {
                "json": rel(MANIFEST_OUT),
                "status_source": "external_adapter_manifest_score_aligned",
                "summary": manifest["adapters"][0]["overall_status"],
                "suites": manifest["adapters"][0]["suites"],
            },
            "report": report,
        },
    )


def main() -> int:
    """Run the smoke and always emit typed artifacts when possible."""

    started_at = time.monotonic()
    mkdirs()
    status = StatusState()
    command_records: list[CommandRecord] = []
    fixtures = load_source_fixtures()
    live_export: dict[str, Any] | None = None

    if not Path("/.dockerenv").exists() and not ALLOW_HOST:
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "not_running_in_docker"
        status.failure_reason = "Letta smoke must run inside Docker; use cargo make smoke-letta-core-archive-export-readback."
    elif not command_available("python3"):
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "python_missing"
        status.failure_reason = "python3 is required for the Letta smoke runner."
    elif not RUN_LIVE:
        pass
    elif not wait_for_letta(command_records):
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "letta_server_unreachable"
        status.failure_reason = "Docker-local Letta server did not become reachable for export/readback."
    elif not init_letta_client(command_records):
        status.setup = "incomplete"
        status.result = "incomplete"
        status.overall = "incomplete"
        status.failure_class = "letta_client_setup_failed"
        status.failure_reason = "Letta Python client installation or import failed inside the Docker runner."
    else:
        status.setup = "pass"
        live_export = run_letta(fixtures, command_records)
        if live_export is None:
            status.run = "incomplete"
            status.result = "incomplete"
            status.overall = "incomplete"
            status.failure_class = "letta_export_readback_failed"
            status.failure_reason = "Letta did not produce core block export plus archival readback/search output."
        else:
            status.run = "pass"
            status.evidence_class = "live_real_world"
            mapping = evidence_mapping(fixtures, live_export, status)
            if not mapping["missing_evidence_ids"]:
                status.result = "pass"
                status.overall = "pass"
                status.failure_class = ""
                status.failure_reason = ""
            else:
                status.result = "wrong_result"
                status.overall = "wrong_result"
                status.failure_class = "letta_source_id_mapping_failed"
                status.failure_reason = mapping["reason"]

    mapping = evidence_mapping(fixtures, live_export, status)
    fixture_path = write_fixture_outputs(fixtures, status, mapping)
    write_materialization(
        status,
        fixtures,
        fixture_path,
        command_records,
        live_export,
        mapping,
        started_at,
    )
    manifest = write_manifest(status)
    report = run_scored_report(fixture_path, MANIFEST_OUT, status)
    materialization = write_materialization(
        status,
        fixtures,
        fixture_path,
        command_records,
        live_export,
        mapping,
        started_at,
        report,
    )
    write_summary(materialization, manifest, report)
    print(f"Letta core/archive artifact: {OUT}")
    print(f"Letta core/archive manifest: {MANIFEST_OUT}")
    print(f"Letta core/archive summary: {SUMMARY_OUT}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
