#!/usr/bin/env python3
"""Canonical opacity, validation, normalization, and scoring contract."""

from __future__ import annotations

import hashlib
import json
import math
import re
import statistics
from pathlib import Path
from typing import Any


PRODUCT_IDS = {
    "elf",
    "mem0",
    "qmd",
    "lightrag",
    "openviking",
    "graphrag",
    "graphiti",
    "letta",
    "sag",
    "pageindex",
    "openkb",
    "honcho",
}
SUITE_IDS = {
    "common-core-v1": 24,
    "memory-lifecycle-v1": 8,
    "knowledge-structure-v1": 8,
    "repository-knowledge-v1": 8,
}
FAILURE_CLASSES = {
    "completed",
    "product_failed",
    "adapter_failed",
    "harness_failed",
    "provider_failed",
    "configuration_failed",
    "timeout_failed",
    "cleanup_failed",
    "not_applicable",
}
FORBIDDEN_PRODUCT_KEYS = {
    "answer_facts",
    "expected_answer",
    "expect_unsupported",
    "forbidden_answer_facts",
    "forbidden_evidence",
    "negative_traps",
    "privacy",
    "qrels",
    "required_evidence",
    "required_operations",
    "scoring_rubric",
}


class NativeContextError(ValueError):
    """A mutation job omitted or malformed its native ranked readback."""


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return value


def canonical_json(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
        + "\n"
    ).encode("utf-8")


def sha256_json(value: Any) -> str:
    return hashlib.sha256(canonical_json(value)).hexdigest()


def opaque_identifier(kind: str, value: str) -> str:
    digest = hashlib.sha256(
        f"elf-benchmark-v4:{kind}:{value}".encode("utf-8")
    ).hexdigest()[:24]
    return f"{kind[:1]}_{digest}"


def opaque_job_id(job_id: str) -> str:
    return opaque_identifier("job", job_id)


def opaque_evidence_id(evidence_id: str) -> str:
    return opaque_identifier("evidence", evidence_id)


def assert_product_payload_is_blind(value: Any, path: str = "$") -> None:
    if isinstance(value, dict):
        leaked = FORBIDDEN_PRODUCT_KEYS.intersection(value)
        if leaked:
            raise ValueError(
                f"product payload leaks evaluator keys at {path}: "
                + ", ".join(sorted(leaked))
            )
        for key, child in value.items():
            assert_product_payload_is_blind(child, f"{path}.{key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            assert_product_payload_is_blind(child, f"{path}[{index}]")


def validate_manifest(manifest: dict[str, Any]) -> None:
    if manifest.get("schema") != "elf.benchmark_manifest/v4":
        raise ValueError("manifest schema must be elf.benchmark_manifest/v4")
    runner = manifest.get("runner")
    if not isinstance(runner, dict) or int(runner.get("capacity", 0)) not in {1, 2}:
        raise ValueError("runner capacity must be one or two")
    targets = manifest.get("targets")
    if not isinstance(targets, list):
        raise ValueError("manifest targets must be a list")
    target_ids = [target.get("id") for target in targets if isinstance(target, dict)]
    if set(target_ids) != PRODUCT_IDS or len(target_ids) != len(PRODUCT_IDS):
        raise ValueError("manifest must contain every retained product exactly once")
    suites = manifest.get("suites")
    if not isinstance(suites, list):
        raise ValueError("manifest suites must be a list")
    suite_ids = [suite.get("id") for suite in suites if isinstance(suite, dict)]
    if set(suite_ids) != set(SUITE_IDS) or len(suite_ids) != len(SUITE_IDS):
        raise ValueError("manifest must contain common-core and exactly three capability suites")
    if set(manifest.get("failure_classes") or {}) != FAILURE_CLASSES:
        raise ValueError("manifest failure taxonomy differs from the normalized schema")
    for target in targets:
        pin = target.get("pin")
        if not isinstance(pin, dict) or not pin.get("kind"):
            raise ValueError(f"{target.get('id')} has no pinned runtime identity")
        target_suites = target.get("suites")
        if not isinstance(target_suites, list) or not set(target_suites) <= set(SUITE_IDS):
            raise ValueError(f"{target.get('id')} has invalid suite eligibility")


def validate_suite(suite: dict[str, Any]) -> None:
    suite_id = suite.get("suite_id")
    if suite_id not in SUITE_IDS:
        raise ValueError(f"unknown suite id: {suite_id}")
    jobs = suite.get("jobs")
    expected_job_count = SUITE_IDS[suite_id]
    readiness_subset = suite.get("execution_mode") == "readiness"
    valid_job_count = (
        isinstance(jobs, list)
        and (
            1 <= len(jobs) <= expected_job_count
            if readiness_subset
            else len(jobs) == expected_job_count
        )
    )
    if not valid_job_count:
        raise ValueError(
            f"{suite_id} must contain exactly {expected_job_count} jobs"
            if not readiness_subset
            else f"{suite_id} readiness subset must contain 1-{expected_job_count} jobs"
        )
    job_ids: set[str] = set()
    for job in jobs:
        if not isinstance(job, dict):
            raise ValueError(f"{suite_id} contains a non-object job")
        job_id = job.get("job_id")
        if not isinstance(job_id, str) or not job_id or job_id in job_ids:
            raise ValueError(f"{suite_id} has an invalid or duplicate job id")
        job_ids.add(job_id)
        corpus = job.get("corpus")
        if not isinstance(corpus, list) or not corpus:
            raise ValueError(f"{job_id} has no corpus")
        evidence_ids = [item.get("evidence_id") for item in corpus]
        if any(not isinstance(value, str) or not value for value in evidence_ids):
            raise ValueError(f"{job_id} has an invalid evidence id")
        if len(set(evidence_ids)) != len(evidence_ids):
            raise ValueError(f"{job_id} has duplicate evidence ids")
        qrels = job.get("qrels")
        if not isinstance(qrels, dict):
            raise ValueError(f"{job_id} has no scorer-only qrels")
        referenced = set(qrels.get("relevant_evidence") or []) | set(
            qrels.get("forbidden_evidence") or []
        )
        if not referenced <= set(evidence_ids):
            raise ValueError(f"{job_id} qrels reference unknown evidence")
        operations = job.get("operations") or []
        if suite.get("kind") in {"memory_lifecycle", "repository_knowledge"}:
            if not operations:
                raise ValueError(f"{job_id} must execute a native operation")
        for operation in operations:
            if operation.get("type") not in {"update", "delete"}:
                raise ValueError(f"{job_id} has an unsupported operation")
            if operation.get("evidence_id") not in evidence_ids:
                raise ValueError(f"{job_id} operation references unknown evidence")
            if operation["type"] == "update" and not operation.get("text"):
                raise ValueError(f"{job_id} update has no replacement text")


def product_job(job: dict[str, Any], suite: dict[str, Any]) -> dict[str, Any]:
    operations = []
    for operation in job.get("operations") or []:
        product_operation = {
            "type": operation["type"],
            "evidence_id": opaque_evidence_id(operation["evidence_id"]),
        }
        if operation.get("text") is not None:
            product_operation["text"] = operation["text"]
        operations.append(product_operation)
    payload = {
        "schema": "elf.real_world_job/v1",
        "job_id": opaque_job_id(job["job_id"]),
        "suite": suite["kind"],
        "title": "Benchmark task",
        "corpus": {
            "items": [
                {
                    "evidence_id": opaque_evidence_id(item["evidence_id"]),
                    "text": item["text"],
                }
                for item in job["corpus"]
            ]
        },
        "prompt": {"content": job["query"]},
        "operations": operations,
        "memory_evolution": None,
    }
    assert_product_payload_is_blind(payload)
    return payload


def materialize_product_fixtures(
    suite: dict[str, Any], destination: Path
) -> list[Path]:
    validate_suite(suite)
    destination.mkdir(parents=True, exist_ok=True)
    paths: list[Path] = []
    for job in suite["jobs"]:
        payload = product_job(job, suite)
        path = destination / f"{opaque_job_id(job['job_id'])}.json"
        path.write_bytes(canonical_json(payload))
        paths.append(path)
    return paths


def _dedupe(values: list[str]) -> list[str]:
    output: list[str] = []
    for value in values:
        if value not in output:
            output.append(value)
    return output


def _ndcg_at_five(retrieved: list[str], relevant: set[str]) -> float | None:
    if not relevant:
        return None
    dcg = sum(
        1.0 / math.log2(index + 2)
        for index, evidence_id in enumerate(retrieved[:5])
        if evidence_id in relevant
    )
    ideal = sum(1.0 / math.log2(index + 2) for index in range(min(5, len(relevant))))
    return dcg / ideal


def _mean(values: list[float | None]) -> float | None:
    measured = [value for value in values if value is not None]
    return sum(measured) / len(measured) if measured else None


def _rounded(value: float | None) -> float | None:
    return round(value, 6) if value is not None else None


def _ci95(values: list[float | None]) -> dict[str, Any] | None:
    measured = [value for value in values if value is not None]
    if not measured:
        return None
    mean = statistics.fmean(measured)
    if len(measured) == 1:
        lower = upper = mean
    else:
        margin = 1.96 * statistics.stdev(measured) / math.sqrt(len(measured))
        lower, upper = max(0.0, mean - margin), min(1.0, mean + margin)
    return {
        "method": "job_level_normal_approximation",
        "n": len(measured),
        "mean": _rounded(mean),
        "lower": _rounded(lower),
        "upper": _rounded(upper),
    }


def _normalized_text(value: str) -> str:
    return " ".join(re.findall(r"[a-z0-9._:/+-]+", value.casefold()))


def _answer_scores(qrels: dict[str, Any], raw_answer: Any) -> tuple[Any, Any, list[str]]:
    if not isinstance(raw_answer, dict):
        return None, None, []
    text = str(raw_answer.get("text") or "")
    normalized = _normalized_text(text)
    forbidden = [
        fact
        for fact in qrels.get("forbidden_answer_facts") or []
        if _normalized_text(str(fact)) in normalized
    ]
    if qrels.get("expect_unsupported"):
        unsupported_error = (
            raw_answer.get("supported") is not False or normalized != "unknown"
        )
        return float(not unsupported_error), float(unsupported_error), forbidden
    facts = [str(fact) for fact in qrels.get("answer_facts") or []]
    correct = bool(facts) and all(_normalized_text(fact) in normalized for fact in facts)
    correct = correct and not forbidden and raw_answer.get("supported") is not False
    return float(correct), None, forbidden


def _normalized_native_contexts(
    raw: Any, evidence_id_map: dict[str, str]
) -> list[dict[str, Any]] | None:
    if not isinstance(raw, list):
        return None
    contexts: list[dict[str, Any]] = []
    for item in raw:
        if not isinstance(item, dict) or not isinstance(item.get("text"), str):
            return None
        evidence_id = item.get("evidence_id")
        if evidence_id is not None and not isinstance(evidence_id, str):
            return None
        contexts.append(
            {
                "evidence_id": evidence_id_map.get(evidence_id, evidence_id),
                "text": item["text"],
            }
        )
    return contexts


def _operation_scores(
    expected: list[str],
    operations: list[dict[str, Any]],
    native: Any,
    retrieved: list[str],
    contexts: list[dict[str, Any]] | None,
) -> tuple[float | None, float | None, list[str]]:
    rows = native if isinstance(native, list) else []
    missing: list[str] = []
    update_values: list[float] = []
    delete_values: list[float] = []
    for requested in expected:
        operation = next(
            (item for item in operations if item.get("type") == requested), None
        )
        row = next(
            (
                item
                for item in rows
                if isinstance(item, dict)
                and item.get("requested_type") == requested
                and item.get("classification") == "completed"
            ),
            None,
        )
        if row is None:
            missing.append(requested)
            value = 0.0
            native_type = None
        else:
            native_type = row.get("native_type")
            value = 1.0 if row.get("native_success") is True else 0.0
        if requested == "update":
            exact = native_type in {"update", "replace", "reindex_update"}
            evidence_id = operation.get("evidence_id") if operation else None
            replacement = operation.get("text") if operation else None
            readback = bool(
                contexts is not None
                and isinstance(evidence_id, str)
                and isinstance(replacement, str)
                and evidence_id in retrieved
                and any(
                    context.get("evidence_id") == evidence_id
                    and _normalized_text(replacement)
                    in _normalized_text(str(context.get("text") or ""))
                    for context in contexts
                )
            )
            update_values.append(value if exact and readback else 0.0)
        elif requested == "delete":
            exact = native_type in {"delete", "forget", "session_delete"}
            evidence_id = operation.get("evidence_id") if operation else None
            readback = bool(
                contexts is not None
                and isinstance(evidence_id, str)
                and evidence_id not in retrieved
                and all(
                    context.get("evidence_id") != evidence_id for context in contexts
                )
            )
            delete_values.append(value if exact and readback else 0.0)
    return _mean(update_values), _mean(delete_values), missing


def evaluate_unit(
    suite: dict[str, Any], unit: dict[str, Any], target: dict[str, Any]
) -> dict[str, Any]:
    """Normalize one native unit and score only centrally visible evidence."""
    validate_suite(suite)
    target_id = target["id"]
    if unit.get("schema") != "elf.benchmark_unit_result/v4":
        raise ValueError(f"{target_id} unit schema is not v4")
    if unit.get("target") != target_id:
        raise ValueError(f"{target_id} unit target identity differs from the manifest")
    if unit.get("result_class") not in FAILURE_CLASSES:
        raise ValueError(f"{target_id} unit has an unknown result class")
    if unit.get("result_class") == "completed":
        if unit.get("ingest_count") != 1:
            raise ValueError(f"{target_id} completed unit did not ingest exactly once")
        if unit.get("warm_reused_state") is not True:
            raise ValueError(f"{target_id} completed unit did not reuse cold state for warm")
    score_eligible = bool(unit.get("score_eligible"))
    if score_eligible != bool(target.get("score_eligible")):
        raise ValueError(f"{target_id} score eligibility differs from the manifest")

    jobs_by_id = {job["job_id"]: job for job in suite["jobs"]}
    job_id_map = {opaque_job_id(job_id): job_id for job_id in jobs_by_id}
    evidence_id_map = {
        opaque_evidence_id(item["evidence_id"]): item["evidence_id"]
        for job in suite["jobs"]
        for item in job["corpus"]
    }
    suite_kind = suite["kind"]
    contract_failures: list[str] = []
    phases: dict[str, Any] = {}
    for phase_name in ("cold", "warm"):
        raw_phase = (unit.get("phases") or {}).get(phase_name) or {
            "status": unit.get("result_class", "harness_failed"),
            "jobs": [],
        }
        normalized_rows: list[dict[str, Any]] = []
        normalized_job_ids: list[str] = []
        for raw_job in raw_phase.get("jobs") or []:
            if raw_job.get("classification") not in FAILURE_CLASSES:
                raise ValueError(f"{target_id} job has an unknown failure classification")
            raw_job_id = str(raw_job.get("job_id") or "unknown")
            job_id = job_id_map.get(raw_job_id, raw_job_id)
            normalized_job_ids.append(job_id)
            definition = jobs_by_id.get(job_id)
            if definition is None:
                continue
            qrels = definition["qrels"]
            raw_evidence = _dedupe(
                [str(evidence_id) for evidence_id in raw_job.get("evidence_ids") or []]
            )[:5]
            original_evidence_ids = set(evidence_id_map.values())
            retrieved = [
                evidence_id_map.get(evidence_id, evidence_id)
                for evidence_id in raw_evidence
            ]
            native_contexts = (
                _normalized_native_contexts(raw_job.get("contexts"), evidence_id_map)
                if "contexts" in raw_job
                else None
            )
            requires_native_readback = bool(definition.get("operations"))
            if phase_name == "warm" and requires_native_readback:
                if "contexts" not in raw_job:
                    contract_failures.append(
                        f"{job_id} omitted native ranked contexts after mutation"
                    )
                elif native_contexts is None:
                    contract_failures.append(
                        f"{job_id} returned malformed native ranked contexts after mutation"
                    )
            relevant = set(qrels.get("relevant_evidence") or [])
            forbidden = set(qrels.get("forbidden_evidence") or [])
            relevant_hits = [value for value in retrieved if value in relevant]
            forbidden_hits = [value for value in retrieved if value in forbidden]
            returned_count = int(raw_job.get("returned_count") or 0)
            mapped_count = sum(
                evidence_id in evidence_id_map or evidence_id in original_evidence_ids
                for evidence_id in raw_evidence
            )
            trace_rate = (
                mapped_count / min(5, returned_count)
                if returned_count > 0
                else None
            )
            answer_correct, unsupported_error, forbidden_answer = _answer_scores(
                qrels, raw_job.get("answer")
            )
            update_success, delete_success, missing_operations = _operation_scores(
                list(qrels.get("required_operations") or []),
                list(definition.get("operations") or []),
                raw_job.get("operations"),
                retrieved,
                native_contexts,
            )
            if phase_name == "warm" and missing_operations:
                contract_failures.append(
                    f"{job_id} omitted native operations: {', '.join(missing_operations)}"
                )
            privacy_violation = None
            if qrels.get("privacy"):
                privacy_violation = float(bool(forbidden_hits or forbidden_answer))
            normalized_rows.append(
                {
                    "job_id": job_id,
                    "classification": raw_job.get(
                        "classification", raw_phase.get("status")
                    ),
                    "retrieved_evidence": retrieved,
                    "native_contexts": native_contexts or [],
                    "recall_at_5": _rounded(
                        len(relevant_hits) / len(relevant) if relevant else None
                    ),
                    "ndcg_at_5": _rounded(_ndcg_at_five(retrieved, relevant)),
                    "forbidden_evidence_hits": forbidden_hits,
                    "source_trace_rate": _rounded(trace_rate),
                    "answer": raw_job.get("answer"),
                    "answer_correct": answer_correct,
                    "unsupported_answer_error": unsupported_error,
                    "forbidden_answer_facts": forbidden_answer,
                    "native_operations": raw_job.get("operations") or [],
                    "native_update_success": update_success,
                    "native_delete_success": delete_success,
                    "privacy_scope_violation": privacy_violation,
                    "latency_ms": raw_job.get("latency_ms"),
                    "failure": raw_job.get("failure"),
                }
            )

        expected_ids = set(jobs_by_id)
        actual_ids = set(normalized_job_ids)
        duplicates = sorted(
            job_id
            for job_id in actual_ids
            if normalized_job_ids.count(job_id) > 1
        )
        coverage = {
            "expected": len(expected_ids),
            "actual": len(normalized_job_ids),
            "missing": sorted(expected_ids - actual_ids),
            "unexpected": sorted(actual_ids - expected_ids),
            "duplicates": duplicates,
            "passed": expected_ids == actual_ids
            and not duplicates
            and len(normalized_job_ids) == len(expected_ids),
        }
        completed_rows = [
            row for row in normalized_rows if row["classification"] == "completed"
        ]
        phase_class = raw_phase.get("status", unit.get("result_class"))
        if unit.get("result_class") == "completed":
            if phase_class != "completed":
                contract_failures.append(
                    f"{phase_name} phase reported {phase_class} in a completed unit"
                )
            if not coverage["passed"]:
                contract_failures.append(
                    f"{phase_name} phase did not return the exact scheduled job set"
                )
            if len(completed_rows) != len(normalized_rows):
                contract_failures.append(
                    f"{phase_name} phase contains non-completed job rows"
                )
        quality_denominator = bool(
            unit.get("result_class") == "completed"
            and score_eligible
            and phase_class == "completed"
            and coverage["passed"]
            and len(completed_rows) == len(normalized_rows)
            and not (phase_name == "cold" and suite_kind != "retrieval")
        )
        metric_rows = completed_rows if quality_denominator else []
        metrics = None
        if metric_rows:
            metrics = {
                "mean_recall_at_5": _rounded(
                    _mean([row["recall_at_5"] for row in metric_rows])
                ),
                "mean_ndcg_at_5": _rounded(
                    _mean([row["ndcg_at_5"] for row in metric_rows])
                ),
                "forbidden_or_stale_evidence_hit_rate": _rounded(
                    _mean(
                        [float(bool(row["forbidden_evidence_hits"])) for row in metric_rows]
                    )
                ),
                "source_or_citation_trace_rate": _rounded(
                    _mean([row["source_trace_rate"] for row in metric_rows])
                ),
                "programmatic_answer_correctness": _rounded(
                    _mean([row["answer_correct"] for row in metric_rows])
                ),
                "native_correction_and_update_success": _rounded(
                    _mean([row["native_update_success"] for row in metric_rows])
                ),
                "native_deletion_or_forgetting_success": _rounded(
                    _mean([row["native_delete_success"] for row in metric_rows])
                ),
                "privacy_scope_violation_rate": _rounded(
                    _mean([row["privacy_scope_violation"] for row in metric_rows])
                ),
                "unsupported_answer_rate": _rounded(
                    _mean([row["unsupported_answer_error"] for row in metric_rows])
                ),
                "mean_query_latency_ms": _rounded(
                    _mean(
                        [
                            float(row["latency_ms"])
                            for row in metric_rows
                            if row["latency_ms"] is not None
                        ]
                    )
                ),
            }
            if suite["suite_id"] == "common-core-v1":
                metrics["job_level_95ci"] = {
                    "recall_at_5": _ci95([row["recall_at_5"] for row in metric_rows]),
                    "ndcg_at_5": _ci95([row["ndcg_at_5"] for row in metric_rows]),
                    "answer_correctness": _ci95(
                        [row["answer_correct"] for row in metric_rows]
                    ),
                }
        phases[phase_name] = {
            "classification": phase_class,
            "quality_denominator": quality_denominator,
            "coverage": coverage,
            "counts": {
                "scheduled": len(expected_ids),
                "completed": len(completed_rows),
                "failed": sum(
                    row["classification"] not in {"completed", "not_applicable"}
                    for row in normalized_rows
                ),
                "not_applicable": sum(
                    row["classification"] == "not_applicable"
                    for row in normalized_rows
                ),
                "scored": len(metric_rows),
            },
            "metrics": metrics,
            "native_metadata": raw_phase.get("adapter_metadata") or {},
            "jobs": sorted(normalized_rows, key=lambda row: row["job_id"]),
        }

    classification = str(unit.get("result_class") or "harness_failed")
    if classification == "completed" and contract_failures:
        classification = "adapter_failed"
        for phase in phases.values():
            phase["quality_denominator"] = False
            phase["metrics"] = None
            phase["counts"]["scored"] = 0
    return {
        "schema": "elf.benchmark_result/v1",
        "suite_id": suite["suite_id"],
        "suite_kind": suite_kind,
        "target": target_id,
        "classification": classification,
        "score_eligible": score_eligible,
        "native_mode": unit.get("native_mode"),
        "contract_failures": contract_failures,
        "cold_ingest_duration_ms": unit.get("ingest_duration_ms"),
        "provider_usage": unit.get("provider_usage") or {},
        "phases": phases,
    }


def answer_cases(
    suite: dict[str, Any], unit: dict[str, Any], context_budget_chars: int
) -> list[dict[str, Any]]:
    """Build target-blind warm answer cases from only retrieved source context."""
    jobs = {opaque_job_id(job["job_id"]): job for job in suite["jobs"]}
    jobs.update({job["job_id"]: job for job in suite["jobs"]})
    cases: list[dict[str, Any]] = []
    for raw in (unit.get("phases") or {}).get("warm", {}).get("jobs", []):
        job = jobs.get(raw.get("job_id"))
        if job is None or raw.get("classification") != "completed":
            continue
        source_evidence = {item["evidence_id"]: item["text"] for item in job["corpus"]}
        evidence = {
            key: text
            for evidence_id, text in source_evidence.items()
            for key in (evidence_id, opaque_evidence_id(evidence_id))
        }
        context = []
        used = 0
        if "contexts" in raw:
            native_contexts = raw.get("contexts")
            if not isinstance(native_contexts, list) or any(
                not isinstance(item, dict)
                or not isinstance(item.get("text"), str)
                or (
                    item.get("evidence_id") is not None
                    and not isinstance(item.get("evidence_id"), str)
                )
                for item in native_contexts
            ):
                raise NativeContextError(
                    f"{raw.get('job_id')} returned malformed native ranked contexts"
                )
            ranked_contexts = native_contexts
        elif job.get("operations"):
            raise NativeContextError(
                f"{raw.get('job_id')} omitted native ranked contexts after mutation"
            )
        else:
            ranked_contexts = [
                {"evidence_id": evidence_id, "text": evidence.get(evidence_id)}
                for evidence_id in raw.get("evidence_ids") or []
            ]
        for rank, item in enumerate(ranked_contexts, start=1):
            if not isinstance(item, dict):
                continue
            text = item.get("text")
            if text is None:
                continue
            rendered = f"[{rank}] {text}"
            if used + len(rendered) > context_budget_chars:
                break
            context.append(rendered)
            used += len(rendered)
        cases.append(
            {
                "case_id": raw["job_id"],
                "question": job["query"],
                "context": context,
            }
        )
    return cases
