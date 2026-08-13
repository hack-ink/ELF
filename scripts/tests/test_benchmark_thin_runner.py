#!/usr/bin/env python3
"""Focused contract tests for the single competitor benchmark path."""

from __future__ import annotations

import copy
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

from benchmark_contract import (  # noqa: E402
    FAILURE_CLASSES,
    FORBIDDEN_PRODUCT_KEYS,
    NativeContextError,
    SUITE_IDS,
    answer_cases,
    evaluate_unit,
    load_json,
    materialize_product_fixtures,
    opaque_evidence_id,
    opaque_job_id,
    sha256_json,
    validate_manifest,
    validate_suite,
)


def load_script(name: str, path: str):
    spec = importlib.util.spec_from_file_location(name, REPO / path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


RUNNER = load_script("benchmark_runner", "scripts/benchmark-runner.py")
PREFLIGHT = load_script(
    "benchmark_provider_preflight", "scripts/benchmark-provider-preflight.py"
)
REPORT = load_script("benchmark_report", "scripts/benchmark-report.py")
OPENKB = load_script("benchmark_openkb", "scripts/benchmark_targets/openkb.py")


class BenchmarkContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = load_json(REPO / "config/benchmark/benchmark-v3.json")
        cls.suites = {
            entry["id"]: load_json(REPO / entry["path"])
            for entry in cls.manifest["suites"]
        }
        cls.targets = {target["id"]: target for target in cls.manifest["targets"]}

    def subset(self, suite_id: str, count: int = 1) -> dict:
        suite = copy.deepcopy(self.suites[suite_id])
        suite["jobs"] = suite["jobs"][:count]
        suite["execution_mode"] = "readiness"
        return suite

    def completed_unit(self, target: str, suite: dict) -> dict:
        cold_jobs = []
        warm_jobs = []
        for job in suite["jobs"]:
            relevant = job["qrels"].get("relevant_evidence") or []
            retrieved = [opaque_evidence_id(value) for value in relevant]
            cold_text = {item["evidence_id"]: item["text"] for item in job["corpus"]}
            warm_text = dict(cold_text)
            for operation in job.get("operations") or []:
                if operation["type"] == "update":
                    warm_text[operation["evidence_id"]] = operation["text"]
                elif operation["type"] == "delete":
                    warm_text.pop(operation["evidence_id"], None)
            cold_contexts = [
                {"evidence_id": opaque_evidence_id(value), "text": cold_text[value]}
                for value in relevant
                if value in cold_text
            ]
            warm_contexts = [
                {"evidence_id": opaque_evidence_id(value), "text": warm_text[value]}
                for value in relevant
                if value in warm_text
            ]
            answer = {
                "text": " ".join(job["qrels"].get("answer_facts") or []) or "unknown",
                "supported": not bool(job["qrels"].get("expect_unsupported")),
            }
            cold_jobs.append(
                {
                    "job_id": opaque_job_id(job["job_id"]),
                    "classification": "completed",
                    "evidence_ids": retrieved,
                    "contexts": cold_contexts,
                    "returned_count": len(retrieved),
                    "latency_ms": 10.0,
                    "operations": [],
                }
            )
            operations = [
                {
                    "requested_type": operation,
                    "native_type": "update" if operation == "update" else "delete",
                    "classification": "completed",
                    "native_success": True,
                }
                for operation in job["qrels"].get("required_operations") or []
            ]
            warm_jobs.append(
                {
                    **cold_jobs[-1],
                    "contexts": warm_contexts,
                    "answer": answer,
                    "operations": operations,
                }
            )
        return {
            "schema": "elf.benchmark_unit_result/v4",
            "target": target,
            "native_mode": self.targets[target]["adapter"],
            "score_eligible": self.targets[target]["score_eligible"],
            "result_class": "completed",
            "warm_reused_state": True,
            "ingest_count": 1,
            "ingest_duration_ms": 100.0,
            "phases": {
                "cold": {"status": "completed", "jobs": cold_jobs},
                "warm": {"status": "completed", "jobs": warm_jobs},
            },
        }

    def test_manifest_and_exact_four_suites_validate(self) -> None:
        validate_manifest(self.manifest)
        self.assertEqual(set(self.suites), set(SUITE_IDS))
        self.assertEqual(len(self.manifest["targets"]), 12)
        self.assertEqual(self.manifest["runner"]["capacity"], 2)
        for suite_id, expected in {
            "common-core-v1": 24,
            "memory-lifecycle-v1": 8,
            "knowledge-structure-v1": 8,
            "repository-knowledge-v1": 8,
        }.items():
            validate_suite(self.suites[suite_id])
            self.assertEqual(len(self.suites[suite_id]["jobs"]), expected)

    def test_readiness_subset_keeps_canonical_suite_identity(self) -> None:
        suite = self.subset("memory-lifecycle-v1", 2)
        validate_suite(suite)
        self.assertEqual(suite["suite_id"], "memory-lifecycle-v1")
        self.assertEqual(len(suite["jobs"]), 2)

    def test_product_payload_is_opaque_and_preserves_native_operations(self) -> None:
        suite = self.subset("memory-lifecycle-v1")
        with tempfile.TemporaryDirectory() as directory:
            paths = materialize_product_fixtures(suite, Path(directory))
            self.assertEqual(len(paths), 1)
            payload = json.loads(paths[0].read_text(encoding="utf-8"))
        encoded = json.dumps(payload, sort_keys=True)
        for forbidden in FORBIDDEN_PRODUCT_KEYS:
            self.assertNotIn(f'"{forbidden}"', encoded)
        self.assertTrue(payload["job_id"].startswith("j_"))
        self.assertTrue(payload["corpus"]["items"][0]["evidence_id"].startswith("e_"))
        self.assertTrue(payload["operations"])
        self.assertTrue(payload["operations"][0]["evidence_id"].startswith("e_"))

    def test_elf_and_external_adapter_have_identical_normalized_shape(self) -> None:
        suite = self.subset("common-core-v1", 2)
        elf = evaluate_unit(suite, self.completed_unit("elf", suite), self.targets["elf"])
        mem0 = evaluate_unit(
            suite, self.completed_unit("mem0", suite), self.targets["mem0"]
        )

        def shape(value):
            if isinstance(value, dict):
                return {key: shape(child) for key, child in value.items()}
            if isinstance(value, list):
                return [shape(value[0])] if value else []
            return type(value).__name__

        self.assertEqual(shape(elf), shape(mem0))
        self.assertEqual(elf["schema"], "elf.benchmark_result/v1")

    def test_every_typed_failure_is_preserved_and_unscored(self) -> None:
        suite = self.subset("common-core-v1")
        target = self.targets["elf"]
        for classification in FAILURE_CLASSES - {"completed"}:
            with self.subTest(classification=classification):
                unit = RUNNER.failure_unit(target, suite, classification, "typed failure")
                result = evaluate_unit(suite, unit, target)
                self.assertEqual(result["classification"], classification)
                self.assertFalse(result["phases"]["warm"]["quality_denominator"])
                self.assertEqual(result["phases"]["warm"]["counts"]["scored"], 0)

    def test_missing_native_operation_becomes_adapter_failure(self) -> None:
        suite = self.subset("memory-lifecycle-v1")
        unit = self.completed_unit("elf", suite)
        unit["phases"]["warm"]["jobs"][0]["operations"] = []
        result = evaluate_unit(suite, unit, self.targets["elf"])
        self.assertEqual(result["classification"], "adapter_failed")
        self.assertTrue(result["contract_failures"])
        self.assertEqual(result["phases"]["warm"]["counts"]["scored"], 0)

    def test_missing_mutation_readback_becomes_adapter_failure_before_answering(self) -> None:
        suite = self.subset("memory-lifecycle-v1")
        unit = self.completed_unit("elf", suite)
        del unit["phases"]["warm"]["jobs"][0]["contexts"]
        with self.assertRaises(NativeContextError):
            answer_cases(suite, unit, 12000)
        attached = RUNNER.attach_shared_answers(suite, unit, {}, 12000)
        self.assertEqual(attached["result_class"], "adapter_failed")
        result = evaluate_unit(suite, unit, self.targets["elf"])
        self.assertEqual(result["classification"], "adapter_failed")
        self.assertIn("omitted native ranked contexts", result["contract_failures"][0])

    def test_incomplete_completed_unit_becomes_adapter_failure(self) -> None:
        suite = self.subset("common-core-v1", 2)
        unit = self.completed_unit("elf", suite)
        unit["phases"]["warm"]["jobs"].pop()
        result = evaluate_unit(suite, unit, self.targets["elf"])
        self.assertEqual(result["classification"], "adapter_failed")
        self.assertIn(
            "warm phase did not return the exact scheduled job set",
            result["contract_failures"],
        )
        self.assertEqual(result["phases"]["warm"]["counts"]["scored"], 0)

    def test_failed_top_level_unit_cannot_enter_quality_denominator(self) -> None:
        suite = self.subset("common-core-v1")
        unit = self.completed_unit("elf", suite)
        unit["result_class"] = "adapter_failed"
        result = evaluate_unit(suite, unit, self.targets["elf"])
        self.assertEqual(result["classification"], "adapter_failed")
        self.assertFalse(result["phases"]["warm"]["quality_denominator"])
        self.assertIsNone(result["phases"]["warm"]["metrics"])
        self.assertEqual(result["phases"]["warm"]["counts"]["scored"], 0)

    def test_deterministic_scoring_replays_byte_identically(self) -> None:
        suite = self.subset("common-core-v1", 2)
        unit = self.completed_unit("elf", suite)
        first = evaluate_unit(suite, copy.deepcopy(unit), self.targets["elf"])
        second = evaluate_unit(suite, copy.deepcopy(unit), self.targets["elf"])
        self.assertEqual(sha256_json(first), sha256_json(second))
        self.assertEqual(first["phases"]["warm"]["metrics"]["mean_recall_at_5"], 1.0)

    def test_shared_answer_context_uses_native_post_update_text(self) -> None:
        suite = self.subset("memory-lifecycle-v1")
        unit = self.completed_unit("elf", suite)
        cases = answer_cases(suite, unit, 12000)
        replacement = suite["jobs"][0]["operations"][0]["text"]
        rendered = "\n".join(cases[0]["context"])
        self.assertIn(replacement, rendered)
        self.assertNotIn(suite["jobs"][0]["corpus"][0]["text"], rendered)

        unit["phases"]["warm"]["jobs"][0]["contexts"][0]["text"] = suite["jobs"][0][
            "corpus"
        ][0]["text"]
        stale = "\n".join(answer_cases(suite, unit, 12000)[0]["context"])
        self.assertNotIn(replacement, stale)

    def test_shared_answer_requires_nonempty_text_and_preserves_raw_response(self) -> None:
        suite = self.subset("common-core-v1")
        case_id = opaque_job_id(suite["jobs"][0]["job_id"])
        unit = self.completed_unit("elf", suite)
        native = {
            "choices": [
                {
                    "message": {
                        "content": json.dumps(
                            {
                                "answers": [
                                    {
                                        "case_id": case_id,
                                        "text": "supported fact",
                                        "supported": True,
                                    }
                                ]
                            }
                        )
                    }
                }
            ],
            "usage": {"total_tokens": 10},
        }
        response = mock.MagicMock()
        response.__enter__.return_value.read.return_value = json.dumps(native).encode()
        env = {
            "BENCHMARK_CHAT_API_BASE": "https://provider.test/v1",
            "BENCHMARK_CHAT_MODEL": "model",
            "BENCHMARK_CHAT_REASONING_EFFORT": "high",
            "BENCHMARK_CHAT_API_KEY": "protected",
        }
        with mock.patch.object(
            RUNNER.urllib.request, "urlopen", return_value=response
        ) as urlopen:
            attached = RUNNER.attach_shared_answers(suite, unit, env, 12000)
        request_body = json.loads(urlopen.call_args.args[0].data)
        prompt = json.loads(request_body["messages"][0]["content"])
        self.assertIn("must never be empty", prompt["instruction"])
        self.assertEqual(attached["provider_raw"]["shared_answer"], native)
        self.assertEqual(
            attached["phases"]["warm"]["jobs"][0]["answer"]["text"],
            "supported fact",
        )

        invalid_native = copy.deepcopy(native)
        invalid_native["choices"][0]["message"]["content"] = json.dumps(
            {
                "answers": [
                    {"case_id": case_id, "text": "", "supported": True}
                ]
            }
        )
        invalid_response = mock.MagicMock()
        invalid_response.__enter__.return_value.read.return_value = json.dumps(
            invalid_native
        ).encode()
        with mock.patch.object(
            RUNNER.urllib.request, "urlopen", return_value=invalid_response
        ):
            failed = RUNNER.attach_shared_answers(
                suite, self.completed_unit("elf", suite), env, 12000
            )
        self.assertEqual(failed["result_class"], "provider_failed")
        self.assertIn("empty text", failed["failure"]["message"])
        self.assertEqual(failed["provider_raw"]["shared_answer"], invalid_native)

    def test_unsupported_answer_requires_unknown_text(self) -> None:
        suite = copy.deepcopy(self.suites["common-core-v1"])
        suite["jobs"] = [
            next(
                job
                for job in suite["jobs"]
                if job["qrels"].get("expect_unsupported")
            )
        ]
        suite["execution_mode"] = "readiness"
        unit = self.completed_unit("elf", suite)
        valid = evaluate_unit(suite, unit, self.targets["elf"])
        self.assertEqual(valid["phases"]["warm"]["jobs"][0]["answer_correct"], 1.0)
        self.assertEqual(
            valid["phases"]["warm"]["jobs"][0]["unsupported_answer_error"],
            0.0,
        )

        unit["phases"]["warm"]["jobs"][0]["answer"]["text"] = ""
        invalid = evaluate_unit(suite, unit, self.targets["elf"])
        self.assertEqual(
            invalid["phases"]["warm"]["jobs"][0]["answer_correct"], 0.0
        )
        self.assertEqual(
            invalid["phases"]["warm"]["jobs"][0]["unsupported_answer_error"],
            1.0,
        )

    def test_native_update_score_requires_receipt_and_post_update_readback(self) -> None:
        suite = self.subset("memory-lifecycle-v1")
        unit = self.completed_unit("elf", suite)
        replacement = suite["jobs"][0]["operations"][0]["text"]
        fresh = evaluate_unit(suite, unit, self.targets["elf"])
        self.assertEqual(
            fresh["phases"]["warm"]["jobs"][0]["native_update_success"], 1.0
        )

        unit["phases"]["warm"]["jobs"][0]["contexts"][0]["text"] = suite["jobs"][0][
            "corpus"
        ][0]["text"]
        stale = evaluate_unit(suite, unit, self.targets["elf"])
        self.assertNotIn(replacement, stale["phases"]["warm"]["jobs"][0]["native_contexts"][0]["text"])
        self.assertEqual(
            stale["phases"]["warm"]["jobs"][0]["native_update_success"], 0.0
        )

    def test_native_delete_score_requires_receipt_and_absent_post_delete_readback(self) -> None:
        suite = copy.deepcopy(self.suites["memory-lifecycle-v1"])
        suite["jobs"] = suite["jobs"][1:2]
        suite["execution_mode"] = "readiness"
        unit = self.completed_unit("elf", suite)
        deleted = suite["jobs"][0]["operations"][0]["evidence_id"]
        clean = evaluate_unit(suite, unit, self.targets["elf"])
        self.assertEqual(
            clean["phases"]["warm"]["jobs"][0]["native_delete_success"], 1.0
        )

        unit["phases"]["warm"]["jobs"][0]["contexts"].append(
            {
                "evidence_id": opaque_evidence_id(deleted),
                "text": suite["jobs"][0]["corpus"][1]["text"],
            }
        )
        stale = evaluate_unit(suite, unit, self.targets["elf"])
        self.assertEqual(
            stale["phases"]["warm"]["jobs"][0]["native_delete_success"], 0.0
        )

    def test_compose_names_are_isolated_and_bounded(self) -> None:
        names = {
            RUNNER.compose_project_name("run", suite, target)
            for suite in self.suites
            for target in self.targets
        }
        self.assertEqual(len(names), len(self.suites) * len(self.targets))
        self.assertTrue(all(name.startswith("elfb-") and len(name) <= 63 for name in names))

    def test_cleanup_checks_containers_volumes_and_networks(self) -> None:
        responses = [
            subprocess.CompletedProcess([], 0, "down"),
            subprocess.CompletedProcess([], 0, ""),
            subprocess.CompletedProcess([], 0, ""),
            subprocess.CompletedProcess([], 0, ""),
        ]
        with mock.patch.object(RUNNER, "command", side_effect=responses):
            cleanup = RUNNER.cleanup_project("elfb-test", Path("compose.yml"), {})
        self.assertTrue(cleanup["passed"])
        self.assertEqual(cleanup["remaining"], {"containers": [], "volumes": [], "networks": []})

    def test_provider_preflight_paths_are_openai_compatible(self) -> None:
        self.assertEqual(
            PREFLIGHT.endpoint("https://provider.test/v1", "embeddings"),
            "https://provider.test/v1/embeddings",
        )
        self.assertEqual(
            PREFLIGHT.endpoint("https://provider.test", "chat/completions"),
            "https://provider.test/v1/chat/completions",
        )

    def test_readiness_repairs_remain_native_and_bounded(self) -> None:
        compose = (REPO / "docker/benchmark/compose.yml").read_text(encoding="utf-8")
        unit_image = (REPO / "docker/benchmark/Dockerfile").read_text(encoding="utf-8")
        honcho_image = (REPO / "docker/benchmark/honcho.Dockerfile").read_text(
            encoding="utf-8"
        )
        self.assertIn('EMBEDDING_SEND_DIM: "true"', compose)
        self.assertIn('OPENKB_TIMEOUT_SECONDS: "1200"', compose)
        self.assertIn(
            "COPY config/local/tokenizer.wordlevel.json /config/local/tokenizer.wordlevel.json",
            unit_image,
        )
        self.assertIn("UV_PYTHON_INSTALL_DIR=/opt/uv-python", honcho_image)

    def test_openkb_timeout_preserves_bytes_and_product_failure(self) -> None:
        timeout = subprocess.TimeoutExpired(
            ["openkb"], 1, output=b"partial stdout\n", stderr=b"partial stderr\n"
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            stdout_path = root / "stdout.log"
            stderr_path = root / "stderr.log"
            with mock.patch.object(OPENKB.subprocess, "run", side_effect=timeout):
                with self.assertRaisesRegex(
                    OPENKB.OpenKBProductFailure, "native operation timed out"
                ):
                    OPENKB._run_native(
                        ["openkb"],
                        cwd=root,
                        env={},
                        stdout_path=stdout_path,
                        stderr_path=stderr_path,
                        timeout=1,
                    )
            self.assertEqual(stdout_path.read_text(encoding="utf-8"), "partial stdout\n")
            self.assertEqual(stderr_path.read_text(encoding="utf-8"), "partial stderr\n")

    def test_openkb_native_command_can_skip_its_interactive_key_prompt(self) -> None:
        completed = subprocess.CompletedProcess(["openkb"], 0, "ready\n", "")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with mock.patch.object(OPENKB.subprocess, "run", return_value=completed) as run:
                stdout, _ = OPENKB._run_native(
                    ["openkb"],
                    cwd=root,
                    env={"LLM_API_KEY": "protected"},
                    stdout_path=root / "stdout.log",
                    stderr_path=root / "stderr.log",
                    timeout=1,
                    stdin_text="\n",
                )
        self.assertEqual(stdout, "ready\n")
        self.assertEqual(run.call_args.kwargs["input"], "\n")

    def test_report_has_required_chinese_decision_sections(self) -> None:
        suite = self.subset("common-core-v1")
        unit = self.completed_unit("elf", suite)
        evaluation = evaluate_unit(suite, unit, self.targets["elf"])
        row = {
            "target": "elf",
            "unit_result": unit,
            "evaluation": evaluation,
            "cleanup": {"passed": True},
        }
        bundle = {
            "schema": "elf.benchmark_bundle/v1",
            "mode": "complete_measured_run",
            "source": {"head": "abc", "dirty": False, "content_sha256": "def"},
            "provider_routes": {},
            "provider_preflight": {},
            "acceptance": {"passed": True, "findings": []},
            "target_pins": {"elf": {}},
            "target_contracts": {
                "pageindex": {
                    "not_applicable": {
                        "common-core-v1": "The pinned revision constructs a hierarchy but exposes no native ranked retrieval operation."
                    }
                }
            },
            "target_image_digests": {},
            "suite_results": {"common-core-v1": {"results": [row]}},
        }
        report = REPORT.publish(bundle)
        for heading in (
            "决策摘要",
            "五项产品决策",
            "覆盖与失败",
            "逐产品实测强弱",
            "ELF 优化顺序",
        ):
            self.assertIn(heading, report)
        self.assertIn(
            "The pinned revision constructs a hierarchy but exposes no native ranked retrieval operation.",
            report,
        )

    def test_report_does_not_interpret_jobs_from_reclassified_elf_unit(self) -> None:
        suite = self.subset("memory-lifecycle-v1")
        unit = self.completed_unit("elf", suite)
        unit["phases"]["warm"]["jobs"][0]["operations"] = []
        evaluation = evaluate_unit(suite, unit, self.targets["elf"])
        row = {
            "target": "elf",
            "unit_result": unit,
            "evaluation": evaluation,
            "cleanup": {"passed": True},
        }
        bundle = {
            "target_pins": {"elf": {}},
            "suite_results": {"memory-lifecycle-v1": {"results": [row]}},
        }
        self.assertEqual(evaluation["classification"], "adapter_failed")
        self.assertEqual(REPORT.elf_jobs(bundle), [])
        summary = "\n".join(REPORT.elf_job_summary(bundle))
        self.assertIn("整个单元不计分", summary)
        self.assertNotIn("最强场景", summary)
        roadmap = "\n".join(REPORT.roadmap(bundle))
        self.assertIn("没有产生可归因的指标失败", roadmap)
        self.assertNotIn("先消除 ELF 原生运行失败", roadmap)
        observations = "\n".join(REPORT.product_observations(bundle))
        self.assertIn("不能从未计分单元形成实测强弱结论", observations)
        self.assertNotIn("memory-lifecycle-v1:adapter_failed", observations)

    def test_zero_retrieval_does_not_become_stale_suppression_strength(self) -> None:
        suite = self.subset("common-core-v1")
        elf = {
            "target": "elf",
            "evaluation": evaluate_unit(
                suite, self.completed_unit("elf", suite), self.targets["elf"]
            ),
            "cleanup": {"passed": True},
        }
        qmd = {
            "target": "qmd",
            "evaluation": evaluate_unit(
                suite, self.completed_unit("qmd", suite), self.targets["qmd"]
            ),
            "cleanup": {"passed": True},
        }
        qmd_metrics = qmd["evaluation"]["phases"]["warm"]["metrics"]
        qmd_metrics["mean_recall_at_5"] = 0.0
        qmd_metrics["mean_ndcg_at_5"] = 0.0
        qmd_metrics["forbidden_or_stale_evidence_hit_rate"] = 0.0
        qmd_metrics["privacy_scope_violation_rate"] = 0.0
        qmd_metrics["source_or_citation_trace_rate"] = None
        leaders = "\n".join(
            REPORT.metric_leaders(
                [elf, qmd],
                [
                    "forbidden_or_stale_evidence_hit_rate",
                    "privacy_scope_violation_rate",
                ],
            )
        )
        self.assertNotIn("qmd", leaders)
        bundle = {
            "target_pins": {"elf": {}, "qmd": {}},
            "suite_results": {"common-core-v1": {"results": [elf, qmd]}},
        }
        observations = "\n".join(REPORT.product_observations(bundle))
        self.assertIn("零陈旧命中与零召回同时出现", observations)
        qmd_line = next(line for line in observations.splitlines() if "**qmd**" in line)
        self.assertNotIn("陈旧证据命中率（方向化值 1.000）", qmd_line)

    def test_roadmap_lists_all_failures_and_separates_privacy(self) -> None:
        suite = self.subset("common-core-v1", 10)
        evaluation = evaluate_unit(
            suite, self.completed_unit("elf", suite), self.targets["elf"]
        )
        for job in evaluation["phases"]["warm"]["jobs"]:
            job["forbidden_evidence_hits"] = ["stale"]
            job["privacy_scope_violation"] = 0.0
            job["answer_correct"] = 1.0
        bundle = {
            "target_pins": {"elf": {}},
            "suite_results": {
                "common-core-v1": {
                    "results": [
                        {
                            "target": "elf",
                            "evaluation": evaluation,
                            "cleanup": {"passed": True},
                        }
                    ]
                }
            },
        }
        roadmap = "\n".join(REPORT.roadmap(bundle))
        self.assertIn("强化陈旧证据抑制", roadmap)
        self.assertIn("失败 jobs（10）", roadmap)
        self.assertNotIn("修复隐私范围隔离", roadmap)
        for job in suite["jobs"]:
            self.assertIn(job["job_id"], roadmap)

    def test_decisions_disclose_performance_and_add_thresholded_action(self) -> None:
        suite = self.subset("common-core-v1", 2)
        rows = []
        for target, latency, ingest in (("elf", 100.0, 1000.0), ("sag", 5.0, 10.0)):
            evaluation = evaluate_unit(
                suite, self.completed_unit(target, suite), self.targets[target]
            )
            evaluation["cold_ingest_duration_ms"] = ingest
            evaluation["phases"]["warm"]["metrics"]["mean_query_latency_ms"] = latency
            for job in evaluation["phases"]["warm"]["jobs"]:
                job["latency_ms"] = latency
                job["answer_correct"] = 1.0
                job["forbidden_evidence_hits"] = []
            rows.append(
                {
                    "target": target,
                    "evaluation": evaluation,
                    "cleanup": {"passed": True},
                }
            )
        bundle = {
            "target_pins": {"elf": {}, "sag": {}},
            "suite_results": {"common-core-v1": {"results": rows}},
        }
        decisions = "\n".join(REPORT.five_decisions(bundle))
        roadmap = "\n".join(REPORT.roadmap(bundle))
        self.assertIn("ELF 性能处置", decisions)
        self.assertIn("warm 为最快非 ELF 的 20.0×", decisions)
        self.assertIn("缩短检索与摄取关键路径", roadmap)
        self.assertIn("触发 10× 同 job 延迟阈值的 jobs（2）", roadmap)
        for job in suite["jobs"]:
            self.assertIn(job["job_id"], roadmap)


if __name__ == "__main__":
    unittest.main()
