"""Docker-contained adapter for VectifyAI OpenKB's native query agent."""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import time
from pathlib import Path
from typing import Any


OPENKB_REVISION = "0d905e40afa6f416c616f00c56a4cd0fd995a787"
OPENKB_REPO = Path(os.environ.get("OPENKB_REPO_DIR", "/opt/openkb"))
OPENKB_PYTHON = Path(
    os.environ.get("OPENKB_PYTHON", "/opt/openkb-venv/bin/python")
)
FORBIDDEN_PRODUCT_KEYS = {
    "expected_answer",
    "required_evidence",
    "negative_traps",
    "qrels",
    "scoring_rubric",
}


class OpenKBProductFailure(RuntimeError):
    """The pinned OpenKB runtime failed after the adapter reached it."""


_QUERY_PROGRAM = r"""
import asyncio
import json
import os
import sys
from pathlib import Path

from agents import ModelSettings, Runner, set_tracing_disabled
from agents.models.openai_chatcompletions import OpenAIChatCompletionsModel
from openai import AsyncOpenAI
from openai.types.shared.reasoning import Reasoning
from openkb.agent.query import MAX_TURNS, build_query_agent
from openkb.config import load_config


def jsonable(value):
    if value is None or isinstance(value, (bool, int, float, str)):
        return value
    if isinstance(value, dict):
        return {str(key): jsonable(child) for key, child in value.items()}
    if isinstance(value, (list, tuple)):
        return [jsonable(child) for child in value]
    if hasattr(value, "model_dump"):
        return jsonable(value.model_dump(mode="json"))
    if hasattr(value, "__dict__"):
        return jsonable(vars(value))
    return str(value)


async def main():
    kb_dir = Path(sys.argv[1])
    question = sys.argv[2]
    config = load_config(kb_dir / ".openkb" / "config.yaml")
    model = str(config["model"])
    language = str(config.get("language", "en"))
    agent = build_query_agent(str(kb_dir / "wiki"), model, language=language)
    reasoning_effort = os.environ.get("CHAT_REASONING_EFFORT", "high")
    model_settings = agent.model_settings.resolve(
        ModelSettings(reasoning=Reasoning(effort=reasoning_effort))
    )
    agent = agent.clone(
        model=OpenAIChatCompletionsModel(
            model=os.environ["CHAT_MODEL"],
            openai_client=AsyncOpenAI(
                base_url=os.environ["OPENAI_BASE_URL"],
                api_key=os.environ["OPENAI_API_KEY"],
            ),
        ),
        model_settings=model_settings,
    )
    result = await Runner.run(agent, question, max_turns=MAX_TURNS)
    print(
        json.dumps(
            {
                "final_output": result.final_output or "",
                "input_items": jsonable(result.to_input_list()),
                "transport": "openai_chat_completions",
            },
            ensure_ascii=True,
            separators=(",", ":"),
            sort_keys=True,
        )
    )


set_tracing_disabled(True)
asyncio.run(main())
"""


def _write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _subprocess_text(value: str | bytes | None) -> str:
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    return value or ""


def _bounded_failure_detail(
    stdout: str, stderr: str, env: dict[str, str], limit: int = 4000
) -> str:
    """Keep typed failures concise while retaining the full native log on disk."""
    detail = (stderr or stdout).strip()
    for name, value in env.items():
        if value and re.search(r"(?:KEY|TOKEN|SECRET|PASSWORD)", name, re.IGNORECASE):
            detail = detail.replace(value, "<redacted>")
    if len(detail) > limit:
        detail = detail[:limit].rstrip() + "\n...[truncated; inspect the preserved native log]"
    return detail


def _load_jobs(input_dir: Path) -> list[dict[str, Any]]:
    return [
        json.loads(path.read_text(encoding="utf-8"))
        for path in sorted(input_dir.glob("*.json"))
    ]


def assert_qrel_blind(value: Any, path: str = "$") -> None:
    """Reject evaluator-only fields before OpenKB can observe a payload."""
    if isinstance(value, dict):
        leaked = FORBIDDEN_PRODUCT_KEYS.intersection(value)
        if leaked:
            raise ValueError(
                f"OpenKB product payload leaks evaluator keys at {path}: "
                + ", ".join(sorted(leaked))
            )
        for key, child in value.items():
            assert_qrel_blind(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            assert_qrel_blind(child, f"{path}[{index}]")


def _verify_revision() -> None:
    completed = subprocess.run(
        ["git", "-C", str(OPENKB_REPO), "rev-parse", "HEAD"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if completed.returncode or completed.stdout.strip() != OPENKB_REVISION:
        raise RuntimeError("OpenKB checkout does not match the frozen revision")


def _safe_name(value: str) -> str:
    stem = re.sub(r"[^A-Za-z0-9._-]+", "-", value).strip(".-") or "evidence"
    suffix = hashlib.sha256(value.encode("utf-8")).hexdigest()[:12]
    return f"{stem[:80]}-{suffix}.md"


def _opaque_source_name(job_id: str, evidence_id: str) -> str:
    digest = hashlib.sha256(f"{job_id}:{evidence_id}".encode("utf-8")).hexdigest()
    return f"source-{digest[:24]}.md"


def _render_source(text: str) -> str:
    return f"{text.rstrip()}\n"


def _materialize_sources(
    jobs: list[dict[str, Any]], source_dir: Path
) -> dict[str, str]:
    source_dir.mkdir(parents=True, exist_ok=True)
    hashes: dict[str, str] = {}
    for job in jobs:
        job_id = str(job["job_id"])
        for item in job["corpus"]["items"]:
            evidence_id = str(item["evidence_id"])
            rendered = _render_source(str(item["text"]))
            path = source_dir / _opaque_source_name(job_id, evidence_id)
            path.write_text(rendered, encoding="utf-8")
            source_hash = hashlib.sha256(path.read_bytes()).hexdigest()
            if source_hash in hashes and hashes[source_hash] != evidence_id:
                raise ValueError("OpenKB cannot distinguish duplicate source content")
            hashes[source_hash] = evidence_id
    return hashes


def _litellm_model(model: str) -> str:
    """Select LiteLLM's OpenAI transport for an operator-defined proxy alias."""
    return model if "/" in model else f"openai/{model}"


def _configure_litellm_api_base(kb_dir: Path, api_base: str) -> None:
    """Use OpenKB's native LiteLLM settings for the isolated proxy route."""
    config_path = kb_dir / ".openkb" / "config.yaml"
    current = config_path.read_text(encoding="utf-8")
    if "\nlitellm:" in f"\n{current}":
        raise ValueError("OpenKB init unexpectedly supplied a LiteLLM config block")
    config_path.write_text(
        current.rstrip()
        + "\nlitellm:\n"
        + f"  api_base: {json.dumps(api_base, ensure_ascii=True)}\n",
        encoding="utf-8",
    )


def _openkb_env() -> dict[str, str]:
    env = os.environ.copy()
    api_key = env.get("OPENAI_API_KEY") or env.get("CHAT_API_KEY") or ""
    api_base = env.get("OPENAI_BASE_URL") or env.get("CHAT_API_BASE") or ""
    if not api_key or not api_base:
        raise ValueError("OpenKB requires an OpenAI-compatible chat API key and base URL")
    env.update(
        {
            "HOME": env.get("OPENKB_HOME", "/tmp/openkb-home"),
            "LITELLM_LOCAL_MODEL_COST_MAP": "True",
            "LLM_API_KEY": api_key,
            "OPENAI_API_KEY": api_key,
            "OPENAI_API_BASE": api_base,
            "OPENAI_BASE_URL": api_base,
            "NO_COLOR": "1",
            "PYTHONPATH": str(OPENKB_REPO),
        }
    )
    return env


def _run_native(
    command: list[str],
    *,
    cwd: Path,
    env: dict[str, str],
    stdout_path: Path,
    stderr_path: Path,
    timeout: int,
    stdin_text: str | None = None,
) -> tuple[str, float]:
    started = time.monotonic()
    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=timeout,
            input=stdin_text,
        )
    except subprocess.TimeoutExpired as error:
        stdout_path.parent.mkdir(parents=True, exist_ok=True)
        stdout_path.write_text(_subprocess_text(error.stdout), encoding="utf-8")
        stderr_path.write_text(_subprocess_text(error.stderr), encoding="utf-8")
        raise OpenKBProductFailure("OpenKB native operation timed out") from error
    elapsed_ms = (time.monotonic() - started) * 1000.0
    stdout_path.parent.mkdir(parents=True, exist_ok=True)
    stdout_path.write_text(completed.stdout, encoding="utf-8")
    stderr_path.write_text(completed.stderr, encoding="utf-8")
    if completed.returncode:
        detail = _bounded_failure_detail(completed.stdout, completed.stderr, env)
        raise OpenKBProductFailure(
            f"OpenKB native operation exited {completed.returncode}: {detail}"
        )
    return completed.stdout, elapsed_ms


def _state_sha256(root: Path) -> str:
    digest = hashlib.sha256()
    candidates = [
        root / ".openkb" / "config.yaml",
        root / ".openkb" / "hashes.json",
    ]
    candidates.extend(
        candidate for candidate in (root / "wiki").rglob("*") if candidate.is_file()
    )
    for path in sorted(candidate for candidate in candidates if candidate.is_file()):
        digest.update(path.relative_to(root).as_posix().encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def _native_document_map(
    registry: dict[str, Any], expected_hashes: dict[str, str]
) -> dict[str, str]:
    mapping: dict[str, str] = {}
    missing = set(expected_hashes)
    for file_hash, evidence_id in expected_hashes.items():
        metadata = registry.get(file_hash)
        if not isinstance(metadata, dict) or not metadata.get("doc_name"):
            continue
        doc_name = str(metadata["doc_name"])
        mapping[f"summaries/{doc_name}.md"] = evidence_id
        mapping[f"sources/{doc_name}.md"] = evidence_id
        mapping[f"sources/{doc_name}.json"] = evidence_id
        missing.discard(file_hash)
    if missing:
        raise OpenKBProductFailure(
            f"OpenKB native hash registry omitted {len(missing)} ingested source(s)"
        )
    return mapping


def _tool_calls(value: Any) -> list[dict[str, Any]]:
    calls: list[dict[str, Any]] = []
    if isinstance(value, dict):
        if value.get("type") in {"function_call", "tool_call"} and isinstance(
            value.get("name"), str
        ):
            arguments = value.get("arguments")
            if isinstance(arguments, str):
                try:
                    arguments = json.loads(arguments)
                except json.JSONDecodeError:
                    arguments = {}
            calls.append(
                {
                    "name": value["name"],
                    "arguments": arguments if isinstance(arguments, dict) else {},
                }
            )
        for child in value.values():
            calls.extend(_tool_calls(child))
    elif isinstance(value, list):
        for child in value:
            calls.extend(_tool_calls(child))
    return calls


def _frontmatter_sources(path: Path) -> list[str]:
    """Parse OpenKB's JSON-style `sources` frontmatter, never page body text."""
    if not path.is_file():
        return []
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        return []
    closing = text.find("\n---", 4)
    if closing < 0:
        return []
    for line in text[4:closing].splitlines():
        if line.startswith("sources:"):
            try:
                value = json.loads(line.split(":", 1)[1].strip())
            except json.JSONDecodeError:
                return []
            if isinstance(value, list):
                return [item for item in value if isinstance(item, str)]
            return []
    return []


def _resolve_source_path(
    wiki_root: Path,
    path: str,
    direct_map: dict[str, str],
    visited: set[str] | None = None,
) -> list[str]:
    if path in direct_map:
        return [direct_map[path]]
    visited = set() if visited is None else visited
    if path in visited:
        return []
    visited.add(path)
    output: list[str] = []
    for source in _frontmatter_sources(wiki_root / path):
        for evidence_id in _resolve_source_path(wiki_root, source, direct_map, visited):
            if evidence_id not in output:
                output.append(evidence_id)
    return output


def native_source_trace(
    native_output: dict[str, Any],
    wiki_root: Path,
    direct_map: dict[str, str],
) -> tuple[list[str], list[dict[str, Any]]]:
    """Map only native OpenKB tool calls and native page source metadata."""
    evidence_ids: list[str] = []
    trace: list[dict[str, Any]] = []
    for call in _tool_calls(native_output.get("input_items")):
        name = call["name"]
        arguments = call["arguments"]
        path: str | None = None
        if name == "read_file" and isinstance(arguments.get("path"), str):
            path = arguments["path"]
        elif name == "get_page_content" and isinstance(
            arguments.get("doc_name"), str
        ):
            path = f"sources/{arguments['doc_name']}.json"
        if path is None:
            continue
        mapped = _resolve_source_path(wiki_root, path, direct_map)
        trace.append(
            {
                "tool": name,
                "arguments": arguments,
                "wiki_path": path,
                "evidence_ids": mapped,
            }
        )
        for evidence_id in mapped:
            if evidence_id not in evidence_ids:
                evidence_ids.append(evidence_id)
    return evidence_ids, trace


def _failure_class(message: str, default: str) -> str:
    lowered = message.lower()
    if re.search(r"(?:error code|status):\s*[45]\d\d", lowered) or any(
        token in lowered for token in ("401", "403", "429", "rate limit")
    ):
        return "provider_failed"
    if any(
        token in lowered
        for token in ("api connection", "openai", "litellm", "provider")
    ):
        return "provider_failed"
    return default


def _phase_status(rows: list[dict[str, Any]]) -> str:
    for classification in (
        "configuration_failed",
        "provider_failed",
        "product_failed",
        "adapter_failed",
    ):
        if any(row["classification"] == classification for row in rows):
            return classification
    return "completed"


def _terminal_result(
    jobs: list[dict[str, Any]], classification: str, reason: str
) -> dict[str, Any]:
    def phase() -> dict[str, Any]:
        return {
            "status": classification,
            "jobs": [
                {
                    "job_id": job.get("job_id", "unknown"),
                    "classification": classification,
                    "evidence_ids": [],
                    "returned_count": 0,
                    "latency_ms": 0.0,
                    "native_status": "not_run",
                    "failure": reason,
                }
                for job in jobs
            ],
            "adapter_metadata": {"index_reused": False},
        }

    return {
        "schema": "elf.benchmark_unit_result/v4",
        "target": "openkb",
        "native_mode": "agent_query_with_native_source_trace",
        "score_eligible": True,
        "result_class": classification,
        "terminal_reason": reason,
        "warm_reused_state": False,
        "ingest_count": 0,
        "phases": {"cold": phase(), "warm": phase()},
    }


def _run_phase(
    phase: str,
    jobs: list[dict[str, Any]],
    *,
    kb_dir: Path,
    artifacts: Path,
    env: dict[str, str],
    direct_map: dict[str, str],
    timeout: int,
) -> dict[str, Any]:
    rows: list[dict[str, Any]] = []
    for job in jobs:
        job_id = str(job["job_id"])
        raw_stem = f"openkb-{phase}-{_safe_name(job_id)[:-3]}"
        stdout_path = artifacts / "raw" / f"{raw_stem}.json"
        stderr_path = artifacts / "raw" / f"{raw_stem}.stderr.log"
        started = time.monotonic()
        native_status = "not_run"
        try:
            stdout, latency_ms = _run_native(
                [
                    str(OPENKB_PYTHON),
                    "-c",
                    _QUERY_PROGRAM,
                    str(kb_dir),
                    str(job["prompt"]["content"]),
                ],
                cwd=OPENKB_REPO,
                env=env,
                stdout_path=stdout_path,
                stderr_path=stderr_path,
                timeout=timeout,
            )
            native_status = "completed"
            native_output = json.loads(stdout)
            answer = native_output.get("final_output")
            evidence_ids, trace = native_source_trace(
                native_output, kb_dir / "wiki", direct_map
            )
            _write_json(
                artifacts / "raw" / f"{raw_stem}.source-trace.json", trace
            )
            if not isinstance(answer, str) or not answer.strip():
                classification = "product_failed"
                failure = "OpenKB native query returned an empty answer"
            elif not evidence_ids:
                classification = "product_failed"
                failure = (
                    "OpenKB native query returned no source-bearing tool trace; "
                    "provenance was not inferred from answer text"
                )
            else:
                classification = "completed"
                failure = None
        except OpenKBProductFailure as error:
            latency_ms = (time.monotonic() - started) * 1000.0
            classification = _failure_class(str(error), "product_failed")
            evidence_ids = []
            failure = str(error)
            native_status = "failed"
        except (json.JSONDecodeError, KeyError, TypeError, ValueError) as error:
            latency_ms = (time.monotonic() - started) * 1000.0
            classification = "adapter_failed"
            evidence_ids = []
            failure = f"OpenKB native output or trace was malformed: {error}"
        rows.append(
            {
                "job_id": job_id,
                "classification": classification,
                "evidence_ids": evidence_ids,
                "returned_count": len(evidence_ids),
                "latency_ms": round(latency_ms, 3),
                "native_status": native_status,
                "failure": failure,
                "raw_native_output": str(stdout_path),
            }
        )
    return {
        "status": _phase_status(rows),
        "jobs": rows,
        "adapter_metadata": {"index_reused": phase == "warm"},
    }


def run_openkb(input_dir: Path, artifacts: Path, state_dir: Path) -> dict[str, Any]:
    """Ingest once, then run cold and warm OpenKB queries over exact state."""
    jobs: list[dict[str, Any]] = []
    try:
        _verify_revision()
        jobs = _load_jobs(input_dir)
        if not jobs:
            raise ValueError("OpenKB received no product fixtures")
        for job in jobs:
            assert_qrel_blind(job)
        env = _openkb_env()
        configured_model = env.get("OPENKB_MODEL") or env.get("CHAT_MODEL")
        if not configured_model:
            raise ValueError("OpenKB requires OPENKB_MODEL or CHAT_MODEL")
        model = _litellm_model(configured_model)
        receipt_path = state_dir / "cold-ingest.json"
        if receipt_path.exists():
            raise ValueError("OpenKB cold state already has an ingest receipt")

        kb_dir = state_dir / "kb"
        source_dir = state_dir / "sources"
        kb_dir.mkdir(parents=True, exist_ok=True)
        expected_hashes = _materialize_sources(jobs, source_dir)
        timeout = int(env.get("OPENKB_TIMEOUT_SECONDS", "300"))

        _run_native(
            [
                str(OPENKB_PYTHON),
                "-m",
                "openkb",
                "init",
                "--model",
                model,
                "--language",
                "en",
            ],
            cwd=kb_dir,
            env=env,
            stdout_path=artifacts / "raw" / "openkb-init.stdout.log",
            stderr_path=artifacts / "raw" / "openkb-init.stderr.log",
            timeout=timeout,
            stdin_text="\n",
        )
        _configure_litellm_api_base(kb_dir, env["OPENAI_BASE_URL"])
        ingest_stdout, ingest_latency_ms = _run_native(
            [str(OPENKB_PYTHON), "-m", "openkb", "add", str(source_dir)],
            cwd=kb_dir,
            env=env,
            stdout_path=artifacts / "raw" / "openkb-ingest.stdout.log",
            stderr_path=artifacts / "raw" / "openkb-ingest.stderr.log",
            timeout=timeout,
        )
        failed_adds = ingest_stdout.count("[ERROR] add failed")
        if failed_adds:
            provider_markers = ("litellm", "provider", "openai", "api connection")
            cause = (
                "provider failure"
                if any(marker in ingest_stdout.lower() for marker in provider_markers)
                else "product failure"
            )
            raise OpenKBProductFailure(
                f"OpenKB native add reported {failed_adds} {cause}(s); "
                "inspect the preserved native ingest log"
            )

        registry_path = kb_dir / ".openkb" / "hashes.json"
        if not registry_path.is_file():
            raise OpenKBProductFailure("OpenKB ingest produced no native hash registry")
        registry = json.loads(registry_path.read_text(encoding="utf-8"))
        if not isinstance(registry, dict):
            raise OpenKBProductFailure("OpenKB native hash registry is malformed")
        direct_map = _native_document_map(registry, expected_hashes)
        _write_json(artifacts / "raw" / "openkb-hashes.json", registry)

        state_hash = _state_sha256(kb_dir)
        receipt = {
            "revision": OPENKB_REVISION,
            "native_state_sha256": state_hash,
            "source_count": len(expected_hashes),
        }
        _write_json(receipt_path, receipt)

        cold = _run_phase(
            "cold",
            jobs,
            kb_dir=kb_dir,
            artifacts=artifacts,
            env=env,
            direct_map=direct_map,
            timeout=timeout,
        )
        if _state_sha256(kb_dir) != state_hash:
            return _terminal_result(
                jobs,
                "adapter_failed",
                "OpenKB cold query mutated the sealed native ingest state",
            )
        warm = _run_phase(
            "warm",
            jobs,
            kb_dir=kb_dir,
            artifacts=artifacts,
            env=env,
            direct_map=direct_map,
            timeout=timeout,
        )
        if (
            json.loads(receipt_path.read_text(encoding="utf-8")) != receipt
            or _state_sha256(kb_dir) != state_hash
        ):
            return _terminal_result(
                jobs,
                "adapter_failed",
                "OpenKB warm query did not reuse the exact sealed cold state",
            )
    except ValueError as error:
        return _terminal_result(jobs, "configuration_failed", str(error))
    except OpenKBProductFailure as error:
        classification = _failure_class(str(error), "product_failed")
        return _terminal_result(jobs, classification, str(error))
    except (OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        return _terminal_result(jobs, "adapter_failed", str(error))

    result_class = _phase_status(cold["jobs"] + warm["jobs"])
    return {
        "schema": "elf.benchmark_unit_result/v4",
        "target": "openkb",
        "native_mode": "agent_query_with_native_source_trace",
        "score_eligible": True,
        "result_class": result_class,
        "warm_reused_state": True,
        "ingest_count": 1,
        "ingest_duration_ms": round(ingest_latency_ms, 3),
        "phases": {
            "cold": {
                **cold,
                "adapter_metadata": {
                    **cold["adapter_metadata"],
                    "ingest_latency_ms": round(ingest_latency_ms, 3),
                    "native_state_sha256": state_hash,
                },
            },
            "warm": {
                **warm,
                "adapter_metadata": {
                    **warm["adapter_metadata"],
                    "native_state_sha256": state_hash,
                },
            },
        },
    }
