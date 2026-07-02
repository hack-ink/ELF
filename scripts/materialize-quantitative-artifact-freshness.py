#!/usr/bin/env python3
"""Materialize provenance gates for a Docker-contained quantitative aggregate."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


SCHEMA = "elf.quantitative_artifact_freshness_manifest/v1"
PRODUCT_MANIFEST_SCHEMA = "elf.agent_memory_quantitative_product_manifest/v1"
BENCHMARK_COMMAND = "cargo make real-world-memory-quantitative-docker"
BENCHMARK_RUNNER = "docker-compose.baseline.yml baseline-runner"
BENCHMARK_COMPOSE_FILE = "docker-compose.baseline.yml"
REPO_ROOT = Path(__file__).resolve().parents[1]

REPRODUCIBILITY_REQUIRED_FIELDS = [
    "command",
    "runner",
    "compose_file",
    "repository_head",
    "env_profile",
    "input_manifest_sha256",
    "artifact_digests",
    "container_image_digest",
    "product_commit",
]

PRODUCT_COMMIT_FIELDS = [
    "product_commit",
    "product_revision",
    "source_revision",
    "repository_commit",
    "git_commit",
    "commit",
    "revision",
]

RUNTIME_SOURCE_ATTESTATION_REQUIREMENTS = {
    "honcho_live_real_world": (
        "Honcho product_commit may satisfy reproducibility only when the artifact "
        "attests that the pinned Honcho checkout was the runtime that emitted the row."
    ),
    "letta_research_gate": (
        "Letta product_commit may satisfy reproducibility only when the artifact attests "
        "that the pinned Letta server/runtime checkout was the runtime that emitted the row."
    ),
    "ragflow_docker_evidence_smoke": (
        "RAGFlow product_commit may satisfy reproducibility only when the artifact attests "
        "that the pinned RAGFlow checkout or image revision emitted retrieval outputs."
    ),
    "ragflow_research_gate": (
        "RAGFlow product_commit may satisfy reproducibility only when the artifact attests "
        "that the pinned RAGFlow checkout or image revision emitted retrieval outputs."
    ),
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sync-log", required=True, type=Path)
    parser.add_argument("--combined-product-manifest", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--operational-evidence-manifest", type=Path)
    parser.add_argument("--run-live-explicit-qrels", required=True)
    parser.add_argument("--run-langgraph", required=True)
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(value, handle, indent=2, sort_keys=True)
        handle.write("\n")


def resolve_artifact_path(path: str | Path) -> Path:
    raw = Path(path)
    if raw.is_absolute():
        return raw
    return REPO_ROOT / raw


def sync_log_rows(path: Path) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    with path.open(encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            fields = line.rstrip("\n").split("\t")
            if len(fields) < 4:
                raise SystemExit(f"{path}:{line_number} has {len(fields)} fields; expected >=4")
            rows.append(
                {
                    "action": fields[0],
                    "label": fields[1],
                    "path": fields[2],
                    "source_kind": fields[3] or "unknown",
                }
            )
    return rows


def combined_inputs(rows: list[dict[str, str]]) -> list[dict[str, str]]:
    return [row for row in rows if row["action"] == "combined-input"]


def sha256_file(path: Path) -> str | None:
    if not path.is_file():
        return None
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def valid_git_commit(value: Any) -> str | None:
    if not isinstance(value, str):
        return None
    commit = value.strip()
    if len(commit) == 40 and all(char in "0123456789abcdefABCDEF" for char in commit):
        return commit.lower()
    return None


def git_head() -> str | None:
    env_head = valid_git_commit(os.environ.get("ELF_REAL_WORLD_QUANTITATIVE_REPOSITORY_HEAD"))
    if env_head is not None:
        return env_head
    try:
        output = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=REPO_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return valid_git_commit(output.stdout.strip())


def env_profile(args: argparse.Namespace) -> dict[str, str]:
    return {
        "run_live_explicit_qrels": args.run_live_explicit_qrels,
        "run_langgraph": args.run_langgraph,
        "audit_profile": os.environ.get("ELF_REAL_WORLD_LIVE_AUDIT_PROFILE", "explicit-qrels-locked"),
        "checked_in_sync_enabled": os.environ.get("ELF_REAL_WORLD_QUANTITATIVE_SYNC_CHECKED_IN", "1"),
    }


def valid_sha256_digest(value: str | None) -> str | None:
    if not value:
        return None
    digest = value.strip().removeprefix("sha256:")
    if len(digest) == 64 and all(char in "0123456789abcdefABCDEF" for char in digest):
        return f"sha256:{digest.lower()}"
    return None


def runner_image_digest() -> tuple[str | None, str | None]:
    for env_name in (
        "ELF_BASELINE_RUNNER_IMAGE_DIGEST",
        "ELF_REAL_WORLD_QUANTITATIVE_RUNNER_IMAGE_DIGEST",
    ):
        digest = valid_sha256_digest(os.environ.get(env_name))
        if digest is not None:
            return digest, f"env:{env_name}"
    return None, None


def first_present(mapping: dict[str, Any], keys: list[str]) -> Any:
    for key in keys:
        value = mapping.get(key)
        if value not in (None, "", []):
            return value
    return None


def structured_product_commit(
    mapping: dict[str, Any] | None,
    *,
    scope: str,
) -> tuple[str | None, str | None, list[dict[str, str]]]:
    if mapping is None:
        return None, None, []

    rejected = []
    for field in PRODUCT_COMMIT_FIELDS:
        value = mapping.get(field)
        if value in (None, "", []):
            continue
        commit = valid_git_commit(value)
        if commit is not None:
            source = first_present(
                mapping,
                [
                    f"{field}_source",
                    "product_commit_source",
                    "product_revision_source",
                    "source_revision_source",
                    "repository_commit_source",
                    "git_commit_source",
                    "commit_source",
                    "revision_source",
                ],
            )
            if not isinstance(source, str) or not source.strip():
                source = f"{scope}.{field}"
            return commit, source.strip(), rejected
        rejected.append(
            {
                "scope": scope,
                "field": field,
                "value": str(value)[:200],
                "reason": "not_40_hex_git_commit",
            }
        )
    return None, None, rejected


def runtime_source_attestation(
    row: dict[str, Any] | None,
    manifest: dict[str, Any] | None,
) -> tuple[dict[str, Any] | None, str | None]:
    for scope, mapping in (("row", row), ("manifest", manifest)):
        if isinstance(mapping, dict) and isinstance(mapping.get("runtime_source_attestation"), dict):
            return mapping["runtime_source_attestation"], f"{scope}.runtime_source_attestation"
    return None, None


def runtime_source_attestation_status(
    *,
    adapter_id: str | None,
    product_commit: str | None,
    row: dict[str, Any] | None,
    manifest: dict[str, Any] | None,
) -> dict[str, Any] | None:
    claim_boundary = RUNTIME_SOURCE_ATTESTATION_REQUIREMENTS.get(adapter_id or "")
    if claim_boundary is None:
        return None

    status: dict[str, Any] = {
        "required": True,
        "status": "missing",
        "required_fields": [
            "runtime_executed=true",
            "source_checkout_used=true or image_revision_label_verified=true",
            "commit matches product_commit",
            "runtime_artifact",
        ],
        "claim_boundary": claim_boundary,
    }
    attestation, source = runtime_source_attestation(row, manifest)
    if attestation is None:
        status["reason"] = "missing_runtime_source_attestation"
        return status

    status["source"] = source
    status["status"] = "fail"
    if attestation.get("status") != "pass":
        reason = attestation.get("reason")
        status["reason"] = (
            reason if isinstance(reason, str) and reason.strip() else "runtime_source_attestation_not_pass"
        )
        return status

    attested_commit = valid_git_commit(
        attestation.get("product_commit")
        or attestation.get("commit")
        or attestation.get("repository_commit")
        or attestation.get("source_revision")
    )
    if attested_commit is None:
        status["reason"] = "missing_attested_40_hex_commit"
        return status
    if product_commit is None or attested_commit != product_commit:
        status["reason"] = "attested_commit_does_not_match_product_commit"
        status["attested_commit"] = attested_commit
        return status
    if attestation.get("runtime_executed") is not True:
        status["reason"] = "runtime_executed_not_true"
        return status
    if (
        attestation.get("source_checkout_used") is not True
        and attestation.get("image_revision_label_verified") is not True
    ):
        status["reason"] = "runtime_source_binding_not_verified"
        return status
    if not isinstance(attestation.get("runtime_artifact"), str) or not attestation["runtime_artifact"].strip():
        status["reason"] = "missing_runtime_artifact"
        return status

    status["status"] = "pass"
    status["reason"] = "runtime_source_attestation_passed"
    status["attested_commit"] = attested_commit
    return status


def missing_reproducibility_fields(record: dict[str, Any]) -> list[str]:
    return [
        field
        for field in REPRODUCIBILITY_REQUIRED_FIELDS
        if record.get(field) in (None, "", [])
    ]


def build_reproducibility_record(
    *,
    args: argparse.Namespace,
    repository_head: str | None,
    entry: dict[str, str],
    manifest: dict[str, Any] | None,
    row: dict[str, Any] | None,
    input_manifest_sha256: str | None,
) -> dict[str, Any]:
    row_commit, row_commit_source, rejected = structured_product_commit(row, scope="row")
    manifest_commit, manifest_commit_source, manifest_rejected = structured_product_commit(
        manifest,
        scope="manifest",
    )
    rejected.extend(manifest_rejected)

    product_commit = row_commit or manifest_commit
    product_commit_source = row_commit_source or manifest_commit_source
    adapter_id = str((row or {}).get("adapter_id") or (manifest or {}).get("adapter_id") or "")
    attestation_status = runtime_source_attestation_status(
        adapter_id=adapter_id,
        product_commit=product_commit,
        row=row,
        manifest=manifest,
    )
    if product_commit is not None and attestation_status is not None and attestation_status["status"] != "pass":
        rejected.append(
            {
                "scope": "runtime_source_attestation",
                "field": "product_commit",
                "value": product_commit,
                "reason": str(attestation_status.get("reason")),
            }
        )
        product_commit = None
        product_commit_source = None

    container_digest, container_digest_source = runner_image_digest()
    artifact_digests = []
    if input_manifest_sha256 is not None:
        artifact_digests.append(
            {
                "kind": "quantitative_product_manifest",
                "path": entry["path"],
                "sha256": input_manifest_sha256,
            }
        )

    record: dict[str, Any] = {
        "command": BENCHMARK_COMMAND,
        "runner": BENCHMARK_RUNNER,
        "compose_file": BENCHMARK_COMPOSE_FILE,
        "repository_head": repository_head,
        "env_profile": env_profile(args),
        "source_kind": entry["source_kind"],
        "input_manifest": entry["path"],
        "input_manifest_sha256": input_manifest_sha256,
        "artifact_digests": artifact_digests,
        "container_image_digest": container_digest,
        "product_commit": product_commit,
        "product_commit_source": product_commit_source,
    }
    if container_digest_source is not None:
        record["container_image_digest_source"] = container_digest_source
    if rejected:
        record["rejected_product_commit_values"] = rejected
    if attestation_status is not None:
        record["runtime_source_attestation"] = attestation_status
    record["missing_fields"] = missing_reproducibility_fields(record)
    record["public_reproducible"] = not record["missing_fields"]
    return record


def load_product_manifest(path: Path) -> dict[str, Any]:
    manifest = read_json(resolve_artifact_path(path))
    if manifest.get("schema") != PRODUCT_MANIFEST_SCHEMA:
        raise SystemExit(f"{path} has unsupported schema {manifest.get('schema')!r}")
    return manifest


def row_key(row: dict[str, Any]) -> tuple[Any, Any]:
    return row.get("product"), row.get("adapter_id")


def main() -> None:
    args = parse_args()
    inputs = combined_inputs(sync_log_rows(args.sync_log))
    if not inputs:
        raise SystemExit(f"{args.sync_log} has no combined-input rows")

    combined = load_product_manifest(args.combined_product_manifest)
    combined_rows = combined.get("rows", [])
    combined_keys = {row_key(row) for row in combined_rows}
    repository_head = git_head()
    missing_field_counts: dict[str, int] = {}
    ready_row_count = 0
    input_row_count = 0
    missing_from_combined = []
    product_commit_gap_rows = []
    combined_input_records = []

    for entry in inputs:
        path = resolve_artifact_path(entry["path"])
        input_digest = sha256_file(path)
        manifest = load_product_manifest(path)
        rows = []
        for row in manifest.get("rows", []):
            input_row_count += 1
            reproducibility = build_reproducibility_record(
                args=args,
                repository_head=repository_head,
                entry=entry,
                manifest=manifest,
                row=row,
                input_manifest_sha256=input_digest,
            )
            for field in reproducibility["missing_fields"]:
                missing_field_counts[field] = missing_field_counts.get(field, 0) + 1
            if reproducibility["public_reproducible"]:
                ready_row_count += 1

            row_record = {
                "label": entry["label"],
                "source_kind": entry["source_kind"],
                "product": row.get("product"),
                "adapter_id": row.get("adapter_id"),
                "evidence_class": row.get("evidence_class"),
                "result_state": row.get("result_state"),
                "leaderboard_eligible": bool(row.get("leaderboard_eligible")),
                "metric_comparable": bool(row.get("metric_comparable")),
                "present_in_combined_manifest": row_key(row) in combined_keys,
                "reproducibility": reproducibility,
            }
            if not row_record["present_in_combined_manifest"]:
                missing_from_combined.append(row_record)
            if "product_commit" in reproducibility["missing_fields"]:
                product_commit_gap_rows.append(row_record)
            rows.append(row_record)

        combined_input_records.append(
            {
                **entry,
                "status": "loaded",
                "row_count": len(rows),
                "input_manifest_sha256": input_digest,
                "rows": rows,
            }
        )

    manifest = {
        "schema": SCHEMA,
        "generated_at": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "status": "pass" if not missing_from_combined else "fail",
        "run_live_explicit_qrels": args.run_live_explicit_qrels,
        "run_langgraph": args.run_langgraph,
        "policy": {
            "combined_manifest_rows_must_have_input_provenance": True,
            "public_reproducibility_requires_runner_digest": True,
            "runtime_sensitive_product_commits_require_runtime_attestation": True,
        },
        "combined_product_manifest": args.combined_product_manifest.as_posix(),
        "combined_input_count": len(inputs),
        "input_row_count": input_row_count,
        "combined_row_count": len(combined_rows),
        "missing_from_combined_count": len(missing_from_combined),
        "missing_from_combined": missing_from_combined,
        "product_commit_gap_rows": product_commit_gap_rows,
        "reproducibility_summary": {
            "state": (
                "public_reproducibility_ready"
                if input_row_count > 0 and ready_row_count == input_row_count and not missing_field_counts
                else "public_reproducibility_not_ready"
            ),
            "row_count": input_row_count,
            "ready_row_count": ready_row_count,
            "required_fields": REPRODUCIBILITY_REQUIRED_FIELDS,
            "missing_field_counts": missing_field_counts,
            "product_commit_gap_count": len(product_commit_gap_rows),
            "public_reproducible_claim_allowed": (
                input_row_count > 0 and ready_row_count == input_row_count and not missing_field_counts
            ),
            "claim_boundary": (
                "A row may support a public reproducibility claim only when it carries "
                "the aggregate command, Docker runner, compose file, repository head, "
                "environment profile, input product-manifest SHA-256, artifact digest, "
                "container image digest, and structured 40-hex product source commit provenance."
            ),
        },
        "combined_inputs": combined_input_records,
    }

    write_json(args.out, manifest)
    if manifest["status"] != "pass":
        raise SystemExit(
            "quantitative artifact freshness gate failed: "
            f"{len(missing_from_combined)} input rows missing from combined manifest"
        )


if __name__ == "__main__":
    main()
