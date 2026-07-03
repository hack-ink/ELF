use std::{
	env, fs,
	path::PathBuf,
	process::{self, Command},
	time::{SystemTime, UNIX_EPOCH},
};

use color_eyre::{Result, eyre};
use serde_json::Value;

use crate::support;

const QMD_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const RUNNER_DIGEST: &str =
	"sha256:cea965615ad701b8b772f4a5607b982f01c3177e29fc8dbcd2b76b19ba862751";

struct QmdCandidateReplayFixture {
	temp_dir: PathBuf,
	product_manifest_path: PathBuf,
	freshness_manifest_path: PathBuf,
	out_path: PathBuf,
}
impl QmdCandidateReplayFixture {
	fn new(name: &str, product_manifest: &Value, freshness_manifest: &Value) -> Result<Self> {
		let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
		let temp_dir = env::temp_dir().join(format!("{name}-{}-{nonce}", process::id()));

		fs::create_dir_all(&temp_dir)?;

		let product_manifest_path = temp_dir.join("qmd-product-manifest.json");
		let freshness_manifest_path = temp_dir.join("freshness-manifest.json");
		let out_path = temp_dir.join("qmd-candidate-replay-gate.json");

		fs::write(
			&product_manifest_path,
			format!("{}\n", serde_json::to_string_pretty(product_manifest)?),
		)?;
		fs::write(
			&freshness_manifest_path,
			format!("{}\n", serde_json::to_string_pretty(freshness_manifest)?),
		)?;

		Ok(Self { temp_dir, product_manifest_path, freshness_manifest_path, out_path })
	}

	fn run_materializer(&self) -> Result<Value> {
		let output = Command::new("python3")
			.arg(
				support::workspace_root()?.join("scripts/materialize-qmd-candidate-replay-gate.py"),
			)
			.arg("--product-manifest")
			.arg(&self.product_manifest_path)
			.arg("--freshness-manifest")
			.arg(&self.freshness_manifest_path)
			.arg("--out")
			.arg(&self.out_path)
			.output()?;

		assert!(
			output.status.success(),
			"qmd candidate-replay gate materializer failed: {}",
			String::from_utf8_lossy(&output.stderr)
		);

		support::load_json(&self.out_path)
	}
}

impl Drop for QmdCandidateReplayFixture {
	fn drop(&mut self) {
		let _ = fs::remove_dir_all(&self.temp_dir);
	}
}

#[test]
fn qmd_candidate_replay_gate_passes_with_full_comparability_evidence() -> Result<()> {
	let fixture = QmdCandidateReplayFixture::new(
		"elf-qmd-candidate-replay-pass",
		&qmd_product_manifest(true, true, Some("qmd-held-out-audit"), qmd_per_query_rows()),
		&qmd_freshness_manifest(Some(RUNNER_DIGEST), Some(QMD_COMMIT)),
	)?;
	let manifest = fixture.run_materializer()?;

	assert_eq!(
		manifest.pointer("/schema").and_then(Value::as_str),
		Some("elf.qmd_candidate_replay_comparability_gate/v1")
	);
	assert_eq!(manifest.pointer("/result_state").and_then(Value::as_str), Some("pass"));
	assert_eq!(manifest.pointer("/comparable").and_then(Value::as_bool), Some(true));
	assert_eq!(
		manifest.pointer("/unqualified_leaderboard_claim_allowed").and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(manifest.pointer("/replay_artifact_row_count").and_then(Value::as_u64), Some(2));
	assert_eq!(manifest.pointer("/per_query_row_count").and_then(Value::as_u64), Some(2));
	assert_eq!(
		support::string_array_at(&manifest, "/source_manifest_corpus_ids")?,
		vec!["qmd-corpus"]
	);

	let gates = manifest
		.pointer("/gates")
		.and_then(Value::as_object)
		.ok_or_else(|| eyre::eyre!("missing gates object"))?;

	for gate_name in gates.keys() {
		assert_eq!(
			manifest.pointer(&format!("/gates/{gate_name}")).and_then(Value::as_bool),
			Some(true),
			"gate {gate_name} should pass"
		);
	}

	Ok(())
}

#[test]
fn qmd_candidate_replay_gate_blocks_missing_digest_audit_and_replay_rows() -> Result<()> {
	let fixture = QmdCandidateReplayFixture::new(
		"elf-qmd-candidate-replay-blocked",
		&qmd_product_manifest(false, false, None, vec![]),
		&qmd_freshness_manifest(None, Some(QMD_COMMIT)),
	)?;
	let manifest = fixture.run_materializer()?;

	assert_eq!(manifest.pointer("/result_state").and_then(Value::as_str), Some("blocked"));
	assert_eq!(manifest.pointer("/comparable").and_then(Value::as_bool), Some(false));
	assert_eq!(
		manifest.pointer("/unqualified_leaderboard_claim_allowed").and_then(Value::as_bool),
		Some(false)
	);
	assert!(support::array_contains_str(&manifest, "/missing_gates", "qmd_held_out")?);
	assert!(support::array_contains_str(&manifest, "/missing_gates", "qmd_leakage_audited")?);
	assert!(support::array_contains_str(
		&manifest,
		"/missing_gates",
		"qmd_audit_manifest_present"
	)?);
	assert!(support::array_contains_str(
		&manifest,
		"/missing_gates",
		"qmd_replay_artifact_present"
	)?);
	assert!(support::array_contains_str(
		&manifest,
		"/missing_gates",
		"qmd_candidate_replay_complete"
	)?);
	assert!(support::array_contains_str(
		&manifest,
		"/missing_gates",
		"qmd_container_digest_present"
	)?);

	Ok(())
}

#[test]
fn qmd_candidate_replay_gate_blocks_non_pass_product_state() -> Result<()> {
	let mut product_manifest =
		qmd_product_manifest(true, true, Some("qmd-held-out-audit"), qmd_per_query_rows());

	support::set_json_pointer(
		&mut product_manifest,
		"/rows/0/result_state",
		serde_json::json!("blocked"),
	)?;

	let fixture = QmdCandidateReplayFixture::new(
		"elf-qmd-candidate-replay-non-pass",
		&product_manifest,
		&qmd_freshness_manifest(Some(RUNNER_DIGEST), Some(QMD_COMMIT)),
	)?;
	let manifest = fixture.run_materializer()?;

	assert_eq!(manifest.pointer("/result_state").and_then(Value::as_str), Some("blocked"));
	assert!(support::array_contains_str(&manifest, "/missing_gates", "qmd_result_state_pass")?);
	assert_eq!(
		manifest.pointer("/unqualified_leaderboard_claim_allowed").and_then(Value::as_bool),
		Some(false)
	);

	Ok(())
}

#[test]
fn qmd_candidate_replay_gate_blocks_incomplete_source_id_mapping() -> Result<()> {
	let mut product_manifest =
		qmd_product_manifest(true, true, Some("qmd-held-out-audit"), qmd_per_query_rows());

	product_manifest
		.pointer_mut("/per_query_rows/0")
		.and_then(Value::as_object_mut)
		.ok_or_else(|| eyre::eyre!("missing qmd per-query row"))?
		.remove("source_manifest_corpus_id");

	let fixture = QmdCandidateReplayFixture::new(
		"elf-qmd-candidate-replay-missing-source-id",
		&product_manifest,
		&qmd_freshness_manifest(Some(RUNNER_DIGEST), Some(QMD_COMMIT)),
	)?;
	let manifest = fixture.run_materializer()?;

	assert_eq!(manifest.pointer("/result_state").and_then(Value::as_str), Some("blocked"));
	assert!(support::array_contains_str(&manifest, "/missing_gates", "qmd_source_id_mapped")?);

	Ok(())
}

#[test]
fn qmd_candidate_replay_gate_blocks_non_pass_or_partial_replay_rows() -> Result<()> {
	let mut product_manifest =
		qmd_product_manifest(true, true, Some("qmd-held-out-audit"), qmd_per_query_rows());

	support::set_json_pointer(
		&mut product_manifest,
		"/per_query_rows/0/result_state",
		serde_json::json!("incomplete"),
	)?;
	support::set_json_pointer(
		&mut product_manifest,
		"/rows/0/ranking_coverage_state",
		serde_json::json!("partial_coverage"),
	)?;

	let fixture = QmdCandidateReplayFixture::new(
		"elf-qmd-candidate-replay-partial-replay",
		&product_manifest,
		&qmd_freshness_manifest(Some(RUNNER_DIGEST), Some(QMD_COMMIT)),
	)?;
	let manifest = fixture.run_materializer()?;

	assert_eq!(manifest.pointer("/result_state").and_then(Value::as_str), Some("blocked"));
	assert!(support::array_contains_str(
		&manifest,
		"/missing_gates",
		"qmd_candidate_replay_complete"
	)?);
	assert!(support::array_contains_str(
		&manifest,
		"/missing_gates",
		"qmd_aggregate_replay_fields_complete"
	)?);

	Ok(())
}

#[test]
fn qmd_candidate_replay_gate_blocks_split_reproducibility_rows() -> Result<()> {
	let fixture = QmdCandidateReplayFixture::new(
		"elf-qmd-candidate-replay-split-repro",
		&qmd_product_manifest(true, true, Some("qmd-held-out-audit"), qmd_per_query_rows()),
		&qmd_split_freshness_manifest(),
	)?;
	let manifest = fixture.run_materializer()?;

	assert_eq!(manifest.pointer("/result_state").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		manifest.pointer("/gates/qmd_container_digest_present").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		manifest.pointer("/gates/qmd_product_commit_present").and_then(Value::as_bool),
		Some(true)
	);
	assert!(support::array_contains_str(
		&manifest,
		"/missing_gates",
		"qmd_reproducibility_row_bound"
	)?);

	Ok(())
}

fn qmd_product_manifest(
	held_out: bool,
	leakage_audited: bool,
	audit_manifest_id: Option<&str>,
	per_query_rows: Vec<Value>,
) -> Value {
	serde_json::json!({
		"schema": "elf.agent_memory_quantitative_product_manifest/v1",
		"manifest_id": "qmd-candidate-replay-product-manifest",
		"corpus_id": "qmd-corpus",
		"rows": [{
			"product": "qmd",
			"adapter_id": "qmd_live_real_world",
			"adapter_name": "qmd live real-world CLI adapter",
			"suite": "retrieval",
			"evidence_class": "live_real_world",
			"source_manifest_corpus_id": "qmd-corpus",
			"result_state": "pass",
			"comparable": true,
			"metric_comparable": true,
			"leaderboard_eligible": false,
			"held_out": held_out,
			"leakage_audited": leakage_audited,
			"audit_manifest_id": audit_manifest_id,
			"fixture_regression_only": false,
			"sample_size": 2,
			"ranking_query_count": 2,
			"ranking_coverage_state": "complete",
			"ranked_candidate_source": "produced_evidence_order",
			"qrel_source": "explicit_qrels",
			"explicit_qrel_query_count": 2,
			"metrics": {},
			"metric_states": {},
			"denominators": {},
			"confidence_intervals": {},
			"claim_boundary": "Qualified qmd candidate-replay comparability only; no unqualified leaderboard claim."
		}],
		"per_query_rows": per_query_rows
	})
}

fn qmd_per_query_rows() -> Vec<Value> {
	vec![qmd_per_query_row("qmd-query-1", 4, 1), qmd_per_query_row("qmd-query-2", 5, 2)]
}

fn qmd_per_query_row(query_id: &str, candidate_count: u64, expected_relevant_count: u64) -> Value {
	serde_json::json!({
		"product": "qmd",
		"adapter_id": "qmd_live_real_world",
		"query_id": query_id,
		"source_manifest_corpus_id": "qmd-corpus",
		"result_state": "pass",
		"candidate_count": candidate_count,
		"expected_relevant_count": expected_relevant_count,
		"qrel_source": "explicit_qrels",
		"metrics": {},
		"metric_states": {},
		"denominators": {},
		"claim_boundary": "Per-query replay row emitted by product runtime candidates with explicit qrels."
	})
}

fn qmd_freshness_manifest(container_digest: Option<&str>, product_commit: Option<&str>) -> Value {
	serde_json::json!({
		"schema": "elf.quantitative_artifact_freshness_manifest/v1",
		"combined_inputs": [{
			"label": "qmd-live-explicit-qrels",
			"rows": [{
				"product": "qmd",
				"adapter_id": "qmd_live_real_world",
				"evidence_class": "live_real_world",
				"result_state": "pass",
				"leaderboard_eligible": false,
				"metric_comparable": true,
				"present_in_combined_manifest": true,
				"reproducibility": {
					"container_image_digest": container_digest,
					"product_commit": product_commit,
					"public_reproducible": container_digest.is_some() && product_commit.is_some(),
					"missing_fields": []
				}
			}]
		}]
	})
}

fn qmd_split_freshness_manifest() -> Value {
	serde_json::json!({
		"schema": "elf.quantitative_artifact_freshness_manifest/v1",
		"combined_inputs": [{
			"label": "qmd-live-explicit-qrels",
			"rows": [
				{
					"product": "qmd",
					"adapter_id": "qmd_live_real_world",
					"reproducibility": {
						"container_image_digest": RUNNER_DIGEST,
						"product_commit": null,
						"public_reproducible": false,
						"missing_fields": ["product_commit"]
					}
				},
				{
					"product": "qmd",
					"adapter_id": "qmd_live_real_world",
					"reproducibility": {
						"container_image_digest": null,
						"product_commit": QMD_COMMIT,
						"public_reproducible": false,
						"missing_fields": ["container_image_digest"]
					}
				}
			]
		}]
	})
}
