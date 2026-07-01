use color_eyre::Result;
use serde_json::Value;

use crate::support;

pub(super) fn assert_production_ops_operational_evidence(report: &Value) -> Result<()> {
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

	let tiers = support::array_at(report, "/operational_evidence/tiers")?;
	let local_fixture = support::find_by_field(tiers, "/tier", "local_fixture")?;
	let public_proxy_tier = support::find_by_field(tiers, "/tier", "public_proxy")?;
	let private_corpus = support::find_by_field(tiers, "/tier", "private_corpus")?;
	let provider_backed = support::find_by_field(tiers, "/tier", "provider_backed")?;

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
