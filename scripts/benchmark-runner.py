#!/usr/bin/env python3
"""One host scheduler for the complete isolated competitor benchmark."""

from __future__ import annotations

import argparse
import concurrent.futures
import copy
import hashlib
import json
import os
import random
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any

from benchmark_contract import (
    NativeContextError,
    answer_cases,
    evaluate_unit,
    load_json,
    materialize_product_fixtures,
    sha256_json,
    validate_manifest,
    validate_suite,
)


REPO = Path(__file__).resolve().parent.parent
DEFAULT_MANIFEST = REPO / "config/benchmark/benchmark-v3.json"
TARGET_IMAGE_ENV = {
    target: f"BENCHMARK_{target.upper()}_IMAGE"
    for target in (
        "pageindex",
        "openviking",
        "graphiti",
        "graphrag",
        "letta",
        "sag",
        "openkb",
        "honcho",
    )
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--suite", action="append", dest="suites")
    parser.add_argument("--only-target")
    parser.add_argument("--job-limit", type=int)
    parser.add_argument("--artifact-root", type=Path)
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--allow-dirty", action="store_true")
    return parser.parse_args()


def command(
    args: list[str],
    *,
    env: dict[str, str] | None = None,
    check: bool = True,
    timeout: int | None = None,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=REPO,
        env=env,
        check=check,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=timeout,
    )


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def load_local_env() -> dict[str, str]:
    values = dict(os.environ)
    common_dir = Path(command(["git", "rev-parse", "--git-common-dir"]).stdout.strip())
    if not common_dir.is_absolute():
        common_dir = (REPO / common_dir).resolve()
    for path in (common_dir.parent / ".env", REPO / ".env"):
        if not path.is_file():
            continue
        for line in path.read_text(encoding="utf-8").splitlines():
            stripped = line.strip()
            if not stripped or stripped.startswith("#") or "=" not in stripped:
                continue
            name, value = stripped.split("=", 1)
            values.setdefault(name.strip(), value.strip().strip('"').strip("'"))
    return values


def container_url(value: str) -> str:
    parsed = urllib.parse.urlsplit(value.rstrip("/"))
    if parsed.hostname not in {"127.0.0.1", "localhost", "::1"}:
        return value.rstrip("/")
    host = "host.docker.internal"
    if parsed.port:
        host = f"{host}:{parsed.port}"
    return urllib.parse.urlunsplit(
        (parsed.scheme, host, parsed.path.rstrip("/"), parsed.query, parsed.fragment)
    )


def provider_environment(
    source: dict[str, str], providers: dict[str, Any], *, inside_container: bool
) -> dict[str, str]:
    embedding_base = source.get("EMBEDDING_API_BASE")
    embedding_key = source.get("EMBEDDING_API_KEY")
    chat_base = (
        source.get("LITELLM_CHAT_COMPAT_BASE_URL")
        or source.get("LITELLM_BASE_URL")
    )
    chat_key = source.get("LITELLM_API_KEY")
    missing = [
        name
        for name, value in (
            ("EMBEDDING_API_BASE", embedding_base),
            ("EMBEDDING_API_KEY", embedding_key),
            ("LITELLM_CHAT_COMPAT_BASE_URL or LITELLM_BASE_URL", chat_base),
            ("LITELLM_API_KEY", chat_key),
        )
        if not value
    ]
    if missing:
        raise KeyError("missing benchmark provider configuration: " + ", ".join(missing))
    transform = container_url if inside_container else lambda value: value.rstrip("/")
    return {
        "BENCHMARK_EMBEDDING_API_BASE": transform(str(embedding_base)),
        "BENCHMARK_EMBEDDING_API_KEY": str(embedding_key),
        "BENCHMARK_EMBEDDING_MODEL": providers["embedding"]["model"],
        "BENCHMARK_EMBEDDING_DIMENSIONS": str(providers["embedding"]["dimensions"]),
        "BENCHMARK_CHAT_API_BASE": transform(str(chat_base)),
        "BENCHMARK_CHAT_API_KEY": str(chat_key),
        "BENCHMARK_CHAT_MODEL": providers["chat"]["model"],
        "BENCHMARK_CHAT_REASONING_EFFORT": providers["chat"]["reasoning_effort"],
    }


def provider_preflight(env: dict[str, str], artifact_root: Path) -> dict[str, Any]:
    completed = command(
        [sys.executable, "scripts/benchmark-provider-preflight.py"],
        env=env,
        check=False,
        timeout=180,
    )
    try:
        result = json.loads(completed.stdout.strip().splitlines()[-1])
    except (IndexError, json.JSONDecodeError):
        result = {
            "schema": "elf.benchmark_provider_preflight/v1",
            "classification": "harness_failed",
            "message": "provider preflight did not return typed JSON",
        }
    result["exit_code"] = completed.returncode
    write_json(artifact_root / "provider-preflight.json", result)
    return result


def source_fingerprint() -> dict[str, Any]:
    head = command(["git", "rev-parse", "HEAD"]).stdout.strip()
    status = command(
        ["git", "status", "--porcelain=v1", "--untracked-files=all"]
    ).stdout
    digest = hashlib.sha256()
    digest.update(command(["git", "diff", "--binary", "HEAD"]).stdout.encode())
    for relative in command(
        ["git", "ls-files", "--others", "--exclude-standard"]
    ).stdout.splitlines():
        path = REPO / relative
        if path.is_file():
            digest.update(relative.encode() + b"\0" + path.read_bytes())
    return {
        "head": head,
        "dirty": bool(status),
        "status_sha256": hashlib.sha256(status.encode()).hexdigest(),
        "content_sha256": digest.hexdigest(),
    }


def image_id(image: str) -> str:
    completed = command(
        ["docker", "image", "inspect", "--format", "{{.Id}}", image],
        check=False,
    )
    value = completed.stdout.strip()
    if completed.returncode or not value.startswith("sha256:"):
        raise RuntimeError(f"Docker image is unavailable: {image}")
    return value


def build_image(image: str, dockerfile: str) -> str:
    source_commit = command(["git", "rev-parse", "HEAD"]).stdout.strip()
    completed = command(
        [
            "docker",
            "build",
            "--file",
            dockerfile,
            "--tag",
            image,
            "--build-arg",
            f"ELF_SOURCE_COMMIT={source_commit}",
            ".",
        ],
        check=False,
    )
    if completed.returncode:
        raise RuntimeError(completed.stdout[-8000:])
    return image_id(image)


def build_images(
    manifest: dict[str, Any], targets: list[dict[str, Any]], *, skip_build: bool
) -> tuple[dict[str, str], dict[str, str], dict[str, str]]:
    tags: dict[str, str] = {}
    digests: dict[str, str] = {}
    failures: dict[str, str] = {}
    main_targets = [target for target in targets if not target.get("image")]
    main_image = manifest["runner"]["image"]
    if main_targets:
        try:
            main_digest = (
                image_id(main_image)
                if skip_build
                else build_image(main_image, "docker/benchmark/Dockerfile")
            )
            for target in main_targets:
                tags[target["id"]] = main_image
                digests[target["id"]] = main_digest
        except Exception as error:
            for target in main_targets:
                failures[target["id"]] = f"benchmark unit image build failed: {error}"
    for target in targets:
        if not target.get("image"):
            continue
        target_id = target["id"]
        try:
            digest = (
                image_id(target["image"])
                if skip_build
                else build_image(target["image"], target["dockerfile"])
            )
            tags[target_id] = target["image"]
            digests[target_id] = digest
        except Exception as error:
            failures[target_id] = f"{target_id} image build failed: {error}"
    return tags, digests, failures


def cleanup_project(project: str, compose_file: Path, env: dict[str, str]) -> dict[str, Any]:
    try:
        down = command(
            [
                "docker",
                "compose",
                "--project-name",
                project,
                "--file",
                str(compose_file),
                "down",
                "--volumes",
                "--remove-orphans",
                "--timeout",
                "15",
            ],
            env=env,
            check=False,
            timeout=90,
        )
        down_exit_code = down.returncode
        down_output = down.stdout[-4000:]
    except subprocess.TimeoutExpired as error:
        down_exit_code = 124
        down_output = str(error)
    remaining: dict[str, list[str]] = {}
    inspection_errors: list[str] = []
    for kind, args in (
        ("containers", ["docker", "ps", "-aq", "--filter", f"label=com.docker.compose.project={project}"]),
        ("volumes", ["docker", "volume", "ls", "-q", "--filter", f"label=com.docker.compose.project={project}"]),
        ("networks", ["docker", "network", "ls", "-q", "--filter", f"label=com.docker.compose.project={project}"]),
    ):
        try:
            inspected = command(args, check=False)
            remaining[kind] = inspected.stdout.split()
            if inspected.returncode:
                inspection_errors.append(f"{kind} inspection exited {inspected.returncode}")
        except Exception as error:
            remaining[kind] = []
            inspection_errors.append(f"{kind} inspection failed: {type(error).__name__}")
    return {
        "down_exit_code": down_exit_code,
        "down_output": down_output,
        "remaining": remaining,
        "inspection_errors": inspection_errors,
        "passed": down_exit_code == 0
        and not inspection_errors
        and not any(remaining.values()),
    }


def project_images(project: str, compose_file: Path, env: dict[str, str]) -> list[dict[str, Any]]:
    try:
        completed = command(
            [
                "docker",
                "compose",
                "--project-name",
                project,
                "--file",
                str(compose_file),
                "images",
                "--format",
                "json",
            ],
            env=env,
            check=False,
        )
    except Exception as error:
        return [
            {
                "classification": "harness_failed",
                "message": f"image provenance failed: {type(error).__name__}",
            }
        ]
    if completed.returncode:
        return [{"classification": "harness_failed", "message": "image provenance unavailable"}]
    try:
        decoded = json.loads(completed.stdout or "[]")
    except json.JSONDecodeError:
        decoded = [json.loads(line) for line in completed.stdout.splitlines() if line.strip()]
    return decoded if isinstance(decoded, list) else [decoded]


def compose_project_name(run_id: str, suite_id: str, target_id: str) -> str:
    token = hashlib.sha256(f"{run_id}:{suite_id}:{target_id}".encode()).hexdigest()[:10]
    return f"elfb-{token}-{target_id}"[:63]


def failure_unit(
    target: dict[str, Any], suite: dict[str, Any], classification: str, message: str
) -> dict[str, Any]:
    jobs = [
        {
            "job_id": job["job_id"],
            "classification": classification,
            "evidence_ids": [],
            "returned_count": 0,
            "latency_ms": 0.0,
            "failure": message,
            "operations": [],
        }
        for job in suite["jobs"]
    ]
    return {
        "schema": "elf.benchmark_unit_result/v4",
        "target": target["id"],
        "native_mode": target["adapter"],
        "score_eligible": bool(target["score_eligible"]),
        "result_class": classification,
        "failure": {"classification": classification, "message": message},
        "warm_reused_state": False,
        "ingest_count": 0,
        "phases": {
            "cold": {"status": classification, "jobs": copy.deepcopy(jobs)},
            "warm": {"status": classification, "jobs": copy.deepcopy(jobs)},
        },
    }


def _api_endpoint(base: str, resource: str) -> str:
    parsed = urllib.parse.urlsplit(base.rstrip("/"))
    path = parsed.path.rstrip("/")
    if not path.endswith(f"/{resource}"):
        path = f"{path}/{resource}" if path.endswith("/v1") else f"{path}/v1/{resource}"
    return urllib.parse.urlunsplit((parsed.scheme, parsed.netloc, path, parsed.query, parsed.fragment))


def attach_shared_answers(
    suite: dict[str, Any], unit: dict[str, Any], env: dict[str, str], context_budget: int
) -> dict[str, Any]:
    if unit.get("result_class") != "completed" or not unit.get("score_eligible"):
        return unit
    try:
        cases = answer_cases(suite, unit, context_budget)
    except NativeContextError as error:
        return failure_unit(
            {
                "id": unit["target"],
                "adapter": unit.get("native_mode"),
                "score_eligible": unit.get("score_eligible"),
            },
            suite,
            "adapter_failed",
            str(error),
        )
    if not cases:
        return unit
    prompt = {
        "instruction": (
            "Answer each case only from its supplied context. Return one JSON object "
            "with key answers. Each answer must contain the unchanged case_id, text, and "
            "supported. The text value must never be empty. If supported=true, copy the "
            "concise requested fact from context into text. If the context does not support "
            "the fact, set supported=false and text exactly to unknown."
        ),
        "cases": cases,
    }
    request = urllib.request.Request(
        _api_endpoint(env["BENCHMARK_CHAT_API_BASE"], "chat/completions"),
        data=json.dumps(
            {
                "model": env["BENCHMARK_CHAT_MODEL"],
                "messages": [{"role": "user", "content": json.dumps(prompt, ensure_ascii=False)}],
                "reasoning_effort": env["BENCHMARK_CHAT_REASONING_EFFORT"],
                "response_format": {"type": "json_object"},
                "stream": False,
                "max_tokens": 4096,
            }
        ).encode(),
        headers={
            "Authorization": f"Bearer {env['BENCHMARK_CHAT_API_KEY']}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    native: dict[str, Any] | None = None
    try:
        with urllib.request.urlopen(request, timeout=300) as response:
            native = json.loads(response.read().decode())
        unit["provider_raw"] = {"shared_answer": native}
        content = native["choices"][0]["message"]["content"]
        decoded = json.loads(content)
        answers = decoded["answers"]
        if not isinstance(answers, list):
            raise TypeError("answers is not a list")
        by_id = {
            row.get("case_id"): row
            for row in answers
            if isinstance(row, dict) and isinstance(row.get("case_id"), str)
        }
        for row in unit["phases"]["warm"]["jobs"]:
            if row.get("classification") == "completed":
                answer = by_id.get(row.get("job_id"))
                if answer is None:
                    raise ValueError(f"shared answer omitted case {row.get('job_id')}")
                text = answer.get("text")
                supported = answer.get("supported")
                if not isinstance(text, str) or not text.strip():
                    raise ValueError(
                        f"shared answer returned empty text for {row.get('job_id')}"
                    )
                if not isinstance(supported, bool):
                    raise TypeError(
                        f"shared answer returned non-boolean supported for {row.get('job_id')}"
                    )
                if not supported and text.strip().casefold() != "unknown":
                    raise ValueError(
                        f"unsupported shared answer did not say unknown for {row.get('job_id')}"
                    )
                row["answer"] = {
                    "text": text,
                    "supported": supported,
                }
        unit["provider_usage"] = {"shared_answer": native.get("usage") or {}}
        return unit
    except Exception as error:
        message = f"shared target-blind answer request failed: {type(error).__name__}: {error}"
        failed = failure_unit(
            {"id": unit["target"], "adapter": unit.get("native_mode"), "score_eligible": unit.get("score_eligible")},
            suite,
            "provider_failed",
            message,
        )
        if native is not None:
            failed["provider_raw"] = {"shared_answer": native}
        return failed


def run_unit(
    *,
    target: dict[str, Any],
    suite: dict[str, Any],
    run_id: str,
    artifact_root: Path,
    compose_file: Path,
    container_env: dict[str, str],
    host_provider_env: dict[str, str],
    image: str | None,
    build_failure: str | None,
    timeout_seconds: int,
    context_budget: int,
) -> dict[str, Any]:
    target_id = target["id"]
    unit_root = artifact_root / "units" / suite["suite_id"] / target_id
    input_root = unit_root / "product-input"
    artifact_dir = unit_root / "artifacts"
    materialize_product_fixtures(suite, input_root)
    input_hash = hashlib.sha256(
        b"".join(path.read_bytes() for path in sorted(input_root.glob("*.json")))
    ).hexdigest()
    if build_failure or image is None:
        raw_unit = failure_unit(
            target, suite, "configuration_failed", build_failure or "image unavailable"
        )
        write_json(artifact_dir / "unit-result.json", raw_unit)
        evaluation = evaluate_unit(suite, raw_unit, target)
        return {
            "target": target_id,
            "project": None,
            "compose_exit_code": None,
            "duration_seconds": 0.0,
            "input_sha256": input_hash,
            "unit_result": raw_unit,
            "evaluation": evaluation,
            "deterministic_replay": {"passed": True, "sha256": sha256_json(evaluation)},
            "cleanup": {"passed": True, "not_started": True},
            "runtime_images": [],
        }

    project = compose_project_name(run_id, suite["suite_id"], target_id)
    env = dict(os.environ)
    env.update(container_env)
    env.update(
        {
            "BENCHMARK_IMAGE": image,
            "BENCHMARK_INPUT_HOST": str(input_root.resolve()),
            "BENCHMARK_ARTIFACT_HOST": str(artifact_dir.resolve()),
            "BENCHMARK_POSTGRES_PASSWORD": hashlib.sha256(project.encode()).hexdigest()[:24],
        }
    )
    image_env = TARGET_IMAGE_ENV.get(target_id)
    if image_env:
        env[image_env] = image
    started = time.monotonic()
    timed_out = False
    harness_error: str | None = None
    try:
        completed = command(
            [
                "docker",
                "compose",
                "--project-name",
                project,
                "--file",
                str(compose_file),
                "run",
                "--rm",
                "--no-TTY",
                f"{target_id}-unit",
            ],
            env=env,
            check=False,
            timeout=timeout_seconds,
        )
        compose_output = completed.stdout
        compose_exit = completed.returncode
    except subprocess.TimeoutExpired as error:
        timed_out = True
        timed_output = error.stdout or ""
        if isinstance(timed_output, bytes):
            timed_output = timed_output.decode(errors="replace")
        compose_output = timed_output + "\nunit timed out\n"
        compose_exit = 124
    except Exception as error:
        harness_error = f"Compose unit invocation failed: {type(error).__name__}: {error}"
        compose_output = harness_error + "\n"
        compose_exit = 125
    unit_root.mkdir(parents=True, exist_ok=True)
    (unit_root / "compose.log").write_text(compose_output, encoding="utf-8")
    runtime_images = project_images(project, compose_file, env)
    cleanup = cleanup_project(project, compose_file, env)
    result_path = artifact_dir / "unit-result.json"
    if timed_out:
        raw_unit = failure_unit(target, suite, "timeout_failed", "isolated unit timed out")
    elif harness_error:
        raw_unit = failure_unit(target, suite, "harness_failed", harness_error)
    elif not result_path.is_file():
        raw_unit = failure_unit(
            target, suite, "harness_failed", "unit did not write unit-result.json"
        )
    else:
        try:
            raw_unit = load_json(result_path)
        except Exception as error:
            raw_unit = failure_unit(
                target,
                suite,
                "adapter_failed",
                f"unit result is not valid JSON: {type(error).__name__}: {error}",
            )
    if not cleanup["passed"]:
        raw_unit = failure_unit(
            target, suite, "cleanup_failed", "Compose project cleanup was incomplete"
        )
    raw_unit = attach_shared_answers(
        suite, raw_unit, host_provider_env, context_budget
    )
    write_json(unit_root / "raw-unit-result.json", raw_unit)
    try:
        evaluation = evaluate_unit(suite, raw_unit, target)
    except Exception as error:
        raw_unit = failure_unit(
            target,
            suite,
            "adapter_failed",
            f"unit result violates the normalized contract: {type(error).__name__}: {error}",
        )
        evaluation = evaluate_unit(suite, raw_unit, target)
    replay = evaluate_unit(suite, copy.deepcopy(raw_unit), target)
    write_json(unit_root / "normalized-result.json", evaluation)
    return {
        "target": target_id,
        "project": project,
        "compose_exit_code": compose_exit,
        "duration_seconds": round(time.monotonic() - started, 3),
        "input_sha256": input_hash,
        "unit_result": raw_unit,
        "evaluation": evaluation,
        "deterministic_replay": {
            "passed": sha256_json(evaluation) == sha256_json(replay),
            "sha256": sha256_json(evaluation),
        },
        "cleanup": cleanup,
        "runtime_images": runtime_images,
    }


def run_matrix(
    *,
    targets: list[dict[str, Any]],
    capacity: int,
    suite: dict[str, Any],
    run_id: str,
    artifact_root: Path,
    compose_file: Path,
    container_env: dict[str, str],
    host_provider_env: dict[str, str],
    images: dict[str, str],
    build_failures: dict[str, str],
    timeout_seconds: int,
    context_budget: int,
) -> list[dict[str, Any]]:
    shuffled = list(targets)
    random.Random(f"{run_id}:{suite['suite_id']}").shuffle(shuffled)
    kwargs = [
        {
            "target": target,
            "suite": suite,
            "run_id": run_id,
            "artifact_root": artifact_root,
            "compose_file": compose_file,
            "container_env": container_env,
            "host_provider_env": host_provider_env,
            "image": images.get(target["id"]),
            "build_failure": build_failures.get(target["id"]),
            "timeout_seconds": timeout_seconds,
            "context_budget": context_budget,
        }
        for target in shuffled
    ]
    if capacity == 1:
        return [run_unit(**item) for item in kwargs]
    with concurrent.futures.ThreadPoolExecutor(max_workers=capacity) as pool:
        futures = [pool.submit(run_unit, **item) for item in kwargs]
        return [future.result() for future in futures]


def acceptance(bundle: dict[str, Any], full_run: bool) -> dict[str, Any]:
    if not full_run:
        return {"passed": True, "mode": "readiness_or_regression", "findings": []}
    findings: list[str] = []
    suites = bundle["suite_results"]
    common = suites["common-core-v1"]["results"]
    common_completed = [
        row["target"]
        for row in common
        if row["evaluation"]["classification"] == "completed"
        and row["evaluation"]["phases"]["warm"]["quality_denominator"]
    ]
    if "elf" not in common_completed:
        findings.append("ELF did not complete common-core")
    if len([target for target in common_completed if target != "elf"]) < 4:
        findings.append("fewer than four non-ELF common-core products completed")
    for suite_id in (
        "memory-lifecycle-v1",
        "knowledge-structure-v1",
        "repository-knowledge-v1",
    ):
        completed = [
            row["target"]
            for row in suites[suite_id]["results"]
            if row["evaluation"]["classification"] == "completed"
        ]
        if len(completed) < 2:
            findings.append(f"{suite_id} has fewer than two completed products")
    for suite in suites.values():
        for row in suite["results"]:
            if not row["cleanup"]["passed"]:
                findings.append(f"{row['target']} cleanup failed")
            if not row["deterministic_replay"]["passed"]:
                findings.append(f"{row['target']} deterministic replay failed")
    return {"passed": not findings, "mode": "complete_measured_run", "findings": findings}


def main() -> int:
    args = parse_args()
    manifest = load_json(args.manifest)
    validate_manifest(manifest)
    selected_suite_ids = args.suites or [entry["id"] for entry in manifest["suites"]]
    unknown = set(selected_suite_ids) - {entry["id"] for entry in manifest["suites"]}
    if unknown:
        raise ValueError(f"unknown suites: {sorted(unknown)}")
    full_run = not args.suites and not args.only_target and not args.job_limit
    now = time.strftime("%Y%m%dT%H%M%SZ", time.gmtime())
    run_id = hashlib.sha256(f"{now}-{os.getpid()}".encode()).hexdigest()[:10]
    artifact_root = args.artifact_root or REPO / "tmp/benchmark-v4" / f"{now}-{run_id}"
    artifact_root.mkdir(parents=True, exist_ok=False)
    source = source_fingerprint()
    if full_run and source["dirty"] and not args.allow_dirty:
        result = {
            "schema": "elf.benchmark_bundle/v1",
            "classification": "configuration_failed",
            "message": "complete measured run requires a clean fixed source commit",
            "source": source,
        }
        write_json(artifact_root / "bundle.json", result)
        print(artifact_root / "bundle.json")
        return 2
    try:
        local_env = load_local_env()
        host_provider_env = dict(os.environ)
        host_provider_env.update(
            provider_environment(local_env, manifest["providers"], inside_container=False)
        )
        container_provider_env = provider_environment(
            local_env, manifest["providers"], inside_container=True
        )
    except KeyError as error:
        result = {
            "schema": "elf.benchmark_provider_preflight/v1",
            "classification": "configuration_failed",
            "message": str(error),
        }
        write_json(artifact_root / "provider-preflight.json", result)
        print(artifact_root / "provider-preflight.json")
        return 2
    preflight = provider_preflight(host_provider_env, artifact_root)
    if preflight.get("classification") != "completed":
        bundle = {
            "schema": "elf.benchmark_bundle/v1",
            "classification": preflight.get("classification"),
            "source": source,
            "provider_preflight": preflight,
            "suite_results": {},
        }
        write_json(artifact_root / "bundle.json", bundle)
        print(artifact_root / "bundle.json")
        return 2

    entries = {
        entry["id"]: entry for entry in manifest["suites"] if entry["id"] in selected_suite_ids
    }
    suites: dict[str, dict[str, Any]] = {}
    for suite_id, entry in entries.items():
        suite = load_json(REPO / entry["path"])
        validate_suite(suite)
        if args.job_limit:
            if args.job_limit < 1 or args.job_limit > len(suite["jobs"]):
                raise ValueError("job limit is outside the suite")
            suite = copy.deepcopy(suite)
            suite["jobs"] = suite["jobs"][: args.job_limit]
            suite["execution_mode"] = "readiness"
        suites[suite_id] = suite
    selected_targets = [
        target
        for target in manifest["targets"]
        if any(suite_id in target["suites"] for suite_id in selected_suite_ids)
        and (args.only_target is None or target["id"] == args.only_target)
    ]
    if args.only_target and not selected_targets:
        raise ValueError(f"target {args.only_target} is not eligible for selected suites")
    images, image_digests, build_failures = build_images(
        manifest, selected_targets, skip_build=args.skip_build
    )
    docker_version = command(["docker", "version", "--format", "{{.Server.Version}}"]).stdout.strip()
    compose_file = REPO / manifest["runner"]["compose_file"]
    suite_results: dict[str, Any] = {}
    for suite_id, suite in suites.items():
        targets = [
            target
            for target in selected_targets
            if suite_id in target["suites"]
        ]
        rows = run_matrix(
            targets=targets,
            capacity=int(manifest["runner"]["capacity"]),
            suite=suite,
            run_id=run_id,
            artifact_root=artifact_root,
            compose_file=compose_file,
            container_env=container_provider_env,
            host_provider_env=host_provider_env,
            images=images,
            build_failures=build_failures,
            timeout_seconds=int(manifest["runner"]["unit_timeout_seconds"]),
            context_budget=int(manifest["runner"]["context_budget_chars"]),
        )
        suite_results[suite_id] = {
            "suite_sha256": sha256_json(suite),
            "scheduled_targets": [target["id"] for target in targets],
            "results": sorted(rows, key=lambda row: row["target"]),
        }
        write_json(artifact_root / "suites" / suite_id / "summary.json", suite_results[suite_id])
    bundle = {
        "schema": "elf.benchmark_bundle/v1",
        "classification": "completed",
        "mode": "complete_measured_run" if full_run else "readiness_or_regression",
        "run_id": run_id,
        "created_at": now,
        "source": source,
        "manifest_sha256": hashlib.sha256(args.manifest.read_bytes()).hexdigest(),
        "provider_routes": {
            "chat_model": manifest["providers"]["chat"]["model"],
            "chat_reasoning_effort": manifest["providers"]["chat"]["reasoning_effort"],
            "embedding_model": manifest["providers"]["embedding"]["model"],
            "embedding_dimensions": manifest["providers"]["embedding"]["dimensions"],
        },
        "provider_preflight": preflight,
        "docker_server_version": docker_version,
        "target_image_tags": images,
        "target_image_digests": image_digests,
        "target_pins": {target["id"]: target["pin"] for target in manifest["targets"]},
        "target_contracts": {
            target["id"]: {
                key: target[key]
                for key in (
                    "adapter",
                    "score_eligible",
                    "suites",
                    "native_deviations",
                    "not_applicable",
                )
                if key in target
            }
            for target in manifest["targets"]
        },
        "build_failures": build_failures,
        "suite_results": suite_results,
    }
    bundle["acceptance"] = acceptance(bundle, full_run)
    bundle_path = artifact_root / "bundle.json"
    write_json(bundle_path, bundle)
    if full_run:
        report_path = artifact_root / "report.zh-CN.md"
        report = command(
            [
                sys.executable,
                "scripts/benchmark-report.py",
                "--bundle",
                str(bundle_path),
                "--out",
                str(report_path),
            ],
            check=False,
        )
        (artifact_root / "report.log").write_text(report.stdout, encoding="utf-8")
        if report.returncode:
            print(bundle_path)
            return 1
    print(bundle_path)
    return 0 if bundle["acceptance"]["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
