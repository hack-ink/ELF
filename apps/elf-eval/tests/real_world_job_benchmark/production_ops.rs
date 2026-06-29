use std::{env, fs, process};

use color_eyre::Result;
use serde_json::Value;

use super::support::*;

#[test]
fn production_ops_fixtures_report_bounded_typed_states() -> Result<()> {
	let report = run_json_report_from(production_ops_fixture_dir())?;

	assert_production_ops_summary(&report)?;
	assert_production_ops_jobs(&report)?;
	assert_production_ops_operational_evidence(&report)?;

	Ok(())
}

fn assert_production_ops_summary(report: &Value) -> Result<()> {
	assert_eq!(report.pointer("/summary/job_count").and_then(Value::as_u64), Some(8));
	assert_eq!(report.pointer("/summary/pass").and_then(Value::as_u64), Some(6));
	assert_eq!(report.pointer("/summary/incomplete").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/blocked").and_then(Value::as_u64), Some(2));
	assert_eq!(report.pointer("/summary/not_encoded").and_then(Value::as_u64), Some(0));
	assert_eq!(report.pointer("/summary/evidence_coverage").and_then(Value::as_f64), Some(1.0));
	assert_eq!(
		report.pointer("/summary/qdrant_rebuild_case_count").and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report.pointer("/private_corpus_redaction/private_fixture_count").and_then(Value::as_u64),
		Some(1)
	);

	let suites = array_at(report, "/suites")?;
	let production_ops = find_by_field(suites, "/suite_id", "production_ops")?;

	assert_eq!(production_ops.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(production_ops.pointer("/encoded_job_count").and_then(Value::as_u64), Some(8));

	Ok(())
}

fn assert_production_ops_jobs(report: &Value) -> Result<()> {
	let jobs = array_at(report, "/jobs")?;
	let authority_recovery =
		find_by_field(jobs, "/job_id", "production-ops-authority-plane-recovery-001")?;
	let backfill = find_by_field(jobs, "/job_id", "production-ops-backfill-resume-001")?;
	let restore = find_by_field(jobs, "/job_id", "production-ops-restore-cold-start-001")?;
	let public_proxy = find_by_field(jobs, "/job_id", "production-ops-public-proxy-addendum-001")?;
	let private_manifest =
		find_by_field(jobs, "/job_id", "production-ops-private-manifest-blocked-001")?;
	let credentials = find_by_field(jobs, "/job_id", "production-ops-credential-boundary-001")?;
	let dependency = find_by_field(jobs, "/job_id", "production-ops-cold-start-dependency-001")?;

	assert_authority_recovery_job(authority_recovery)?;

	assert_eq!(authority_recovery.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(backfill.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(restore.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(restore.pointer("/qdrant_rebuild_case").and_then(Value::as_bool), Some(true));
	assert_eq!(public_proxy.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		public_proxy.pointer("/operational_evidence_tier").and_then(Value::as_str),
		Some("public_proxy")
	);
	assert_eq!(private_manifest.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		private_manifest.pointer("/operational_evidence_tier").and_then(Value::as_str),
		Some("private_corpus")
	);
	assert_eq!(credentials.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(
		credentials.pointer("/operational_evidence_tier").and_then(Value::as_str),
		Some("provider_backed")
	);
	assert_eq!(dependency.pointer("/status").and_then(Value::as_str), Some("pass"));

	Ok(())
}

fn assert_authority_recovery_job(job: &Value) -> Result<()> {
	assert_eq!(job.pointer("/qdrant_rebuild_case").and_then(Value::as_bool), Some(true));
	assert_eq!(job.pointer("/requires_caveat").and_then(Value::as_bool), Some(true));
	assert_eq!(
		job.pointer("/recovery_drills/0/contract_schema").and_then(Value::as_str),
		Some("elf.authority_recovery_drill/v1")
	);
	assert!(array_at(job, "/hard_fail_hits")?.is_empty());

	Ok(())
}

fn assert_production_ops_operational_evidence(report: &Value) -> Result<()> {
	assert_eq!(
		report.pointer("/operational_evidence/schema").and_then(Value::as_str),
		Some("elf.operational_evidence_gates/v1")
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/missing_private_provider_inputs_are_typed_blockers")
			.and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/private_corpus_pass_claim_allowed")
			.and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/provider_backed_pass_claim_allowed")
			.and_then(Value::as_bool),
		Some(false)
	);
	assert_eq!(
		report.pointer("/operational_evidence/latency/measured_job_count").and_then(Value::as_u64),
		Some(8)
	);
	assert_eq!(
		report.pointer("/operational_evidence/cost/jobs_with_cost_report").and_then(Value::as_u64),
		Some(8)
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/resource/resource_envelope_job_count")
			.and_then(Value::as_u64),
		Some(2)
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/cold_start_restore_rebuild/qdrant_rebuild_pass_count")
			.and_then(Value::as_u64),
		Some(2)
	);

	assert_authority_recovery_operational_evidence(report);

	let tiers = array_at(report, "/operational_evidence/tiers")?;
	let local_fixture = find_by_field(tiers, "/tier", "local_fixture")?;
	let public_proxy_tier = find_by_field(tiers, "/tier", "public_proxy")?;
	let private_corpus = find_by_field(tiers, "/tier", "private_corpus")?;
	let provider_backed = find_by_field(tiers, "/tier", "provider_backed")?;

	assert_eq!(local_fixture.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(local_fixture.pointer("/job_count").and_then(Value::as_u64), Some(5));
	assert_eq!(public_proxy_tier.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(public_proxy_tier.pointer("/job_count").and_then(Value::as_u64), Some(1));
	assert_eq!(private_corpus.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(private_corpus.pointer("/blocked").and_then(Value::as_u64), Some(1));
	assert_eq!(provider_backed.pointer("/status").and_then(Value::as_str), Some("blocked"));
	assert_eq!(provider_backed.pointer("/blocked").and_then(Value::as_u64), Some(1));

	Ok(())
}

fn assert_authority_recovery_operational_evidence(report: &Value) {
	assert_eq!(
		report
			.pointer("/operational_evidence/authority_recovery/drill_count")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/authority_recovery/authority_plane_count")
			.and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/authority_recovery/backup_pitr_restored_count")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/authority_recovery/record_count_preserved_count")
			.and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/authority_recovery/source_ref_preserved_count")
			.and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/authority_recovery/lifecycle_history_preserved_count")
			.and_then(Value::as_u64),
		Some(7)
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/authority_recovery/rpo_met_count")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/authority_recovery/rto_met_count")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/authority_recovery/idempotent_outbox_replay_count")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/authority_recovery/qdrant_rebuild_complete_count")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/authority_recovery/migration_repair_count")
			.and_then(Value::as_u64),
		Some(1)
	);
	assert_eq!(
		report
			.pointer("/operational_evidence/authority_recovery/dead_letter_handled_count")
			.and_then(Value::as_u64),
		Some(1)
	);
}

#[test]
fn authority_recovery_fixture_rejects_incomplete_recovery_predicates() -> Result<()> {
	for (slug, pointer, replacement, expected_error) in authority_recovery_failure_cases() {
		assert_authority_recovery_fixture_failure(
			slug,
			|fixture| set_json_pointer(fixture, pointer, replacement),
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
	let fixture_path = production_ops_fixture_dir().join("authority_plane_recovery_drill.json");
	let mut fixture = load_json(&fixture_path)?;

	mutate(&mut fixture)?;

	let temp_dir = env::temp_dir().join(format!("elf-authority-recovery-{slug}-{}", process::id()));

	fs::create_dir_all(&temp_dir)?;
	fs::write(temp_dir.join("fixture.json"), serde_json::to_vec_pretty(&fixture)?)?;

	let stderr = run_json_report_from_failure(temp_dir)?;

	assert!(
		stderr.contains(expected_error),
		"missing expected error `{expected_error}` in stderr: {stderr}",
	);

	Ok(())
}
