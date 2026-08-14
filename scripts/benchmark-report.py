#!/usr/bin/env python3
"""Publish the single English decision report from one measured result bundle."""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable


SUITE_NAMES = {
    "common-core-v1": "Common Core (24 jobs)",
    "memory-lifecycle-v1": "Memory Lifecycle (8 jobs)",
    "knowledge-structure-v1": "Knowledge Structure (8 jobs)",
    "repository-knowledge-v1": "Repository Knowledge (8 jobs)",
}
METRIC_NAMES = {
    "mean_recall_at_5": "Recall@5",
    "mean_ndcg_at_5": "nDCG@5",
    "forbidden_or_stale_evidence_hit_rate": "Forbidden or stale evidence hit rate",
    "source_or_citation_trace_rate": "Source or citation trace rate",
    "programmatic_answer_correctness": "Programmatic answer correctness",
    "native_correction_and_update_success": "Native update success",
    "native_deletion_or_forgetting_success": "Native deletion success",
    "privacy_scope_violation_rate": "Privacy-scope violation rate",
    "unsupported_answer_rate": "Unsupported-answer rate",
    "mean_query_latency_ms": "Warm query latency (ms)",
}
LOWER_IS_BETTER = {
    "forbidden_or_stale_evidence_hit_rate",
    "privacy_scope_violation_rate",
    "unsupported_answer_rate",
    "mean_query_latency_ms",
}
SHARED_ANSWER_METRICS = {
    "programmatic_answer_correctness",
    "unsupported_answer_rate",
}
BASE_TABLE_METRICS = (
    "mean_recall_at_5",
    "mean_ndcg_at_5",
    "forbidden_or_stale_evidence_hit_rate",
    "source_or_citation_trace_rate",
    "programmatic_answer_correctness",
    "unsupported_answer_rate",
)
SUITE_TABLE_METRICS = {
    "common-core-v1": BASE_TABLE_METRICS,
    "knowledge-structure-v1": BASE_TABLE_METRICS,
    "memory-lifecycle-v1": BASE_TABLE_METRICS
    + (
        "native_correction_and_update_success",
        "native_deletion_or_forgetting_success",
        "privacy_scope_violation_rate",
    ),
    "repository-knowledge-v1": BASE_TABLE_METRICS
    + (
        "native_correction_and_update_success",
        "native_deletion_or_forgetting_success",
        "privacy_scope_violation_rate",
    ),
}
TABLE_LABELS = {
    "mean_recall_at_5": "Recall@5",
    "mean_ndcg_at_5": "nDCG@5",
    "forbidden_or_stale_evidence_hit_rate": "Stale hit rate (lower is better)",
    "source_or_citation_trace_rate": "Source trace",
    "programmatic_answer_correctness": "Answer correct",
    "unsupported_answer_rate": "Unsupported answer (lower is better)",
    "native_correction_and_update_success": "Update",
    "native_deletion_or_forgetting_success": "Delete",
    "privacy_scope_violation_rate": "Privacy violation (lower is better)",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    return parser.parse_args()


def fmt(value: Any, digits: int = 3) -> str:
    if value is None:
        return "—"
    if isinstance(value, bool):
        return "Yes" if value else "No"
    if isinstance(value, float):
        return f"{value:.{digits}f}"
    return str(value)


def esc(value: Any) -> str:
    return str(value).replace("|", "\\|").replace("\n", " ")


def warm(result: dict[str, Any]) -> dict[str, Any]:
    return result["evaluation"]["phases"]["warm"]


def metrics(result: dict[str, Any]) -> dict[str, Any]:
    return warm(result).get("metrics") or {}


def decision_metric(result: dict[str, Any], name: str) -> Any:
    value = metrics(result).get(name)
    if name in SHARED_ANSWER_METRICS:
        return None
    if (
        name
        in {
            "forbidden_or_stale_evidence_hit_rate",
            "privacy_scope_violation_rate",
        }
        and metrics(result).get("mean_recall_at_5") == 0
    ):
        return None
    return value


def retrieval_effective(result: dict[str, Any]) -> bool:
    value = metrics(result).get("mean_recall_at_5")
    return isinstance(value, (int, float)) and value > 0


def count_text(phase: dict[str, Any]) -> str:
    counts = phase["counts"]
    return "/".join(
        str(counts[name])
        for name in ("scheduled", "completed", "failed", "not_applicable", "scored")
    )


def result_table(
    rows: list[dict[str, Any]], suite_id: str | None = None
) -> list[str]:
    rows = comparable(rows)
    selected = [
        name
        for name in SUITE_TABLE_METRICS.get(suite_id, BASE_TABLE_METRICS)
        if any(metrics(row).get(name) is not None for row in rows)
    ]
    if suite_id is None:
        optional = (
            "native_correction_and_update_success",
            "native_deletion_or_forgetting_success",
            "privacy_scope_violation_rate",
        )
        selected.extend(
            name
            for name in optional
            if any(metrics(row).get(name) is not None for row in rows)
        )
    headers = [
        "Product",
        "Result",
        "Scheduled/Completed/Failed/Not applicable/Scored",
        *(TABLE_LABELS[name] for name in selected),
        "Cold ingest (ms)",
        "Warm query (ms)",
    ]
    lines = [
        "| " + " | ".join(headers) + " |",
        "|" + "|".join(["---", "---", "---:"] + ["---:"] * (len(headers) - 3)) + "|",
    ]
    for row in rows:
        evaluation = row["evaluation"]
        phase = warm(row)
        value = metrics(row)
        fields = [
            esc(row["target"]),
            esc(evaluation["classification"]),
            count_text(phase),
            *(fmt(value.get(name)) for name in selected),
            fmt(evaluation.get("cold_ingest_duration_ms"), 1),
            fmt(value.get("mean_query_latency_ms"), 1),
        ]
        lines.append("| " + " | ".join(fields) + " |")
    return lines


def comparable(rows: Iterable[dict[str, Any]]) -> list[dict[str, Any]]:
    return [
        row
        for row in rows
        if row["evaluation"]["classification"] == "completed"
        and row["evaluation"].get("score_eligible")
        and warm(row).get("quality_denominator")
    ]


def metric_leaders(
    rows: list[dict[str, Any]], metric_names: Iterable[str]
) -> list[str]:
    completed = comparable(rows)
    output: list[str] = []
    for name in metric_names:
        measured = [
            (row["target"], decision_metric(row, name))
            for row in completed
            if decision_metric(row, name) is not None
        ]
        if not measured:
            continue
        best = (
            min(value for _, value in measured)
            if name in LOWER_IS_BETTER
            else max(value for _, value in measured)
        )
        leaders = sorted(target for target, value in measured if value == best)
        output.append(f"{METRIC_NAMES[name]}: {', '.join(leaders)} ({fmt(best)})")
    return output


def failure_message(row: dict[str, Any]) -> str:
    unit = row.get("unit_result") or {}
    failure = unit.get("failure")
    if isinstance(failure, dict) and failure.get("message"):
        return str(failure["message"])
    failures = row["evaluation"].get("contract_failures") or []
    if failures:
        return "; ".join(map(str, failures))
    for phase_name in ("cold", "warm"):
        for job in row["evaluation"]["phases"][phase_name].get("jobs") or []:
            if job.get("failure"):
                return str(job["failure"])
    return "No additional error detail was provided"


def coverage_table(bundle: dict[str, Any]) -> list[str]:
    lines = [
        "| Suite | Product | Result | Scheduled | Completed | Failed | Not applicable | Scored | Cleanup passed |",
        "|---|---|---|---:|---:|---:|---:|---:|---|",
    ]
    for suite_id, suite in bundle["suite_results"].items():
        for row in suite["results"]:
            counts = warm(row)["counts"]
            lines.append(
                "| {suite} | {target} | {result} | {scheduled} | {completed} | {failed} | {not_applicable} | {scored} | {cleanup} |".format(
                    suite=esc(suite_id),
                    target=esc(row["target"]),
                    result=esc(row["evaluation"]["classification"]),
                    scheduled=counts["scheduled"],
                    completed=counts["completed"],
                    failed=counts["failed"],
                    not_applicable=counts["not_applicable"],
                    scored=counts["scored"],
                    cleanup=fmt(row["cleanup"].get("passed")),
                )
            )
    return lines


def failure_table(bundle: dict[str, Any]) -> list[str]:
    lines = [
        "| Suite | Product | Type | Detail |",
        "|---|---|---|---|",
    ]
    found = False
    for suite_id, suite in bundle["suite_results"].items():
        for row in suite["results"]:
            classification = row["evaluation"]["classification"]
            if classification == "completed":
                continue
            found = True
            lines.append(
                f"| {esc(suite_id)} | {esc(row['target'])} | {esc(classification)} | {esc(failure_message(row))} |"
            )
    if not found:
        lines.append("| — | — | — | No typed failures |")
    return lines


def common_ci(rows: list[dict[str, Any]]) -> list[str]:
    lines = [
        "| Product | Metric | n | Mean | 95% CI |",
        "|---|---|---:|---:|---:|",
    ]
    found = False
    for row in comparable(rows):
        ci = metrics(row).get("job_level_95ci") or {}
        for name, label in (
            ("recall_at_5", "Recall@5"),
            ("ndcg_at_5", "nDCG@5"),
            ("answer_correctness", "Answer correctness"),
        ):
            value = ci.get(name)
            if not value:
                continue
            found = True
            lines.append(
                f"| {esc(row['target'])} | {label} | {value['n']} | {fmt(value['mean'])} | [{fmt(value['lower'])}, {fmt(value['upper'])}] |"
            )
    if not found:
        lines.append("| — | — | 0 | — | — |")
    return lines


def product_observations(bundle: dict[str, Any]) -> list[str]:
    by_target: dict[str, list[tuple[str, dict[str, Any]]]] = defaultdict(list)
    for suite_id, suite in bundle["suite_results"].items():
        for row in suite["results"]:
            by_target[row["target"]].append((suite_id, row))
    lines: list[str] = []
    for target in sorted(bundle.get("target_pins") or {}):
        raw_attempts = by_target.get(target, [])
        attempts = (
            [item for item in raw_attempts if comparable([item[1]])]
            if target == "elf"
            else raw_attempts
        )
        completed = [
            suite_id
            for suite_id, row in attempts
            if row["evaluation"]["classification"] == "completed"
        ]
        failed = [
            f"{suite_id}:{row['evaluation']['classification']}"
            for suite_id, row in attempts
            if row["evaluation"]["classification"] != "completed"
        ]
        measured: list[tuple[str, float]] = []
        for _, row in attempts:
            if not comparable([row]):
                continue
            for name in METRIC_NAMES:
                value = decision_metric(row, name)
                if isinstance(value, (int, float)) and name != "mean_query_latency_ms":
                    desirable = 1.0 - float(value) if name in LOWER_IS_BETTER else float(value)
                    measured.append((name, desirable))
        details: list[str] = []
        if measured:
            strongest = max(measured, key=lambda item: item[1])
            weakest = min(measured, key=lambda item: item[1])
            if strongest[1] > 0:
                details.append(
                    f"measured relative strength: {METRIC_NAMES[strongest[0]]} (directional value {fmt(strongest[1])})"
                )
            else:
                details.append("no positive measured quality strength")
            details.append(
                f"measured relative weakness: {METRIC_NAMES[weakest[0]]} (directional value {fmt(weakest[1])})"
            )
        if completed:
            details.insert(0, f"completed {', '.join(completed)}")
        if failed:
            details.append(f"failed or not comparable: {', '.join(failed)}")
        performance = []
        for suite_id, row in attempts:
            if not comparable([row]):
                continue
            performance.append(
                f"{suite_id} ingest {fmt(row['evaluation'].get('cold_ingest_duration_ms'), 1)} ms / warm {fmt(metrics(row).get('mean_query_latency_ms'), 1)} ms"
            )
            if (
                metrics(row).get("mean_recall_at_5") == 0
                and metrics(row).get("forbidden_or_stale_evidence_hit_rate") == 0
            ):
                details.append(
                    f"{suite_id} has zero stale hits and zero recall; this is not a stale-suppression strength"
                )
        if performance:
            details.append("performance: " + "; ".join(performance))
        if not attempts:
            details.append(
                "no comparable completed quality row; an unscored unit cannot support a measured strength or weakness"
                if raw_attempts
                else "retained in the manifest, but no suite applies in this run"
            )
        lines.append(f"- **{target}**: {'; '.join(details)}.")
    return lines


def job_desirability(job: dict[str, Any]) -> float | None:
    values: list[float] = []
    for name in (
        "recall_at_5",
        "ndcg_at_5",
        "source_trace_rate",
        "native_update_success",
        "native_delete_success",
    ):
        value = job.get(name)
        if isinstance(value, (int, float)):
            values.append(float(value))
    value = job.get("privacy_scope_violation")
    if isinstance(value, (int, float)):
        values.append(1.0 - float(value))
    if job.get("forbidden_evidence_hits") is not None:
        values.append(float(not bool(job["forbidden_evidence_hits"])))
    return sum(values) / len(values) if values else None


def elf_jobs(bundle: dict[str, Any]) -> list[tuple[str, dict[str, Any]]]:
    output: list[tuple[str, dict[str, Any]]] = []
    for suite_id, suite in bundle["suite_results"].items():
        row = next((item for item in suite["results"] if item["target"] == "elf"), None)
        if row is None or not comparable([row]):
            continue
        output.extend((suite_id, job) for job in warm(row).get("jobs") or [])
    return output


def elf_unscored_units(bundle: dict[str, Any]) -> list[tuple[str, str]]:
    output = []
    for suite_id, suite in bundle["suite_results"].items():
        row = next((item for item in suite["results"] if item["target"] == "elf"), None)
        if row is not None and not comparable([row]):
            output.append((suite_id, row["evaluation"]["classification"]))
    return output


def performance_regression_jobs(
    bundle: dict[str, Any], multiplier: float = 10.0
) -> set[str]:
    output: set[str] = set()
    for suite_id, suite in bundle["suite_results"].items():
        elf = next(
            (
                row
                for row in suite["results"]
                if row["target"] == "elf" and comparable([row])
            ),
            None,
        )
        competitors = [
            row
            for row in comparable(suite["results"])
            if row["target"] != "elf" and retrieval_effective(row)
        ]
        if elf is None or not competitors:
            continue
        competitor_latency: dict[str, list[float]] = defaultdict(list)
        for row in competitors:
            for job in warm(row).get("jobs") or []:
                value = job.get("latency_ms")
                if (
                    job.get("classification") == "completed"
                    and isinstance(value, (int, float))
                    and value > 0
                ):
                    competitor_latency[job["job_id"]].append(float(value))
        for job in warm(elf).get("jobs") or []:
            value = job.get("latency_ms")
            baselines = competitor_latency.get(job.get("job_id")) or []
            if (
                job.get("classification") == "completed"
                and isinstance(value, (int, float))
                and baselines
                and float(value) > multiplier * min(baselines)
            ):
                output.add(f"{suite_id}/{job['job_id']}")
    return output


def elf_job_summary(bundle: dict[str, Any]) -> list[str]:
    measured = [
        (suite, job, job_desirability(job))
        for suite, job in elf_jobs(bundle)
        if job.get("classification") == "completed" and job_desirability(job) is not None
    ]
    failed = [
        f"{suite}/* ({classification}; whole unit unscored)"
        for suite, classification in elf_unscored_units(bundle)
    ]
    lines: list[str] = []
    if measured:
        ordered = sorted(measured, key=lambda item: (item[2], item[0], item[1]["job_id"]))
        weakest = ordered[: min(5, len(ordered))]
        strongest = list(reversed(ordered[-min(5, len(ordered)) :]))
        lines.append(
            "- Strongest scenarios: "
            + ", ".join(
                f"`{suite}/{job['job_id']}` ({fmt(score)})"
                for suite, job, score in strongest
            )
            + "."
        )
        lines.append(
            "- Weakest scenarios: "
            + ", ".join(
                f"`{suite}/{job['job_id']}` ({fmt(score)})"
                for suite, job, score in weakest
            )
            + "."
        )
    if failed:
        lines.append("- Incomplete scenarios: " + ", ".join(failed) + ".")
    if not lines:
        lines.append("- ELF did not enter a scored denominator, so no strength claim is possible.")
    return lines


def roadmap(bundle: dict[str, Any]) -> list[str]:
    categories: dict[str, set[str]] = defaultdict(set)
    for suite_id, job in elf_jobs(bundle):
        identity = f"{suite_id}/{job['job_id']}"
        if isinstance(job.get("recall_at_5"), (int, float)) and job["recall_at_5"] < 1:
            categories["retrieval"].add(identity)
        if job.get("forbidden_evidence_hits"):
            categories["stale"].add(identity)
        if job.get("privacy_scope_violation") == 1:
            categories["privacy"].add(identity)
        if isinstance(job.get("source_trace_rate"), (int, float)) and job["source_trace_rate"] < 1:
            categories["trace"].add(identity)
        if job.get("native_update_success") == 0 or job.get("native_delete_success") == 0:
            categories["lifecycle"].add(identity)
    categories["performance"].update(performance_regression_jobs(bundle))

    definitions = [
        (
            "retrieval",
            "Improve evidence recall and ranking",
            "Candidate generation, chunking, or scope routing did not place expected evidence in the top five; confirm the cause from the trace.",
            "Tune candidate recall, chunking, and scope activation before reranking.",
            "Recall@5 and nDCG@5",
            "failed jobs",
            "Rerun the same frozen suite and jobs.",
        ),
        (
            "stale",
            "Strengthen stale-evidence suppression",
            "Current and stale evidence both reached the candidate set or final top five; confirm the filter boundary from hit traces.",
            "Apply supersession and current-state filters before final retrieval and retain an audit reason.",
            "Forbidden or stale evidence hit rate",
            "failed jobs",
            "Rerun the same frozen suite and jobs.",
        ),
        (
            "privacy",
            "Repair privacy-scope isolation",
            "A scope filter did not apply before retrieval; confirm the cause from the violating hit trace.",
            "Apply private scope as a hard pre-retrieval filter and record the rejection reason.",
            "Privacy-scope violation rate",
            "failed jobs",
            "Rerun the same frozen suite and jobs.",
        ),
        (
            "trace",
            "Complete stable source identity",
            "A native result did not map back to a stable source reference.",
            "Carry a non-inferred native source reference on every candidate and final evidence item.",
            "Source or citation trace rate",
            "failed jobs",
            "Rerun the same frozen suite and jobs.",
        ),
        (
            "performance",
            "Shorten the retrieval and ingest critical path",
            "ELF warm latency exceeded the fastest effective non-ELF row for the same job by more than 10 times.",
            "Use existing traces to separate embedding, storage, candidate generation, and rerank time, then optimize only the dominant stage.",
            "Warm query latency and cold ingest duration, with Recall@5, nDCG@5, and source trace as quality guardrails",
            "jobs above the 10× same-job latency threshold",
            "Rerun the same frozen suite and jobs; latency must return below the threshold without reducing a quality guardrail.",
        ),
        (
            "lifecycle",
            "Repair the native update or deletion loop",
            "A native mutation receipt or later visibility check failed.",
            "Repair update or deletion, asynchronous indexing, and read-after-write state handling.",
            "Native update success and native deletion success",
            "failed jobs",
            "Rerun the same frozen suite and jobs.",
        ),
    ]
    actions: list[str] = []
    for key, title, cause, change, expected, job_label, regression in definitions:
        jobs = sorted(categories.get(key) or [])
        if not jobs:
            continue
        identities = ", ".join(f"`{job}`" for job in jobs)
        actions.append(
            f"{len(actions) + 1}. **{title}** — {job_label} ({len(jobs)}): {identities}. "
            f"Possible cause: {cause} Suggested change: {change} Expected metrics: {expected}. Regression: {regression}"
        )
        if len(actions) == 5:
            break
    if not actions:
        actions.append(
            "1. This run produced no attributable ELF metric failure. Do not infer a product change; retain the benchmark as a regression baseline."
        )
    return actions


def native_contract_boundaries(bundle: dict[str, Any]) -> list[str]:
    lines = [
        "| Product | Type | Suite or field | Exact contract statement |",
        "|---|---|---|---|",
    ]
    found = False
    for target, contract in sorted((bundle.get("target_contracts") or {}).items()):
        for suite_id, reason in sorted((contract.get("not_applicable") or {}).items()):
            found = True
            lines.append(
                f"| {esc(target)} | not_applicable | {esc(suite_id)} | {esc(reason)} |"
            )
        for field, reason in sorted((contract.get("native_deviations") or {}).items()):
            found = True
            lines.append(
                f"| {esc(target)} | native_deviation | {esc(field)} | {esc(reason)} |"
            )
    if not found:
        lines.append("| — | — | — | No declared native-interface boundary |")
    return lines


def provider_usage(bundle: dict[str, Any]) -> Counter[str]:
    usage: Counter[str] = Counter()
    for suite in bundle["suite_results"].values():
        for row in suite["results"]:
            for provider_value in (row["evaluation"].get("provider_usage") or {}).values():
                if not isinstance(provider_value, dict):
                    continue
                for name, value in provider_value.items():
                    if isinstance(value, int):
                        usage[name] += value
    return usage


def quality_anchor(rows: list[dict[str, Any]]) -> list[str]:
    candidates = comparable(rows)
    for name in ("mean_recall_at_5", "mean_ndcg_at_5"):
        measured = [
            row
            for row in candidates
            if isinstance(decision_metric(row, name), (int, float))
        ]
        if not measured:
            continue
        best = max(float(decision_metric(row, name)) for row in measured)
        candidates = [
            row for row in measured if float(decision_metric(row, name)) == best
        ]
    return sorted(row["target"] for row in candidates)


def fastest_targets(
    rows: list[dict[str, Any]], field: str
) -> tuple[list[str], float | None]:
    measured: list[tuple[str, float]] = []
    for row in comparable(rows):
        value = (
            row["evaluation"].get("cold_ingest_duration_ms")
            if field == "ingest"
            else metrics(row).get("mean_query_latency_ms")
        )
        if isinstance(value, (int, float)):
            measured.append((row["target"], float(value)))
    if not measured:
        return [], None
    best = min(value for _, value in measured)
    return sorted(target for target, value in measured if value == best), best


def suite_performance_disposition(
    suite_id: str, rows: list[dict[str, Any]]
) -> str | None:
    elf = next(
        (row for row in rows if row["target"] == "elf" and comparable([row])), None
    )
    competitors = [
        row
        for row in comparable(rows)
        if row["target"] != "elf" and retrieval_effective(row)
    ]
    if elf is None or not competitors:
        return None
    ingest_values = [
        float(row["evaluation"]["cold_ingest_duration_ms"])
        for row in competitors
        if isinstance(row["evaluation"].get("cold_ingest_duration_ms"), (int, float))
        and row["evaluation"]["cold_ingest_duration_ms"] > 0
    ]
    latency_values = [
        float(metrics(row)["mean_query_latency_ms"])
        for row in competitors
        if isinstance(metrics(row).get("mean_query_latency_ms"), (int, float))
        and metrics(row)["mean_query_latency_ms"] > 0
    ]
    elf_ingest = elf["evaluation"].get("cold_ingest_duration_ms")
    elf_latency = metrics(elf).get("mean_query_latency_ms")
    details = []
    if isinstance(elf_ingest, (int, float)) and ingest_values:
        details.append(f"ingest is {float(elf_ingest) / min(ingest_values):.1f}× the fastest non-ELF row")
    if isinstance(elf_latency, (int, float)) and latency_values:
        details.append(f"warm is {float(elf_latency) / min(latency_values):.1f}× the fastest non-ELF row")
    return f"{suite_id}: " + ", ".join(details) if details else None


def five_decisions(bundle: dict[str, Any]) -> list[str]:
    suites = bundle.get("suite_results") or {}
    common_rows = (suites.get("common-core-v1") or {}).get("results") or []
    repository_rows = (suites.get("repository-knowledge-v1") or {}).get("results") or []
    memory_rows = (suites.get("memory-lifecycle-v1") or {}).get("results") or []
    knowledge_rows = (suites.get("knowledge-structure-v1") or {}).get("results") or []

    source_parts = []
    common_elf = next(
        (row for row in common_rows if row["target"] == "elf" and comparable([row])), None
    )
    if common_elf is not None:
        source_parts.append(
            "Common Core ELF Recall@5/nDCG@5/source trace is "
            f"{fmt(metrics(common_elf).get('mean_recall_at_5'))}/"
            f"{fmt(metrics(common_elf).get('mean_ndcg_at_5'))}/"
            f"{fmt(metrics(common_elf).get('source_or_citation_trace_rate'))}"
        )
    non_elf_anchor = quality_anchor(
        [row for row in common_rows if row["target"] != "elf"]
    )
    if non_elf_anchor:
        source_parts.append(
            "the non-ELF quality anchor (Recall first, then nDCG) is "
            + ", ".join(non_elf_anchor)
        )
    repository_elf = next(
        (row for row in repository_rows if row["target"] == "elf" and comparable([row])),
        None,
    )
    if repository_elf is not None:
        source_parts.append(
            "Repository Knowledge ELF Recall@5/nDCG@5 is "
            f"{fmt(metrics(repository_elf).get('mean_recall_at_5'))}/"
            f"{fmt(metrics(repository_elf).get('mean_ndcg_at_5'))}"
        )

    lifecycle_parts = []
    for row in comparable(memory_rows):
        value = metrics(row)
        lifecycle_parts.append(
            f"{row['target']} update/delete/stale/privacy="
            f"{fmt(value.get('native_correction_and_update_success'))}/"
            f"{fmt(value.get('native_deletion_or_forgetting_success'))}/"
            f"{fmt(value.get('forbidden_or_stale_evidence_hit_rate'))}/"
            f"{fmt(value.get('privacy_scope_violation_rate'))}, warm {fmt(value.get('mean_query_latency_ms'), 1)} ms"
        )

    trace_parts = []
    knowledge_anchor = quality_anchor(knowledge_rows)
    if knowledge_anchor:
        trace_parts.append("Knowledge Structure quality anchor: " + ", ".join(knowledge_anchor))
    knowledge_elf = next(
        (row for row in knowledge_rows if row["target"] == "elf" and comparable([row])), None
    )
    if knowledge_elf is not None:
        value = metrics(knowledge_elf)
        trace_parts.append(
            "ELF source trace="
            f"{fmt(value.get('source_or_citation_trace_rate'))}; "
            "shared answer correct/unsupported="
            f"{fmt(value.get('programmatic_answer_correctness'))}/"
            f"{fmt(value.get('unsupported_answer_rate'))} "
            "(end-to-end observation, not product or roadmap attribution)"
        )

    capability_parts = []
    performance_parts = []
    for suite_id, suite in suites.items():
        rows = suite.get("results") or []
        quality = quality_anchor(rows)
        ingest_targets, ingest_value = fastest_targets(rows, "ingest")
        warm_targets, warm_value = fastest_targets(rows, "warm")
        capability_parts.append(
            f"{suite_id}: quality anchor={', '.join(quality) or 'none'}, "
            f"fastest ingest={', '.join(ingest_targets) or 'none'} ({fmt(ingest_value, 1)} ms), "
            f"fastest warm={', '.join(warm_targets) or 'none'} ({fmt(warm_value, 1)} ms)"
        )
        disposition = suite_performance_disposition(suite_id, rows)
        if disposition:
            performance_parts.append(disposition)

    action_titles = [
        line.split("**", 2)[1] for line in roadmap(bundle) if line.count("**") >= 2
    ]
    return [
        "1. **Source-linked retrieval comparison** — "
        + ("; ".join(source_parts) or "no comparable completed source-linked row")
        + ".",
        "2. **Temporal state, correction, deletion, scope, and continuity** — "
        + ("; ".join(lifecycle_parts) or "no comparable completed lifecycle row")
        + ".",
        "3. **Source, citation, refusal, and organization reliability** — "
        + ("; ".join(trace_parts) or "no comparable completed knowledge-structure row")
        + ".",
        "4. **Strongest evidence by capability and performance trade-off** — No aggregate score is used. The quality anchor compares Recall@5 first and nDCG@5 second. A performance leader must be read with the quality anchor; zero recall is not a quality strength. "
        + "; ".join(capability_parts)
        + (". ELF performance disposition: " + "; ".join(performance_parts) if performance_parts else "")
        + ".",
        "5. **What ELF should change first** — "
        + (" → ".join(action_titles) if action_titles else "no attributable product change")
        + "; the full jobs, causes, metrics, changes, and regression conditions are in the ELF Development Order section.",
    ]


def publish(bundle: dict[str, Any]) -> str:
    source = bundle.get("source") or {}
    routes = bundle.get("provider_routes") or {}
    preflight = bundle.get("provider_preflight") or {}
    acceptance = bundle.get("acceptance") or {}
    lines = [
        "# ELF Competitor Benchmark Report",
        "",
        "## Interpretation Boundaries",
        "",
        "This complete measured run is an internal ELF development decision tool. It is not a public leaderboard or superiority claim. Only comparable rows that used real self-hosted runtimes, real APIs, and ended as `completed` enter a quality denominator. Provider, product, adapter, harness, timeout, cleanup, and configuration failures keep their exact type and are not converted to zero scores.",
        "",
        f"- Run mode: `{bundle.get('mode')}`; acceptance passed: `{fmt(bool(acceptance.get('passed')))}`.",
        f"- Fixed source: `{source.get('head', 'unknown')}`; dirty=`{source.get('dirty')}`; content SHA-256=`{source.get('content_sha256', 'unknown')}`.",
        f"- Manifest SHA-256: `{bundle.get('manifest_sha256', 'unknown')}`; Docker Server: `{bundle.get('docker_server_version', 'unknown')}`.",
        f"- Chat: `{routes.get('chat_model')}` / reasoning `{routes.get('chat_reasoning_effort')}`; embedding: `{routes.get('embedding_model')}` / `{routes.get('embedding_dimensions')}` dimensions.",
        f"- Preflight: embedding `{(preflight.get('embedding') or {}).get('classification')}`; chat `{(preflight.get('chat') or {}).get('classification')}`.",
        "- Unsupported-answer rate, forbidden or stale evidence hit rate, and privacy-scope violation rate are lower-is-better. Other quality success rates are higher-is-better.",
        "- One target-blind Luna request produces the shared answers for each score-eligible unit. Answer correctness and unsupported-answer rate remain end-to-end observations. One sample and the query-to-qrel entailment boundary do not support native product attribution, so these fields do not declare product strengths, winners, ELF scenario strengths, or roadmap actions.",
        "",
        "## Decision Summary",
        "",
    ]
    for suite_id, suite in bundle.get("suite_results", {}).items():
        leaders = metric_leaders(
            suite["results"],
            (
                "mean_recall_at_5",
                "mean_ndcg_at_5",
                "source_or_citation_trace_rate",
                "programmatic_answer_correctness",
                "native_correction_and_update_success",
                "native_deletion_or_forgetting_success",
                "forbidden_or_stale_evidence_hit_rate",
                "privacy_scope_violation_rate",
            ),
        )
        if leaders:
            lines.append(f"- **{SUITE_NAMES.get(suite_id, suite_id)}**: " + "; ".join(leaders) + ".")
        else:
            lines.append(
                f"- **{SUITE_NAMES.get(suite_id, suite_id)}**: insufficient comparable completed rows; no leader is declared."
            )
    lines.extend(
        [
            "",
            "Conclusions are metric-specific. Conflicting metric leaders do not become one aggregate score or one global winner. Zero stale hits with zero recall do not prove stale suppression. Shared-answer metrics remain visible in tables and confidence intervals, but they do not support product attribution.",
            "",
            "## Five Product Decisions",
            "",
            *five_decisions(bundle),
        ]
    )
    for suite_id, suite in bundle.get("suite_results", {}).items():
        lines.extend(
            [
                "",
                f"## {SUITE_NAMES.get(suite_id, suite_id)}",
                "",
                "This numerical table contains only completed, score-eligible rows in the quality denominator. Coverage, failures, and native not-applicable boundaries remain in the dedicated tables below.",
                "",
                *result_table(suite["results"], suite_id),
            ]
        )
        if suite_id == "common-core-v1":
            lines.extend(
                [
                    "",
                    "### Job-level 95% confidence intervals",
                    "",
                    *common_ci(suite["results"]),
                ]
            )
    lines.extend(
        [
            "",
            "## Coverage and Failures",
            "",
            "Denominators are scheduled, completed, failed, directly proven not applicable, and actually scored. A capability-only native operation may complete, but `score_eligible=false` keeps it outside retrieval-quality denominators.",
            "",
            *coverage_table(bundle),
            "",
            "### Typed failures",
            "",
            *failure_table(bundle),
            "",
            "## Measured Product Observations",
            "",
            *product_observations(bundle),
            "",
            "## ELF Strongest and Weakest Scenarios",
            "",
            "The parenthesized value is the directional mean of metrics executed for one job. It locates ELF-internal strengths and weaknesses and is not a cross-product aggregate rank.",
            "",
            *elf_job_summary(bundle),
            "",
            "## ELF Development Order",
            "",
            *roadmap(bundle),
            "",
            "## Reproducibility and Limitations",
            "",
            "- Each `{suite,target}` uses an isolated Compose project. Scheduler capacity is two. Cleanup results are in the coverage table.",
            "- Shared answers must contain non-empty fact text or exact `unknown`. The raw chat response stays in the corresponding raw unit, then the central scorer applies deterministic fact checks.",
            "- Shared answers are sampled once. If a query does not entail every scorer-only answer fact, or the same native context can produce another answer, the value cannot prove a native product difference. The report keeps the observation but does not use it for a product winner, ELF scenario strength, or roadmap action.",
            f"- Image digests: `{json.dumps(bundle.get('target_image_digests') or {}, sort_keys=True)}`.",
            f"- Product pins: `{json.dumps(bundle.get('target_pins') or {}, sort_keys=True)}`.",
            "- Common Core uses job-level normal-approximation 95% confidence intervals. The frozen suite is internal descriptive evidence only.",
            "- The table below reproduces manifest not-applicable reasons and native deviations verbatim. Adapters do not disguise different native semantics as one CRUD contract.",
            "",
            "### Native-interface boundaries",
            "",
            *native_contract_boundaries(bundle),
        ]
    )
    usage = provider_usage(bundle)
    if usage:
        lines.append(
            f"- Provider-returned shared-answer usage total: `{json.dumps(dict(usage), sort_keys=True)}`."
        )
    else:
        lines.append("- The provider returned no aggregate token or request usage; cost is not inferred.")
    if acceptance.get("findings"):
        lines.append("- Unmet acceptance items: " + "; ".join(map(str, acceptance["findings"])) + ".")
    return "\n".join(lines) + "\n"


def main() -> int:
    args = parse_args()
    bundle = json.loads(args.bundle.read_text(encoding="utf-8"))
    if bundle.get("schema") != "elf.benchmark_bundle/v1":
        raise ValueError("input is not an ELF benchmark bundle")
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(publish(bundle), encoding="utf-8")
    print(args.out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
