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
            answer = {
                "text": " ".join(job["qrels"].get("answer_facts") or []) or "unknown",
                "supported": not bool(job["qrels"].get("expect_unsupported")),
            }
            cold_jobs.append(
                {
                    "job_id": opaque_job_id(job["job_id"]),
                    "classification": "completed",
                    "evidence_ids": retrieved,
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

    def test_deterministic_scoring_replays_byte_identically(self) -> None:
        suite = self.subset("common-core-v1", 2)
        unit = self.completed_unit("elf", suite)
        first = evaluate_unit(suite, copy.deepcopy(unit), self.targets["elf"])
        second = evaluate_unit(suite, copy.deepcopy(unit), self.targets["elf"])
        self.assertEqual(sha256_json(first), sha256_json(second))
        self.assertEqual(first["phases"]["warm"]["metrics"]["mean_recall_at_5"], 1.0)

    def test_shared_answer_context_uses_post_update_text(self) -> None:
        suite = self.subset("memory-lifecycle-v1")
        unit = self.completed_unit("elf", suite)
        cases = answer_cases(suite, unit, 12000)
        replacement = suite["jobs"][0]["operations"][0]["text"]
        rendered = "\n".join(cases[0]["context"])
        self.assertIn(replacement, rendered)
        self.assertNotIn(suite["jobs"][0]["corpus"][0]["text"], rendered)

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
            "target_image_digests": {},
            "suite_results": {"common-core-v1": {"results": [row]}},
        }
        report = REPORT.publish(bundle)
        for heading in ("决策摘要", "覆盖与失败", "逐产品实测强弱", "ELF 优化顺序"):
            self.assertIn(heading, report)


if __name__ == "__main__":
    unittest.main()
