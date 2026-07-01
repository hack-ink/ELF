use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(super) fn assert_production_ops_jobs(report: &Value) -> Result<()> {
	let jobs = support::array_at(report, "/jobs")?;
	let authority_recovery =
		support::find_by_field(jobs, "/job_id", "production-ops-authority-plane-recovery-001")?;
	let backfill = support::find_by_field(jobs, "/job_id", "production-ops-backfill-resume-001")?;
	let restore = support::find_by_field(jobs, "/job_id", "production-ops-restore-cold-start-001")?;
	let public_proxy =
		support::find_by_field(jobs, "/job_id", "production-ops-public-proxy-addendum-001")?;
	let private_manifest =
		support::find_by_field(jobs, "/job_id", "production-ops-private-manifest-blocked-001")?;
	let credentials =
		support::find_by_field(jobs, "/job_id", "production-ops-credential-boundary-001")?;
	let dependency =
		support::find_by_field(jobs, "/job_id", "production-ops-cold-start-dependency-001")?;

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
	assert!(support::array_at(job, "/hard_fail_hits")?.is_empty());

	Ok(())
}
