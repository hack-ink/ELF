"""Docker-contained adapter for Zleap-AI SAG native retrieval."""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import time
import uuid
from pathlib import Path
from typing import Any


SAG_REVISION = "84a5b8c9bd45944b8a3cd76e0ba4762ce68ccc72"
SAG_REPO = Path(os.environ.get("SAG_REPO_DIR", "/opt/sag"))
SAG_TSX = Path(os.environ.get("SAG_TSX", "/opt/sag/node_modules/.bin/tsx"))
FORBIDDEN_PRODUCT_KEYS = {
    "expected_answer",
    "negative_traps",
    "qrels",
    "required_evidence",
    "scoring_rubric",
}
REQUIRED_PROVIDER_VALUES = {
    "CHAT_MODEL": "gpt-5.6-luna",
    "CHAT_REASONING_EFFORT": "high",
    "EMBEDDING_MODEL": "Qwen3-Embedding-8B",
    "EMBEDDING_DIMENSIONS": "1024",
}


class SagProductFailure(RuntimeError):
    """The pinned SAG runtime failed after the adapter reached it."""


class SagAdapterFailure(RuntimeError):
    """The adapter input, configuration, or state contract is invalid."""


class SagConfigurationFailure(SagAdapterFailure):
    """The SAG unit does not have its required frozen configuration."""


_NATIVE_RUNNER = r"""
import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import { ingestionService } from "/opt/sag/src/services/ingestion-service.js";
import { searchService } from "/opt/sag/src/services/search-service.js";
import { migrate } from "/opt/sag/src/db/migrate.js";
import { seed } from "/opt/sag/src/db/seed.js";
import { pool, closePool } from "/opt/sag/src/db/pool.js";

const inputPath = process.argv[2];
const outputPath = process.argv[3];

async function stateDigest(sourceIds) {
  const queries = [
    ["sources", "select * from sources where id = any($1::uuid[]) order by id"],
    ["documents", "select * from documents where source_id = any($1::uuid[]) order by id"],
    ["document_sections", `
      select ds.* from document_sections ds
      join documents d on d.id = ds.document_id
      where d.source_id = any($1::uuid[]) order by ds.id
    `],
    ["source_chunks", "select * from source_chunks where source_id = any($1::uuid[]) order by id"],
    ["entities", "select * from entities where source_id = any($1::uuid[]) order by id"],
    ["events", "select * from events where source_id = any($1::uuid[]) order by id"],
    ["event_entities", `
      select ee.* from event_entities ee
      join events e on e.id = ee.event_id
      where e.source_id = any($1::uuid[]) order by ee.id
    `]
  ];
  const hash = createHash("sha256");
  const counts = {};
  for (const [name, sql] of queries) {
    const result = await pool.query(sql, [sourceIds]);
    counts[name] = result.rows.length;
    hash.update(name);
    hash.update("\0");
    hash.update(JSON.stringify(result.rows));
    hash.update("\0");
  }
  return { sha256: hash.digest("hex"), counts };
}

async function searchJobs(jobs) {
  const results = [];
  for (const job of jobs) {
    const started = performance.now();
    const search = await searchService.search({
      query: job.query,
      sourceIds: job.items.map((item) => item.sourceId),
      strategy: "multi",
      searchMode: "standard",
      topK: 5,
      returnTrace: true,
      multi: {
        entityTopK: 8,
        multiTopK: 8,
        maxEvents: 8,
        rerankTopK: 5,
        maxSections: 5,
        maxHops: 2
      }
    });
    results.push({
      jobId: job.jobId,
      latencyMs: performance.now() - started,
      search
    });
  }
  return results;
}

async function main() {
  const input = JSON.parse(await readFile(inputPath, "utf8"));
  const sourceIds = input.jobs.flatMap((job) => job.items.map((item) => item.sourceId));
  await migrate();
  await seed();

  const existing = await pool.query(
    "select id from sources where id = any($1::uuid[]) limit 1",
    [sourceIds]
  );
  if (existing.rowCount) {
    throw new Error("SAG cold state already contains a benchmark source");
  }

  const ingest = [];
  const ingestStarted = performance.now();
  for (const job of input.jobs) {
    for (const item of job.items) {
      const result = await ingestionService.ingestDocument({
        sourceId: item.sourceId,
        title: item.title,
        content: item.content,
        metadata: { elfBenchmarkSourceKey: item.sourceKey },
        extract: true,
        chunking: { mode: "heading_strict" }
      });
      ingest.push({ jobId: job.jobId, sourceKey: item.sourceKey, result });
    }
  }

  const ingestLatencyMs = performance.now() - ingestStarted;
  const stateBeforeCold = await stateDigest(sourceIds);
  const cold = await searchJobs(input.jobs);
  const stateAfterCold = await stateDigest(sourceIds);
  const warm = await searchJobs(input.jobs);
  const stateAfterWarm = await stateDigest(sourceIds);
  await writeFile(outputPath, JSON.stringify({
    ingest,
    ingestLatencyMs,
    stateBeforeCold,
    cold,
    stateAfterCold,
    warm,
    stateAfterWarm
  }, null, 2) + "\n");
}

main()
  .then(async () => closePool())
  .catch(async (error) => {
    console.error(error instanceof Error ? error.stack || error.message : String(error));
    await closePool();
    process.exit(1);
  });
"""


def _write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _assert_qrel_blind(value: Any, path: str = "$") -> None:
    if isinstance(value, dict):
        leaked = FORBIDDEN_PRODUCT_KEYS.intersection(value)
        if leaked:
            raise SagAdapterFailure(
                f"SAG fixture leaks evaluator keys at {path}: "
                + ", ".join(sorted(leaked))
            )
        for key, child in value.items():
            _assert_qrel_blind(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            _assert_qrel_blind(child, f"{path}[{index}]")


def _load_jobs(input_dir: Path) -> list[dict[str, Any]]:
    jobs: list[dict[str, Any]] = []
    for path in sorted(input_dir.glob("*.json")):
        try:
            job = json.loads(path.read_text(encoding="utf-8"))
            _assert_qrel_blind(job)
            if not isinstance(job["job_id"], str) or not job["job_id"]:
                raise TypeError("job_id must be a non-empty string")
            if not isinstance(job["prompt"]["content"], str):
                raise TypeError("prompt.content must be a string")
            items = job["corpus"]["items"]
            if not isinstance(items, list) or not items:
                raise TypeError("corpus.items must be a non-empty list")
            for item in items:
                if not isinstance(item["evidence_id"], str) or not isinstance(
                    item["text"], str
                ):
                    raise TypeError("corpus evidence_id and text must be strings")
        except SagAdapterFailure:
            raise
        except (KeyError, TypeError, json.JSONDecodeError) as error:
            raise SagAdapterFailure(
                f"invalid SAG fixture {path.name}: {error}"
            ) from error
        jobs.append(job)
    if not jobs:
        raise SagAdapterFailure("SAG received no product fixtures")
    return jobs


def _verify_revision() -> None:
    completed = subprocess.run(
        ["git", "-C", str(SAG_REPO), "rev-parse", "HEAD"],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if completed.returncode or completed.stdout.strip() != SAG_REVISION:
        raise SagAdapterFailure("SAG checkout does not match the frozen revision")


def _required_environment() -> dict[str, str]:
    required = (
        "DATABASE_URL",
        "CHAT_API_BASE",
        "CHAT_API_KEY",
        "CHAT_MODEL",
        "CHAT_REASONING_EFFORT",
        "EMBEDDING_API_BASE",
        "EMBEDDING_API_KEY",
        "EMBEDDING_MODEL",
        "EMBEDDING_DIMENSIONS",
    )
    missing = [name for name in required if not os.environ.get(name)]
    if missing:
        raise SagConfigurationFailure(
            "missing SAG configuration: " + ", ".join(missing)
        )
    for name, expected in REQUIRED_PROVIDER_VALUES.items():
        if os.environ[name] != expected:
            raise SagConfigurationFailure(f"SAG requires {name}={expected}")
    return {name: os.environ[name] for name in required}


def _source_identity(job_id: str, index: int) -> tuple[str, str]:
    identity = f"{job_id}:{index}"
    digest = hashlib.sha256(identity.encode("utf-8")).hexdigest()[:16]
    source_key = f"item-{index:04d}-{digest}"
    source_id = str(uuid.uuid5(uuid.NAMESPACE_URL, f"elf-sag:{identity}"))
    return source_id, source_key


def _native_input(jobs: list[dict[str, Any]]) -> tuple[dict[str, Any], dict[str, str]]:
    source_map: dict[str, str] = {}
    native_jobs: list[dict[str, Any]] = []
    for job in jobs:
        native_items = []
        for index, item in enumerate(job["corpus"]["items"]):
            source_id, source_key = _source_identity(job["job_id"], index)
            if source_id in source_map:
                raise SagAdapterFailure("SAG generated a duplicate native source id")
            source_map[source_id] = item["evidence_id"]
            native_items.append(
                {
                    "sourceId": source_id,
                    "sourceKey": source_key,
                    "title": f"{job['job_id']} source {index + 1}",
                    "content": item["text"],
                }
            )
        native_jobs.append(
            {
                "jobId": job["job_id"],
                "query": job["prompt"]["content"],
                "items": native_items,
            }
        )
    payload = {"jobs": native_jobs}
    _assert_qrel_blind(payload)
    return payload, source_map


def evidence_ids_from_search(
    native_search: Any, source_map: dict[str, str]
) -> list[str]:
    """Map evidence only through SAG's explicit SearchSection.sourceId field."""
    if not isinstance(native_search, dict):
        return []
    sections = native_search.get("sections")
    if not isinstance(sections, list):
        return []
    output: list[str] = []
    for section in sections:
        if not isinstance(section, dict):
            continue
        source_id = section.get("sourceId")
        evidence_id = source_map.get(source_id) if isinstance(source_id, str) else None
        if evidence_id is not None and evidence_id not in output:
            output.append(evidence_id)
    return output


def _provider_failure(message: str) -> bool:
    lowered = message.lower()
    return bool(
        re.search(r"(?:request failed|status):\s*[45]\d\d", lowered)
        or any(
            token in lowered
            for token in ("401", "403", "429", "rate limit", "fetch failed")
        )
    )


def _sanitized_error(error: BaseException) -> str:
    message = str(error)
    for name in ("CHAT_API_KEY", "EMBEDDING_API_KEY"):
        secret = os.environ.get(name)
        if secret:
            message = message.replace(secret, "[redacted]")
    return message


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
        "target": "sag",
        "native_mode": "native_multi_search_source_id_trace",
        "score_eligible": True,
        "result_class": classification,
        "terminal_reason": reason,
        "warm_reused_state": False,
        "ingest_count": 0,
        "phases": {"cold": phase(), "warm": phase()},
    }


def _run_native(
    runner_path: Path,
    input_path: Path,
    output_path: Path,
    artifacts: Path,
    environment: dict[str, str],
) -> None:
    raw_dir = artifacts / "raw"
    raw_dir.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env.update(
        {
            "DATABASE_URL": environment["DATABASE_URL"],
            "DEFAULT_TENANT_ID": "elf-benchmark",
            "EMBEDDING_BASE_URL": environment["EMBEDDING_API_BASE"],
            "EMBEDDING_API_KEY": environment["EMBEDDING_API_KEY"],
            "EMBEDDING_MODEL": environment["EMBEDDING_MODEL"],
            "EMBEDDING_DIMENSIONS": environment["EMBEDDING_DIMENSIONS"],
            "LLM_BASE_URL": environment["CHAT_API_BASE"],
            "LLM_API_KEY": environment["CHAT_API_KEY"],
            "LLM_MODEL": environment["CHAT_MODEL"],
            "LLM_REASONING_EFFORT": environment["CHAT_REASONING_EFFORT"],
            "LLM_TIMEOUT_MS": os.environ.get("SAG_LLM_TIMEOUT_MS", "120000"),
            "LLM_MAX_RETRIES": "1",
            "DEFAULT_SEARCH_MODE": "standard",
            "INGEST_CONCURRENCY": "1",
            "NO_COLOR": "1",
            "NODE_ENV": "production",
        }
    )
    started = time.monotonic()
    try:
        completed = subprocess.run(
            [str(SAG_TSX), str(runner_path), str(input_path), str(output_path)],
            cwd=SAG_REPO,
            env=env,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=int(os.environ.get("SAG_TIMEOUT_SECONDS", "900")),
        )
    except subprocess.TimeoutExpired as error:
        stdout = error.stdout or ""
        stderr = error.stderr or ""
        if isinstance(stdout, bytes):
            stdout = stdout.decode("utf-8", errors="replace")
        if isinstance(stderr, bytes):
            stderr = stderr.decode("utf-8", errors="replace")
        (raw_dir / "sag-stdout.log").write_text(stdout, encoding="utf-8")
        (raw_dir / "sag-stderr.log").write_text(stderr, encoding="utf-8")
        raise SagProductFailure("SAG native runner timed out") from error
    (raw_dir / "sag-stdout.log").write_text(completed.stdout, encoding="utf-8")
    (raw_dir / "sag-stderr.log").write_text(completed.stderr, encoding="utf-8")
    if completed.returncode:
        detail = (completed.stderr or completed.stdout).strip()
        raise SagProductFailure(
            f"SAG native runner exited {completed.returncode} after "
            f"{time.monotonic() - started:.3f}s: {detail}"
        )


def _phase(
    jobs: list[dict[str, Any]],
    rows: Any,
    source_map: dict[str, str],
    *,
    reused: bool,
) -> dict[str, Any]:
    if not isinstance(rows, list):
        raise SagAdapterFailure("SAG native search output is malformed")
    by_job = {
        row.get("jobId"): row for row in rows if isinstance(row, dict)
    }
    output = []
    for job in jobs:
        row = by_job.get(job["job_id"])
        if not isinstance(row, dict):
            raise SagAdapterFailure(f"SAG omitted native result for {job['job_id']}")
        search = row.get("search")
        sections = search.get("sections") if isinstance(search, dict) else None
        if not isinstance(sections, list):
            raise SagAdapterFailure(
                f"SAG returned malformed sections for {job['job_id']}"
            )
        output.append(
            {
                "job_id": job["job_id"],
                "classification": "completed",
                "evidence_ids": evidence_ids_from_search(search, source_map),
                "returned_count": len(sections),
                "latency_ms": round(float(row.get("latencyMs") or 0.0), 3),
                "native_status": "completed",
                "failure": None,
            }
        )
    return {
        "status": "completed",
        "jobs": output,
        "adapter_metadata": {
            "index_reused": reused,
            "revision": SAG_REVISION,
            "embedding_dimensions": 1024,
            "embedding_dimension_deviation": "native_sag_schema_limit",
        },
    }


def run_sag(input_dir: Path, artifacts: Path, state_dir: Path) -> dict[str, Any]:
    """Run one SAG ingest and cold/warm searches against the exact same state."""
    jobs: list[dict[str, Any]] = []
    try:
        _verify_revision()
        jobs = _load_jobs(input_dir)
        environment = _required_environment()
        state_dir.mkdir(parents=True, exist_ok=True)
        receipt_path = state_dir / "cold-ingest.json"
        if receipt_path.exists():
            raise SagAdapterFailure("SAG cold ingest receipt already exists")
        native_input, source_map = _native_input(jobs)
        runner_path = state_dir / "sag-native-runner.ts"
        input_path = state_dir / "sag-native-input.json"
        output_path = artifacts / "raw" / "sag-native-output.json"
        runner_path.write_text(_NATIVE_RUNNER, encoding="utf-8")
        _write_json(input_path, native_input)
        _run_native(
            runner_path, input_path, output_path, artifacts, environment
        )
        native = json.loads(output_path.read_text(encoding="utf-8"))
        state_digests = [
            native.get("stateBeforeCold"),
            native.get("stateAfterCold"),
            native.get("stateAfterWarm"),
        ]
        if not all(isinstance(item, dict) for item in state_digests):
            raise SagAdapterFailure("SAG omitted native state evidence")
        hashes = [item.get("sha256") for item in state_digests]
        if not hashes[0] or len(set(hashes)) != 1:
            raise SagAdapterFailure(
                "SAG cold and warm queries did not reuse exact state"
            )
        ingest = native.get("ingest")
        expected_ingests = sum(len(job["corpus"]["items"]) for job in jobs)
        if not isinstance(ingest, list) or len(ingest) != expected_ingests:
            raise SagAdapterFailure(
                "SAG did not return one native ingest per corpus item"
            )
        ingest_latency_ms = native.get("ingestLatencyMs")
        if not isinstance(ingest_latency_ms, (int, float)) or ingest_latency_ms < 0:
            raise SagAdapterFailure("SAG omitted native ingest latency")
        receipt = {
            "revision": SAG_REVISION,
            "fixture_sha256": hashlib.sha256(
                json.dumps(
                    jobs, ensure_ascii=True, separators=(",", ":"), sort_keys=True
                ).encode("utf-8")
            ).hexdigest(),
            "source_identity": source_map,
            "native_state_sha256": hashes[0],
        }
        _write_json(receipt_path, receipt)
        if json.loads(receipt_path.read_text(encoding="utf-8")) != receipt:
            raise SagAdapterFailure("SAG cold ingest receipt readback changed")
        cold = _phase(jobs, native.get("cold"), source_map, reused=False)
        warm = _phase(jobs, native.get("warm"), source_map, reused=True)
    except SagConfigurationFailure as error:
        return _terminal_result(
            jobs, "configuration_failed", _sanitized_error(error)
        )
    except SagAdapterFailure as error:
        return _terminal_result(jobs, "adapter_failed", _sanitized_error(error))
    except SagProductFailure as error:
        reason = _sanitized_error(error)
        classification = (
            "provider_failed" if _provider_failure(reason) else "product_failed"
        )
        return _terminal_result(jobs, classification, reason)
    except (
        KeyError,
        OSError,
        TypeError,
        ValueError,
        subprocess.SubprocessError,
        json.JSONDecodeError,
    ) as error:
        return _terminal_result(
            jobs, "adapter_failed", _sanitized_error(error)
        )

    return {
        "schema": "elf.benchmark_unit_result/v4",
        "target": "sag",
        "native_mode": "native_multi_search_source_id_trace",
        "score_eligible": True,
        "result_class": "completed",
        "warm_reused_state": True,
        "ingest_count": 1,
        "ingest_duration_ms": round(float(ingest_latency_ms), 3),
        "phases": {"cold": cold, "warm": warm},
    }
