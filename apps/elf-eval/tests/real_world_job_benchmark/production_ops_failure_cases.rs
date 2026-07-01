use std::{env, fs, process};

use color_eyre::Result;
use serde_json::Value;

use crate::support;

#[test]
fn authority_recovery_fixture_rejects_incomplete_recovery_predicates() -> Result<()> {
	for (slug, pointer, replacement, expected_error) in authority_recovery_failure_cases() {
		assert_authority_recovery_fixture_failure(
			slug,
			|fixture| support::set_json_pointer(fixture, pointer, replacement),
			expected_error,
		)?;
	}

	Ok(())
}

fn authority_recovery_failure_cases() -> Vec<(&'static str, &'static str, Value, &'static str)> {
	vec![
		(
			"unrestored-backup",
			"/corpus/adapter_response/answer/recovery_drills/0/backup_pitr/restored",
			serde_json::json!(false),
			"incomplete backup/PITR drill evidence",
		),
		(
			"record-count-loss",
			"/corpus/adapter_response/answer/recovery_drills/0/authority_record_counts/0/after_count",
			serde_json::json!(2),
			"lost or gained source authority records",
		),
		(
			"source-ref-loss",
			"/corpus/adapter_response/answer/recovery_drills/0/authority_record_counts/0/source_refs_preserved",
			serde_json::json!(false),
			"did not preserve source authority source refs",
		),
		(
			"lifecycle-history-loss",
			"/corpus/adapter_response/answer/recovery_drills/0/authority_record_counts/0/lifecycle_history_preserved",
			serde_json::json!(false),
			"did not preserve source authority lifecycle history",
		),
		(
			"hidden-source-of-truth",
			"/corpus/adapter_response/answer/recovery_drills/0/degraded_read/source_of_truth_visible",
			serde_json::json!(false),
			"hidden source-of-truth records during degraded read",
		),
		(
			"rpo-miss",
			"/corpus/adapter_response/answer/recovery_drills/0/rpo/measured_seconds",
			serde_json::json!(61.0),
			"exceeded rpo recovery target",
		),
		(
			"non-idempotent-outbox",
			"/corpus/adapter_response/answer/recovery_drills/0/outbox_replay/duplicate_write_count",
			serde_json::json!(1),
			"incomplete outbox replay drill evidence",
		),
		(
			"incomplete-qdrant-rebuild",
			"/corpus/adapter_response/answer/recovery_drills/0/qdrant_rebuild/complete",
			serde_json::json!(false),
			"incomplete Qdrant rebuild drill evidence",
		),
		(
			"missing-migration-repair",
			"/corpus/adapter_response/answer/recovery_drills/0/migration_repair/applied",
			serde_json::json!(false),
			"incomplete migration repair drill evidence",
		),
		(
			"dead-letter-underhandled",
			"/corpus/adapter_response/answer/recovery_drills/0/dead_letter/handled_count",
			serde_json::json!(1),
			"incomplete dead-letter handling drill evidence",
		),
	]
}

fn assert_authority_recovery_fixture_failure<F>(
	slug: &str,
	mutate: F,
	expected_error: &str,
) -> Result<()>
where
	F: FnOnce(&mut Value) -> Result<()>,
{
	let fixture_path =
		support::production_ops_fixture_dir().join("authority_plane_recovery_drill.json");
	let mut fixture = support::load_json(&fixture_path)?;

	mutate(&mut fixture)?;

	let temp_dir = env::temp_dir().join(format!("elf-authority-recovery-{slug}-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("fixture.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let stderr = support::run_json_report_from_failure(temp_dir)?;

	assert!(
		stderr.contains(expected_error),
		"missing expected error `{expected_error}` in stderr: {stderr}",
	);

	Ok(())
}
