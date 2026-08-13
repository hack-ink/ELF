#!/usr/bin/env python3
"""Publish the single Chinese decision report from one measured result bundle."""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable


SUITE_NAMES = {
    "common-core-v1": "Common Core（24 个任务）",
    "memory-lifecycle-v1": "Memory Lifecycle（8 个任务）",
    "knowledge-structure-v1": "Knowledge Structure（8 个任务）",
    "repository-knowledge-v1": "Repository Knowledge（8 个任务）",
}
METRIC_NAMES = {
    "mean_recall_at_5": "Recall@5",
    "mean_ndcg_at_5": "nDCG@5",
    "forbidden_or_stale_evidence_hit_rate": "禁用/陈旧证据命中率",
    "source_or_citation_trace_rate": "来源可追溯率",
    "programmatic_answer_correctness": "答案正确率",
    "native_correction_and_update_success": "原生更新成功率",
    "native_deletion_or_forgetting_success": "原生删除成功率",
    "privacy_scope_violation_rate": "隐私越界率",
    "unsupported_answer_rate": "无依据仍作答率",
    "mean_query_latency_ms": "warm 查询延迟（ms）",
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


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    return parser.parse_args()


def fmt(value: Any, digits: int = 3) -> str:
    if value is None:
        return "N/A"
    if isinstance(value, bool):
        return "是" if value else "否"
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


def result_table(rows: list[dict[str, Any]]) -> list[str]:
    lines = [
        "| 产品 | 结果 | 计划/完成/失败/N/A/计分 | Recall@5 | nDCG@5 | 陈旧命中↓ | 来源追溯 | 答案正确 | 更新 | 删除 | 隐私越界↓ | 无依据作答↓ | ingest ms | warm ms |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for row in rows:
        evaluation = row["evaluation"]
        phase = warm(row)
        value = metrics(row)
        lines.append(
            "| {target} | {classification} | {counts} | {recall} | {ndcg} | {stale} | {trace} | {answer} | {update} | {delete} | {privacy} | {unsupported} | {ingest} | {latency} |".format(
                target=esc(row["target"]),
                classification=esc(evaluation["classification"]),
                counts=count_text(phase),
                recall=fmt(value.get("mean_recall_at_5")),
                ndcg=fmt(value.get("mean_ndcg_at_5")),
                stale=fmt(value.get("forbidden_or_stale_evidence_hit_rate")),
                trace=fmt(value.get("source_or_citation_trace_rate")),
                answer=fmt(value.get("programmatic_answer_correctness")),
                update=fmt(value.get("native_correction_and_update_success")),
                delete=fmt(value.get("native_deletion_or_forgetting_success")),
                privacy=fmt(value.get("privacy_scope_violation_rate")),
                unsupported=fmt(value.get("unsupported_answer_rate")),
                ingest=fmt(evaluation.get("cold_ingest_duration_ms"), 1),
                latency=fmt(value.get("mean_query_latency_ms"), 1),
            )
        )
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
        output.append(f"{METRIC_NAMES[name]}：{', '.join(leaders)}（{fmt(best)}）")
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
    return "未提供额外错误信息"


def coverage_table(bundle: dict[str, Any]) -> list[str]:
    lines = [
        "| Suite | 产品 | 结果 | 计划 | 完成 | 失败 | N/A | 计分 | cleanup |",
        "|---|---|---|---:|---:|---:|---:|---:|---|",
    ]
    for suite_id, suite in bundle["suite_results"].items():
        for row in suite["results"]:
            counts = warm(row)["counts"]
            lines.append(
                "| {suite} | {target} | {result} | {scheduled} | {completed} | {failed} | {na} | {scored} | {cleanup} |".format(
                    suite=esc(suite_id),
                    target=esc(row["target"]),
                    result=esc(row["evaluation"]["classification"]),
                    scheduled=counts["scheduled"],
                    completed=counts["completed"],
                    failed=counts["failed"],
                    na=counts["not_applicable"],
                    scored=counts["scored"],
                    cleanup=fmt(row["cleanup"].get("passed")),
                )
            )
    return lines


def failure_table(bundle: dict[str, Any]) -> list[str]:
    lines = [
        "| Suite | 产品 | 类型 | 说明 |",
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
        lines.append("| — | — | — | 没有 typed failure |")
    return lines


def common_ci(rows: list[dict[str, Any]]) -> list[str]:
    lines = [
        "| 产品 | 指标 | n | 均值 | 95% CI |",
        "|---|---|---:|---:|---:|",
    ]
    found = False
    for row in comparable(rows):
        ci = metrics(row).get("job_level_95ci") or {}
        for name, label in (
            ("recall_at_5", "Recall@5"),
            ("ndcg_at_5", "nDCG@5"),
            ("answer_correctness", "答案正确率"),
        ):
            value = ci.get(name)
            if not value:
                continue
            found = True
            lines.append(
                f"| {esc(row['target'])} | {label} | {value['n']} | {fmt(value['mean'])} | [{fmt(value['lower'])}, {fmt(value['upper'])}] |"
            )
    if not found:
        lines.append("| — | — | 0 | N/A | N/A |")
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
        completed = [suite_id for suite_id, row in attempts if row["evaluation"]["classification"] == "completed"]
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
                    f"实测相对强项为 {METRIC_NAMES[strongest[0]]}（方向化值 {fmt(strongest[1])}）"
                )
            else:
                details.append("没有测得正向质量强项")
            details.append(
                f"实测相对弱项为 {METRIC_NAMES[weakest[0]]}（方向化值 {fmt(weakest[1])}）"
            )
        if completed:
            details.insert(0, f"完成 {', '.join(completed)}")
        if failed:
            details.append(f"失败或不可比：{', '.join(failed)}")
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
                    f"{suite_id} 的零陈旧命中与零召回同时出现，不构成陈旧抑制强项"
                )
        if performance:
            details.append("性能：" + "；".join(performance))
        if not attempts:
            details.append(
                "没有可比 completed 质量行，不能从未计分单元形成实测强弱结论"
                if raw_attempts
                else "清单中保留，但本次没有适用 suite"
            )
        lines.append(f"- **{target}**：{'；'.join(details)}。")
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
    for name in ("privacy_scope_violation",):
        value = job.get(name)
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
        f"{suite}/*（{classification}，整个单元不计分）"
        for suite, classification in elf_unscored_units(bundle)
    ]
    lines: list[str] = []
    if measured:
        ordered = sorted(measured, key=lambda item: (item[2], item[0], item[1]["job_id"]))
        weakest = ordered[: min(5, len(ordered))]
        strongest = list(reversed(ordered[-min(5, len(ordered)) :]))
        lines.append(
            "- 最强场景："
            + "、".join(
                f"`{suite}/{job['job_id']}`（{fmt(score)}）"
                for suite, job, score in strongest
            )
            + "。"
        )
        lines.append(
            "- 最弱场景："
            + "、".join(
                f"`{suite}/{job['job_id']}`（{fmt(score)}）"
                for suite, job, score in weakest
            )
            + "。"
        )
    if failed:
        lines.append("- 未完成场景：" + "、".join(failed) + "。")
    if not lines:
        lines.append("- ELF 没有进入可计分分母，不能形成强弱结论。")
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
            "runtime",
            "先消除 ELF 原生运行失败",
            "运行或配置边界在检索前失败；需用 raw trace 定位，当前只作为待证假设。",
            "修复具体失败边界，不改变 suite 或评分器。",
            "完成任务数与 typed failure 数",
            "失败 jobs",
            "固定复跑这些相同 suite/job。",
        ),
        (
            "retrieval",
            "提升证据召回与排序",
            "候选生成、分块或 scope routing 未把期望证据带入 top-5；需从 trace 验证。",
            "优先调候选召回、分块和范围激活，再评估 rerank。",
            "Recall@5、nDCG@5",
            "失败 jobs",
            "固定复跑这些相同 suite/job。",
        ),
        (
            "stale",
            "强化陈旧证据抑制",
            "当前与陈旧证据同时进入候选或最终 top-5；需从命中 trace 验证过滤边界。",
            "把 supersession 与当前状态作为检索硬过滤，并保留审计原因。",
            "禁用/陈旧证据命中率",
            "失败 jobs",
            "固定复跑这些相同 suite/job。",
        ),
        (
            "privacy",
            "修复隐私范围隔离",
            "scope filter 未在检索前稳定生效；需从越界命中 trace 验证。",
            "把私有范围作为候选生成前的硬过滤，并记录拒绝原因。",
            "隐私越界率",
            "失败 jobs",
            "固定复跑这些相同 suite/job。",
        ),
        (
            "trace",
            "补齐稳定来源身份",
            "原生结果没有全部映射回稳定 source_ref。",
            "让每个候选和最终证据都携带不可推断的原生 source_ref。",
            "来源可追溯率",
            "失败 jobs",
            "固定复跑这些相同 suite/job。",
        ),
        (
            "performance",
            "缩短检索与摄取关键路径",
            "这些 job 的 ELF warm 延迟超过同 job 最快非 ELF 完成行的 10 倍；聚合表还显示 cold ingest 存在数量级差距。",
            "先用现有 trace 分解 embedding、存储、候选生成与 rerank 时间，再只优化主导阶段。",
            "warm 查询延迟、cold ingest duration；Recall@5、nDCG@5 与来源追溯率作为质量护栏",
            "触发 10× 同 job 延迟阈值的 jobs",
            "固定复跑这些相同 suite/job；warm 延迟不再超过本次最快非 ELF 基线 10 倍，且质量护栏不得下降。",
        ),
        (
            "lifecycle",
            "修复原生更新或删除闭环",
            "原生 mutation receipt 或后续可见性检查失败。",
            "修复 update/delete、异步索引与 read-after-write 状态闭环。",
            "原生更新成功率、原生删除成功率",
            "失败 jobs",
            "固定复跑这些相同 suite/job。",
        ),
    ]
    actions: list[str] = []
    for key, title, cause, change, expected, job_label, regression_text in definitions:
        jobs = sorted(categories.get(key) or [])
        if not jobs:
            continue
        regression = ", ".join(f"`{job}`" for job in jobs)
        actions.append(
            f"{len(actions) + 1}. **{title}** — {job_label}（{len(jobs)}）：{regression}。可能原因：{cause} 建议变更：{change} 期望指标：{expected}。回归：{regression_text}"
        )
        if len(actions) == 5:
            break
    if not actions:
        actions.append("1. 本次 ELF 没有产生可归因的指标失败；不据此提出产品改动。先保留本 benchmark 作为回归基线。")
    return actions


def native_contract_boundaries(bundle: dict[str, Any]) -> list[str]:
    lines = [
        "| 产品 | 类型 | suite/字段 | 精确合同说明 |",
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
        lines.append("| — | — | — | 没有声明的原生接口边界 |")
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
        details.append(f"ingest 为最快非 ELF 的 {float(elf_ingest) / min(ingest_values):.1f}×")
    if isinstance(elf_latency, (int, float)) and latency_values:
        details.append(f"warm 为最快非 ELF 的 {float(elf_latency) / min(latency_values):.1f}×")
    if not details:
        return None
    return f"{suite_id}：" + "，".join(details)


def five_decisions(bundle: dict[str, Any]) -> list[str]:
    suites = bundle.get("suite_results") or {}
    common_rows = (suites.get("common-core-v1") or {}).get("results") or []
    repository_rows = (suites.get("repository-knowledge-v1") or {}).get("results") or []
    memory_rows = (suites.get("memory-lifecycle-v1") or {}).get("results") or []
    knowledge_rows = (suites.get("knowledge-structure-v1") or {}).get("results") or []

    common_elf = next(
        (row for row in common_rows if row["target"] == "elf" and comparable([row])),
        None,
    )
    non_elf_anchor = quality_anchor(
        [row for row in common_rows if row["target"] != "elf"]
    )
    source_parts = []
    if common_elf is not None:
        source_parts.append(
            "Common Core 的 ELF Recall@5/nDCG@5/来源追溯为 "
            f"{fmt(metrics(common_elf).get('mean_recall_at_5'))}/"
            f"{fmt(metrics(common_elf).get('mean_ndcg_at_5'))}/"
            f"{fmt(metrics(common_elf).get('source_or_citation_trace_rate'))}"
        )
    if non_elf_anchor:
        source_parts.append(
            "非 ELF 质量锚点（先 Recall、再 nDCG）为 " + ", ".join(non_elf_anchor)
        )
    repository_elf = next(
        (
            row
            for row in repository_rows
            if row["target"] == "elf" and comparable([row])
        ),
        None,
    )
    if repository_elf is not None:
        source_parts.append(
            "Repository Knowledge 的 ELF Recall@5/nDCG@5 为 "
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
            f"{fmt(value.get('privacy_scope_violation_rate'))}，warm {fmt(value.get('mean_query_latency_ms'), 1)} ms"
        )

    knowledge_anchor = quality_anchor(knowledge_rows)
    trace_parts = []
    if knowledge_anchor:
        trace_parts.append("Knowledge Structure 质量锚点为 " + ", ".join(knowledge_anchor))
    knowledge_elf = next(
        (row for row in knowledge_rows if row["target"] == "elf" and comparable([row])),
        None,
    )
    if knowledge_elf is not None:
        value = metrics(knowledge_elf)
        trace_parts.append(
            "ELF 来源追溯率="
            f"{fmt(value.get('source_or_citation_trace_rate'))}；"
            "共享回答正确率/无依据作答率="
            f"{fmt(value.get('programmatic_answer_correctness'))}/"
            f"{fmt(value.get('unsupported_answer_rate'))}"
            "（端到端观测，不用于产品强弱或路线图归因）"
        )

    capability_parts = []
    performance_parts = []
    for suite_id, suite in suites.items():
        rows = suite.get("results") or []
        quality = quality_anchor(rows)
        ingest_targets, ingest_value = fastest_targets(rows, "ingest")
        warm_targets, warm_value = fastest_targets(rows, "warm")
        capability_parts.append(
            f"{suite_id} 质量锚点={', '.join(quality) or '无'}；"
            f"ingest 最快={', '.join(ingest_targets) or '无'}({fmt(ingest_value, 1)} ms)；"
            f"warm 最快={', '.join(warm_targets) or '无'}({fmt(warm_value, 1)} ms)"
        )
        disposition = suite_performance_disposition(suite_id, rows)
        if disposition:
            performance_parts.append(disposition)

    actions = roadmap(bundle)
    action_titles = [
        line.split("**", 2)[1] for line in actions if line.count("**") >= 2
    ]
    return [
        "1. **来源支持的检索比较** — " + "；".join(source_parts) + "。",
        "2. **时序、纠正、删除、范围与连续性** — "
        + ("；".join(lifecycle_parts) or "没有足够的 completed 生命周期行")
        + "。",
        "3. **来源、引用、拒答与组织可靠性** — "
        + ("；".join(trace_parts) or "没有足够的 completed 知识结构行")
        + "。",
        "4. **各能力最强证据与性能权衡** — 不合成总分；质量锚点只先比较 Recall@5、再比较 nDCG@5；性能最快项必须与同段质量锚点一起读，零召回不算质量强项。"
        + "；".join(capability_parts)
        + ("。ELF 性能处置：" + "；".join(performance_parts) if performance_parts else "")
        + "。",
        "5. **ELF 先改什么** — "
        + (" → ".join(action_titles) if action_titles else "没有可归因的产品改动")
        + "；完整失败 job、原因、变更、指标和回归条件见“ELF 优化顺序”。",
    ]


def publish(bundle: dict[str, Any]) -> str:
    source = bundle.get("source") or {}
    routes = bundle.get("provider_routes") or {}
    preflight = bundle.get("provider_preflight") or {}
    acceptance = bundle.get("acceptance") or {}
    lines = [
        "# ELF 竞品基准报告",
        "",
        "## 结论边界",
        "",
        "这是用于 ELF 内部研发排序的完整实测报告，不是公开 leaderboard 或优越性声明。只有真实自托管运行、真实 API 调用且结果为 `completed` 的可比行进入质量分母；任何 provider、product、adapter、harness、timeout、cleanup 或 configuration failure 都保留原类型，不按零分处理。",
        "",
        f"- 运行模式：`{bundle.get('mode')}`；验收：`{'通过' if acceptance.get('passed') else '未通过'}`。",
        f"- 固定源码：`{source.get('head', 'unknown')}`；dirty=`{source.get('dirty')}`；content SHA-256=`{source.get('content_sha256', 'unknown')}`。",
        f"- Manifest SHA-256：`{bundle.get('manifest_sha256', 'unknown')}`；Docker Server：`{bundle.get('docker_server_version', 'unknown')}`。",
        f"- Chat：`{routes.get('chat_model')}` / reasoning `{routes.get('chat_reasoning_effort')}`；embedding：`{routes.get('embedding_model')}` / `{routes.get('embedding_dimensions')}` 维。",
        f"- Preflight：embedding `{(preflight.get('embedding') or {}).get('classification')}`，chat `{(preflight.get('chat') or {}).get('classification')}`。",
        "- `无依据仍作答率`、`禁用/陈旧证据命中率` 与 `隐私越界率` 越低越好；其余质量成功率越高越好。",
        "- 共享回答由每个可计分单元的一次 target-blind Luna 调用生成。答案正确率与无依据作答率保留为端到端观测，但单次生成差异和 query/qrel 蕴含边界不能可靠归因给原生产品；因此二者不用于宣布产品强项、赢家、ELF 场景强弱或产品路线图。",
        "",
        "## 决策摘要",
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
            lines.append(
                f"- **{SUITE_NAMES.get(suite_id, suite_id)}**：" + "；".join(leaders) + "。"
            )
        else:
            lines.append(
                f"- **{SUITE_NAMES.get(suite_id, suite_id)}**：没有足够的可比 completed 行，不能宣布强者。"
            )
    lines.extend(
        [
            "",
            "这些结论按具体指标报告；指标领跑者不一致时不合成全局分数，也不制造单一赢家。",
            "零召回时的零陈旧命中只表示没有检索结果，不作为陈旧抑制强项。",
            "共享回答指标仍在数值表和置信区间中完整呈现，但只作为端到端观测，不参与产品归因。",
            "",
            "## 五项产品决策",
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
                *result_table(suite["results"]),
            ]
        )
        if suite_id == "common-core-v1":
            lines.extend(["", "### Job-level 95% 置信区间", "", *common_ci(suite["results"])])
    lines.extend(
        [
            "",
            "## 覆盖与失败",
            "",
            "分母列依次为计划、完成、失败、not-applicable、实际计分；capability-only 的真实操作可以 completed，但 `score_eligible=false` 时不会进入检索质量分母。",
            "",
            *coverage_table(bundle),
            "",
            "### Typed failures",
            "",
            *failure_table(bundle),
            "",
            "## 逐产品实测强弱",
            "",
            *product_observations(bundle),
            "",
            "## ELF 最强与最弱场景",
            "",
            "括号为同一 job 内已执行指标的方向化均值，仅用于定位 ELF 内部强弱，不用于跨产品总排名。",
            "",
            *elf_job_summary(bundle),
            "",
            "## ELF 优化顺序",
            "",
            *roadmap(bundle),
            "",
            "## 可复现性与限制",
            "",
            "- 每个 `{suite,target}` 使用独立 Compose project；并发上限为 2；cleanup 结果见覆盖表。",
            "- 共享回答必须返回非空事实文本或精确的 `unknown`；原始 chat response 保存在对应 raw unit 中，评分器再做确定性事实校验。",
            "- 共享回答只采样一次。若查询不蕴含全部 scorer-only answer facts，或相同 native context 产生不同回答，该结果不能证明原生产品差异；报告保留原值，但不据此形成产品赢家、ELF 强弱场景或路线图动作。",
            f"- Image digests：`{json.dumps(bundle.get('target_image_digests') or {}, sort_keys=True)}`。",
            f"- Product pins：`{json.dumps(bundle.get('target_pins') or {}, sort_keys=True)}`。",
            "- Common Core 使用 job-level 正态近似 95% CI；样本只代表冻结 suite，是内部描述性证据。",
            "- 下表逐字呈现 manifest 中的 not-applicable 理由与 native deviation；不同原生语义不被适配器伪装成同一种 CRUD。",
            "",
            "### 原生接口边界",
            "",
            *native_contract_boundaries(bundle),
        ]
    )
    usage = provider_usage(bundle)
    if usage:
        lines.append(f"- Provider 返回的共享回答 usage 汇总：`{json.dumps(dict(usage), sort_keys=True)}`。")
    else:
        lines.append("- Provider 未返回可汇总的 token/request usage；不推算成本。")
    if acceptance.get("findings"):
        lines.append("- 未满足验收项：" + "；".join(map(str, acceptance["findings"])) + "。")
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
