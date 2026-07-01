"""Runtime setup and live Letta execution."""

from __future__ import annotations

import json
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any

from .common import run_command, write_json
from .context import (
    INSTALL_CLIENT,
    LETTA_BASE_URL,
    LETTA_CLIENT_PACKAGE,
    LETTA_EMBEDDING,
    LETTA_MODEL,
    RUN_ID,
    STARTUP_ATTEMPTS,
    STARTUP_INTERVAL_SECONDS,
    WORK_DIR,
)
from .fixtures import benchmark_input_contract, slug
from .models import CommandRecord

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
