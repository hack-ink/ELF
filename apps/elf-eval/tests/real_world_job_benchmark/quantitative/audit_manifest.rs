use std::{
	env, fs,
	process::{self, Command},
};

use color_eyre::{Result, eyre};
use serde_json::Value;

use crate::support;

#[test]
fn quantitative_audit_manifest_exports_and_opens_current_row_gates() -> Result<()> {
	let temp_dir =
		env::temp_dir().join(format!("elf-quantitative-audit-manifest-test-{}", process::id()));
	let manifest_path = temp_dir.join("audit-manifest.json");

	fs::create_dir_all(&temp_dir)?;

	let export = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("export-quantitative-audit-manifest")
		.arg("--fixtures")
		.arg(support::adversarial_quality_fixture_dir())
		.arg("--out")
		.arg(&manifest_path)
		.arg("--run-id")
		.arg("audit-import-test")
		.arg("--held-out")
		.arg("--leakage-audited")
		.arg("--control")
		.arg("query_ids_locked_before_product_runtime")
		.arg("--control")
		.arg("product_runtime_did_not_receive_expected_answers_or_qrels")
		.arg("--control")
		.arg("ranked_candidates_emitted_by_product_runtime")
		.output()?;

	assert!(
		export.status.success(),
		"quantitative audit export failed: {}",
		String::from_utf8_lossy(&export.stderr)
	);

	let manifest = support::load_json(&manifest_path)?;

	assert_eq!(
		manifest.pointer("/schema").and_then(Value::as_str),
		Some("elf.agent_memory_quantitative_audit_manifest/v1")
	);
	assert_eq!(manifest.pointer("/held_out").and_then(Value::as_bool), Some(true));
	assert_eq!(manifest.pointer("/leakage_audited").and_then(Value::as_bool), Some(true));
	assert_eq!(
		support::array_at(&manifest, "/query_ids")?.len() as u64,
		manifest.pointer("/ranking_query_count").and_then(Value::as_u64).unwrap_or_default()
	);

	let imported = super::run_report_with_quantitative_audit(&manifest_path, "audit-import-test")?;
	let row = support::array_at(&imported, "/quantitative_scoreboard/rows")?
		.first()
		.ok_or_else(|| eyre::eyre!("missing quantitative row"))?;

	assert_eq!(row.pointer("/held_out").and_then(Value::as_bool), Some(true));
	assert_eq!(row.pointer("/leakage_audited").and_then(Value::as_bool), Some(true));
	assert_eq!(
		row.pointer("/audit_manifest_id").and_then(Value::as_str),
		Some("audit-import-test-quantitative-audit-manifest")
	);
	assert_eq!(row.pointer("/leaderboard_eligible").and_then(Value::as_bool), Some(false));

	Ok(())
}

#[test]
fn quantitative_audit_manifest_rejects_wrong_run_id_imports() -> Result<()> {
	let temp_dir =
		env::temp_dir().join(format!("elf-quantitative-audit-manifest-run-test-{}", process::id()));
	let manifest_path = temp_dir.join("audit-manifest.json");

	fs::create_dir_all(&temp_dir)?;

	let export = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("export-quantitative-audit-manifest")
		.arg("--fixtures")
		.arg(support::adversarial_quality_fixture_dir())
		.arg("--out")
		.arg(&manifest_path)
		.arg("--run-id")
		.arg("audit-import-test")
		.output()?;

	assert!(
		export.status.success(),
		"quantitative audit export failed: {}",
		String::from_utf8_lossy(&export.stderr)
	);

	let output = Command::new(env!("CARGO_BIN_EXE_real_world_job_benchmark"))
		.arg("run")
		.arg("--fixtures")
		.arg(support::adversarial_quality_fixture_dir())
		.arg("--run-id")
		.arg("different-run")
		.arg("--quantitative-audit-manifest")
		.arg(&manifest_path)
		.output()?;

	assert!(!output.status.success());
	assert!(String::from_utf8_lossy(&output.stderr).contains("expected different-run"));

	Ok(())
}
