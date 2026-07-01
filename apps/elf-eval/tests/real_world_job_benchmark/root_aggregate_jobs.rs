use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(crate) fn assert_root_aggregate_jobs(report: &Value) -> Result<()> {
	let jobs = support::array_at(report, "/jobs")?;
	let rebuild = support::find_by_field(jobs, "/job_id", "trust-sot-rebuild-001")?;
	let redaction = support::find_by_field(jobs, "/job_id", "capture-redaction-exclusion-001")?;
	let personalization =
		support::find_by_field(jobs, "/job_id", "personalization-scoped-preference-001")?;
	let relation_job =
		support::find_by_field(jobs, "/job_id", "memory-evolution-relation-temporal-001")?;
	let delete_job = support::find_by_field(jobs, "/job_id", "memory-evolution-delete-ttl-001")?;
	let stage_job =
		support::find_by_field(jobs, "/job_id", "operator-debug-stage-attribution-001")?;
	let production_restore =
		support::find_by_field(jobs, "/job_id", "production-ops-restore-cold-start-001")?;
	let production_authority =
		support::find_by_field(jobs, "/job_id", "production-ops-authority-plane-recovery-001")?;
	let core_fallback =
		support::find_by_field(jobs, "/job_id", "core-archival-archival-fallback-001")?;
	let stale_core =
		support::find_by_field(jobs, "/job_id", "core-archival-stale-core-detection-001")?;
	let scheduled_weekly =
		support::find_by_field(jobs, "/job_id", "scheduled-weekly-project-status-summary-001")?;

	assert_eq!(rebuild.pointer("/qdrant_rebuild_case").and_then(Value::as_bool), Some(true));
	assert_eq!(
		production_restore.pointer("/qdrant_rebuild_case").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(
		production_authority.pointer("/qdrant_rebuild_case").and_then(Value::as_bool),
		Some(true)
	);
	assert_eq!(production_authority.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		production_authority.pointer("/recovery_drills/0/contract_schema").and_then(Value::as_str),
		Some("elf.authority_recovery_drill/v1")
	);
	assert_eq!(redaction.pointer("/redaction_leak_count").and_then(Value::as_u64), Some(0));
	assert_eq!(personalization.pointer("/scope_check_count").and_then(Value::as_u64), Some(1));
	assert_eq!(personalization.pointer("/scope_correct_count").and_then(Value::as_u64), Some(1));
	assert_eq!(stage_job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(relation_job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(delete_job.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		delete_job.pointer("/evolution/selected_tombstone_evidence/0").and_then(Value::as_str),
		Some("delete-tombstone")
	);
	assert_eq!(
		delete_job.pointer("/evolution/selected_invalidation_evidence/0").and_then(Value::as_str),
		Some("delete-tombstone")
	);
	assert_eq!(core_fallback.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(stale_core.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(scheduled_weekly.pointer("/status").and_then(Value::as_str), Some("pass"));
	assert_eq!(
		scheduled_weekly.pointer("/scheduled_memory/trace_coverage").and_then(Value::as_f64),
		Some(1.0)
	);
	assert_eq!(
		stage_job.pointer("/trace_explainability/failure_stage").and_then(Value::as_str),
		Some("rerank.score")
	);
	assert!(support::array_contains_str(stage_job, "/produced_evidence", "stage-target")?);

	Ok(())
}
