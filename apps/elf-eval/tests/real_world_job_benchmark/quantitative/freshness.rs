use std::{
	env, fs,
	path::PathBuf,
	process::{self, Command},
	time::{SystemTime, UNIX_EPOCH},
};

use color_eyre::{Result, eyre};
use serde_json::Value;

use crate::support;

const HONCHO_COMMIT: &str = "60a15e664d7298eb790b788e95c6ca2e6bd30c80";
const RUNNER_DIGEST: &str =
	"sha256:cea965615ad701b8b772f4a5607b982f01c3177e29fc8dbcd2b76b19ba862751";

struct FreshnessFixture {
	temp_dir: PathBuf,
	product_manifest_path: PathBuf,
	sync_log_path: PathBuf,
	out_path: PathBuf,
}
impl FreshnessFixture {
	fn new(name: &str, product_manifest: &Value) -> Result<Self> {
		let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
		let temp_dir = env::temp_dir().join(format!("{name}-{}-{nonce}", process::id()));

		fs::create_dir_all(&temp_dir)?;

		let product_manifest_path = temp_dir.join("product-manifest.json");
		let sync_log_path = temp_dir.join("synced-artifacts.tsv");
		let out_path = temp_dir.join("freshness.json");

		fs::write(
			&product_manifest_path,
			format!("{}\n", serde_json::to_string_pretty(product_manifest)?),
		)?;
		fs::write(
			&sync_log_path,
			format!(
				"combined-input\thoncho-live\t{}\tcurrent_docker_run\n",
				product_manifest_path.display()
			),
		)?;

		Ok(Self { temp_dir, product_manifest_path, sync_log_path, out_path })
	}

	fn run_materializer(&self) -> Result<Value> {
		let output = Command::new("python3")
			.arg(
				support::workspace_root()?
					.join("scripts/materialize-quantitative-artifact-freshness.py"),
			)
			.arg("--sync-log")
			.arg(&self.sync_log_path)
			.arg("--combined-product-manifest")
			.arg(&self.product_manifest_path)
			.arg("--out")
			.arg(&self.out_path)
			.arg("--run-live-explicit-qrels")
			.arg("1")
			.arg("--run-langgraph")
			.arg("1")
			.env("ELF_BASELINE_RUNNER_IMAGE_DIGEST", RUNNER_DIGEST)
			.output()?;

		assert!(
			output.status.success(),
			"freshness materializer failed: {}",
			String::from_utf8_lossy(&output.stderr)
		);

		support::load_json(&self.out_path)
	}
}

impl Drop for FreshnessFixture {
	fn drop(&mut self) {
		let _ = fs::remove_dir_all(&self.temp_dir);
	}
}

#[test]
fn quantitative_freshness_accepts_runner_image_digest_for_public_reproducibility() -> Result<()> {
	let fixture = FreshnessFixture::new(
		"elf-runner-digest-freshness",
		&honcho_product_manifest(honcho_runtime_attestation(
			"pass",
			Some("runtime_source_checkout_verified"),
		)),
	)?;
	let manifest = fixture.run_materializer()?;

	assert_eq!(
		manifest.pointer("/reproducibility_summary/state").and_then(Value::as_str),
		Some("public_reproducibility_ready")
	);
	assert_eq!(
		manifest
			.pointer("/reproducibility_summary/public_reproducible_claim_allowed")
			.and_then(Value::as_bool),
		Some(true)
	);
	assert!(
		manifest
			.pointer("/reproducibility_summary/missing_field_counts/container_image_digest")
			.is_none()
	);

	let row = only_freshness_row(&manifest)?;

	assert_eq!(
		row.pointer("/reproducibility/container_image_digest").and_then(Value::as_str),
		Some(RUNNER_DIGEST)
	);
	assert_eq!(
		row.pointer("/reproducibility/container_image_digest_source").and_then(Value::as_str),
		Some("env:ELF_BASELINE_RUNNER_IMAGE_DIGEST")
	);
	assert_eq!(
		row.pointer("/reproducibility/public_reproducible").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(support::array_at(row, "/reproducibility/missing_fields")?.len(), 0);

	Ok(())
}

#[test]
fn quantitative_docker_task_routes_through_split_makefile_and_digest_runner() -> Result<()> {
	let task_catalog = support::make_task_catalog()?;
	let workspace = support::workspace_root()?;
	let docker_script = fs::read_to_string(workspace.join("scripts/real-world-docker.sh"))?;
	let aggregate_script =
		fs::read_to_string(workspace.join("scripts/real-world-quantitative-docker.sh"))?;

	assert!(task_catalog.contains("[tasks.real-world-memory-quantitative-docker]"));
	assert!(task_catalog.contains("\"memory-quantitative-docker\""));
	assert!(docker_script.contains("memory-quantitative-docker)"));
	assert!(docker_script.contains("build_baseline_runner_with_digest"));
	assert!(aggregate_script.contains("require_runner_image_digest"));
	assert!(aggregate_script.contains("materialize-quantitative-artifact-freshness.py"));

	Ok(())
}

#[test]
fn quantitative_freshness_rejects_runtime_sensitive_commit_without_attestation() -> Result<()> {
	let mut product_manifest = honcho_product_manifest(honcho_runtime_attestation(
		"pass",
		Some("runtime_source_checkout_verified"),
	));

	product_manifest
		.pointer_mut("/rows/0/runtime_source_attestation")
		.ok_or_else(|| eyre::eyre!("missing row attestation"))?
		.take();
	product_manifest
		.as_object_mut()
		.ok_or_else(|| eyre::eyre!("product manifest is not an object"))?
		.remove("runtime_source_attestation");

	let fixture = FreshnessFixture::new("elf-runtime-sensitive-commit-spoof", &product_manifest)?;
	let manifest = fixture.run_materializer()?;

	assert_eq!(
		manifest.pointer("/reproducibility_summary/ready_row_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		manifest
			.pointer("/reproducibility_summary/missing_field_counts/product_commit")
			.and_then(Value::as_u64),
		Some(1)
	);

	let gap = only_product_commit_gap(&manifest)?;

	assert_eq!(
		gap.pointer("/reproducibility/runtime_source_attestation/status").and_then(Value::as_str),
		Some("missing")
	);
	assert_eq!(
		gap.pointer("/reproducibility/runtime_source_attestation/reason").and_then(Value::as_str),
		Some("missing_runtime_source_attestation")
	);
	assert!(support::array_at(gap, "/reproducibility/rejected_product_commit_values")?.iter().any(
		|item| item.pointer("/value").and_then(Value::as_str) == Some(HONCHO_COMMIT)
			&& item.pointer("/reason").and_then(Value::as_str)
				== Some("missing_runtime_source_attestation")
	));

	Ok(())
}

#[test]
fn quantitative_freshness_rejects_non_pass_attestation_without_reason() -> Result<()> {
	let fixture = FreshnessFixture::new(
		"elf-non-pass-attestation",
		&honcho_product_manifest(honcho_runtime_attestation("fail", None)),
	)?;
	let manifest = fixture.run_materializer()?;

	assert_eq!(
		manifest.pointer("/reproducibility_summary/ready_row_count").and_then(Value::as_u64),
		Some(0)
	);
	assert_eq!(
		manifest
			.pointer("/reproducibility_summary/missing_field_counts/product_commit")
			.and_then(Value::as_u64),
		Some(1)
	);

	let gap = only_product_commit_gap(&manifest)?;

	assert_eq!(
		gap.pointer("/reproducibility/runtime_source_attestation/status").and_then(Value::as_str),
		Some("fail")
	);
	assert_eq!(
		gap.pointer("/reproducibility/runtime_source_attestation/reason").and_then(Value::as_str),
		Some("runtime_source_attestation_not_pass")
	);
	assert!(
		support::array_at(gap, "/reproducibility/rejected_product_commit_values")?
			.iter()
			.any(|item| item.pointer("/reason").and_then(Value::as_str)
				== Some("runtime_source_attestation_not_pass"))
	);

	Ok(())
}

fn honcho_product_manifest(runtime_source_attestation: Value) -> Value {
	serde_json::json!({
		"schema": "elf.agent_memory_quantitative_product_manifest/v1",
		"manifest_id": "honcho-test-product-manifest",
		"product": "Honcho",
		"adapter_id": "honcho_live_real_world",
		"corpus_id": "test-corpus",
		"rows": [{
			"product": "Honcho",
			"adapter_id": "honcho_live_real_world",
			"adapter_name": "Honcho live adapter",
			"suite": "retrieval",
			"evidence_class": "live_real_world",
			"result_state": "wrong_result",
			"leaderboard_eligible": false,
			"metric_comparable": false,
			"product_commit": HONCHO_COMMIT,
			"product_commit_source": "git.rev_parse_head:honcho_source_dir",
			"runtime_source_attestation": runtime_source_attestation
		}]
	})
}

fn honcho_runtime_attestation(status: &str, reason: Option<&str>) -> Value {
	let mut attestation = serde_json::json!({
		"status": status,
		"product_commit": HONCHO_COMMIT,
		"runtime_executed": true,
		"source_checkout_used": true,
		"runtime_artifact": "tmp/honcho/product-manifest.json"
	});

	if let Some(reason) = reason {
		attestation["reason"] = serde_json::json!(reason);
	}

	attestation
}

fn only_freshness_row(manifest: &Value) -> Result<&Value> {
	let inputs = support::array_at(manifest, "/combined_inputs")?;
	let input = inputs.first().ok_or_else(|| eyre::eyre!("missing freshness input"))?;
	let rows = support::array_at(input, "/rows")?;

	rows.first().ok_or_else(|| eyre::eyre!("missing freshness row"))
}

fn only_product_commit_gap(manifest: &Value) -> Result<&Value> {
	let gaps = support::array_at(manifest, "/product_commit_gap_rows")?;

	gaps.first().ok_or_else(|| eyre::eyre!("missing product commit gap row"))
}
