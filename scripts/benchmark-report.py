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
            (row["target"], metrics(row).get(name))
            for row in completed
            if metrics(row).get(name) is not None
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
        attempts = by_target.get(target, [])
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
            for name, value in metrics(row).items():
                if name in METRIC_NAMES and isinstance(value, (int, float)) and name != "mean_query_latency_ms":
                    desirable = 1.0 - float(value) if name in LOWER_IS_BETTER else float(value)
                    measured.append((name, desirable))
        details: list[str] = []
        if measured:
            strongest = max(measured, key=lambda item: item[1])
            weakest = min(measured, key=lambda item: item[1])
            details.append(
                f"实测相对强项为 {METRIC_NAMES[strongest[0]]}（方向化值 {fmt(strongest[1])}）"
            )
            details.append(
                f"实测相对弱项为 {METRIC_NAMES[weakest[0]]}（方向化值 {fmt(weakest[1])}）"
            )
        if completed:
            details.insert(0, f"完成 {', '.join(completed)}")
        if failed:
            details.append(f"失败或不可比：{', '.join(failed)}")
        if not attempts:
            details.append("清单中保留，但本次没有适用 suite")
        lines.append(f"- **{target}**：{'；'.join(details)}。")
    return lines


def job_desirability(job: dict[str, Any]) -> float | None:
    values: list[float] = []
    for name in (
        "recall_at_5",
        "ndcg_at_5",
        "answer_correct",
        "source_trace_rate",
        "native_update_success",
        "native_delete_success",
    ):
        value = job.get(name)
        if isinstance(value, (int, float)):
            values.append(float(value))
    for name in ("privacy_scope_violation", "unsupported_answer_error"):
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
        if job.get("forbidden_evidence_hits") or job.get("privacy_scope_violation") == 1:
            categories["stale_scope"].add(identity)
        if isinstance(job.get("source_trace_rate"), (int, float)) and job["source_trace_rate"] < 1:
            categories["trace"].add(identity)
        if job.get("answer_correct") == 0 or job.get("unsupported_answer_error") == 1:
            categories["answer"].add(identity)
        if job.get("native_update_success") == 0 or job.get("native_delete_success") == 0:
            categories["lifecycle"].add(identity)

    definitions = [
        (
            "runtime",
            "先消除 ELF 原生运行失败",
            "运行或配置边界在检索前失败；需用 raw trace 定位，当前只作为待证假设。",
            "修复具体失败边界，不改变 suite 或评分器。",
            "完成任务数与 typed failure 数",
        ),
        (
            "retrieval",
            "提升证据召回与排序",
            "候选生成、分块或 scope routing 未把期望证据带入 top-5；需从 trace 验证。",
            "优先调候选召回、分块和范围激活，再评估 rerank。",
            "Recall@5、nDCG@5",
        ),
        (
            "stale_scope",
            "强化陈旧抑制与隐私范围",
            "生命周期状态或 scope filter 未在检索前稳定生效；需从命中证据验证。",
            "把 supersession、delete 与 scope 状态作为硬过滤，并保留审计原因。",
            "禁用/陈旧证据命中率、隐私越界率",
        ),
        (
            "trace",
            "补齐稳定来源身份",
            "原生结果没有全部映射回稳定 source_ref。",
            "让每个候选和最终证据都携带不可推断的原生 source_ref。",
            "来源可追溯率",
        ),
        (
            "answer",
            "收紧证据绑定回答与拒答",
            "共享回答器看到的检索上下文缺少关键事实或包含冲突事实。",
            "在回答前校验来源充分性、冲突和 forbidden 状态；不足时明确拒答。",
            "答案正确率、无依据仍作答率",
        ),
        (
            "lifecycle",
            "修复原生更新或删除闭环",
            "原生 mutation receipt 或后续可见性检查失败。",
            "修复 update/delete、异步索引与 read-after-write 状态闭环。",
            "原生更新成功率、原生删除成功率",
        ),
    ]
    actions: list[str] = []
    for key, title, cause, change, expected in definitions:
        jobs = sorted(categories.get(key) or [])
        if not jobs:
            continue
        regression = ", ".join(f"`{job}`" for job in jobs[:8])
        actions.append(
            f"{len(actions) + 1}. **{title}** — 失败 jobs：{regression}。可能原因：{cause} 建议变更：{change} 期望指标：{expected}。回归：固定复跑这些相同 suite/job。"
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
