use std::{
	env, fs,
	process::{self, Command},
};

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn graphify_generated_manifest_keeps_retrieval_unscored() -> Result<()> {
	let manifest = serde_json::json!({
		"schema": "elf.real_world_external_adapter_manifest/v1",
		"manifest_id": "graphify-generated-manifest-test",
		"docker_isolation": {
			"default": true,
			"compose_file": "docker-compose.baseline.yml",
			"runner": "scripts/graphify-docker-graph-report-smoke.py",
			"artifact_dir": "tmp/real-world-memory/graphify-smoke",
			"host_global_installs_required": false,
			"notes": ["Synthetic graphify generated-manifest regression test."]
		},
		"adapters": [{
			"adapter_id": "graphify_docker_smoke",
			"project": "graphify",
			"adapter_kind": "docker_cli_graph_report_smoke",
			"evidence_class": "live_real_world",
			"docker_default": true,
			"host_global_installs_required": false,
			"overall_status": "wrong_result",
			"setup": {
				"status": "pass",
				"evidence": "setup evidence",
				"command": "cargo make smoke-graphify-docker-graph-report",
				"artifact": "tmp/real-world-memory/graphify-smoke/graphify-smoke.json"
			},
			"run": {
				"status": "pass",
				"evidence": "run evidence",
				"command": "cargo make smoke-graphify-docker-graph-report",
				"artifact": "tmp/real-world-memory/graphify-smoke/summary.json"
			},
			"result": {
				"status": "wrong_result",
				"evidence": "result evidence",
				"artifact": "tmp/real-world-memory/graphify-smoke/graphify-report.json"
			},
			"capabilities": [{
				"capability": "quality_or_scale_claim",
				"status": "not_encoded",
				"evidence": "No broad graph quality claim."
			}],
			"suites": [
				{
					"suite_id": "knowledge_compilation",
					"status": "wrong_result",
					"evidence": "Only the generated graph/report evidence-mapping job is represented."
				},
				{
					"suite_id": "retrieval",
					"status": "blocked",
					"evidence": "The smoke uses graphify query output only to support source mapping; broad retrieval quality is not scored."
				}
			],
			"evidence": [],
			"execution_metadata": {
				"setup_path": "cargo make smoke-graphify-docker-graph-report",
				"runtime_boundary": "Docker-only generated graph/report smoke.",
				"resource_expectation": "Tiny generated corpus only.",
				"retry_guidance": [],
				"sources": [{
					"label": "graphify",
					"url": "https://github.com/safishamsi/graphify",
					"evidence": "Synthetic generated-manifest regression source."
				}],
				"research_depth": "Generated smoke manifest path"
			},
			"notes": ["tiny smoke non-pass"]
		}]
	});
	let temp_dir =
		env::temp_dir().join(format!("elf-real-world-graphify-manifest-test-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;

	let manifest_path = temp_dir.join("manifest.json");
	let report_path = temp_dir.join("report.json");

	fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(support::fixture_dir())
		.arg("--out")
		.arg(&report_path)
		.arg("--external-adapter-manifest")
		.arg(&manifest_path)
		.output()?;

	assert!(
		output.status.success(),
		"real_world_job runner failed: {}",
		String::from_utf8_lossy(&output.stderr),
	);

	let report: Value = serde_json::from_slice(&fs::read(&report_path)?)?;
	let adapters = support::array_at(&report, "/external_adapters/adapters")?;
	let graphify = support::find_by_field(adapters, "/adapter_id", "graphify_docker_smoke")?;
	let suites = support::array_at(graphify, "/suites")?;
	let retrieval = support::find_by_field(suites, "/suite_id", "retrieval")?;

	assert_eq!(retrieval.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert!(
		retrieval
			.pointer("/evidence")
			.and_then(Value::as_str)
			.is_some_and(|text| { text.contains("broad retrieval quality is not scored") })
	);

	Ok(())
}
