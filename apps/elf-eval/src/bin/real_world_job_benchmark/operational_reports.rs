use crate::{CostReport, Deserialize, Serialize, TypedStatus};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct OperationalEvidenceReport {
	pub(super) schema: String,
	#[serde(default)]
	pub(super) tiers: Vec<OperationalEvidenceTierReport>,
	pub(super) latency: OperationalLatencyReport,
	pub(super) cost: OperationalCostSummary,
	pub(super) resource: OperationalResourceSummary,
	pub(super) cold_start_restore_rebuild: OperationalColdStartRestoreRebuild,
	#[serde(default)]
	pub(super) authority_recovery: OperationalAuthorityRecoveryReport,
	pub(super) missing_private_provider_inputs_are_typed_blockers: bool,
	pub(super) private_corpus_pass_claim_allowed: bool,
	pub(super) provider_backed_pass_claim_allowed: bool,
	pub(super) claim_boundary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct OperationalEvidenceTierReport {
	pub(super) tier: String,
	pub(super) status: TypedStatus,
	pub(super) job_count: usize,
	pub(super) pass: usize,
	pub(super) wrong_result: usize,
	pub(super) lifecycle_fail: usize,
	pub(super) incomplete: usize,
	pub(super) blocked: usize,
	pub(super) not_encoded: usize,
	pub(super) unsupported_claim: usize,
	pub(super) mean_latency_ms: Option<f64>,
	pub(super) total_cost: Option<CostReport>,
	pub(super) resource_evidence_count: usize,
	pub(super) cold_start_evidence_count: usize,
	pub(super) restore_evidence_count: usize,
	pub(super) qdrant_rebuild_evidence_count: usize,
	pub(super) pass_claim_allowed: bool,
	#[serde(default)]
	pub(super) blocker_reasons: Vec<String>,
	#[serde(default)]
	pub(super) job_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct OperationalLatencyReport {
	pub(super) measured_job_count: usize,
	pub(super) missing_latency_job_count: usize,
	pub(super) mean_ms: Option<f64>,
	pub(super) max_ms: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct OperationalCostSummary {
	pub(super) jobs_with_cost_report: usize,
	pub(super) missing_cost_job_count: usize,
	pub(super) zero_cost_job_count: usize,
	pub(super) total: Option<CostReport>,
	pub(super) claim_boundary: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct OperationalResourceSummary {
	pub(super) resource_envelope_job_count: usize,
	pub(super) resource_envelope_pass_count: usize,
	pub(super) latency_resource_dimension_job_count: usize,
	#[serde(default)]
	pub(super) job_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct OperationalColdStartRestoreRebuild {
	pub(super) cold_start_job_count: usize,
	pub(super) cold_start_pass_count: usize,
	pub(super) restore_job_count: usize,
	pub(super) restore_pass_count: usize,
	pub(super) qdrant_rebuild_job_count: usize,
	pub(super) qdrant_rebuild_pass_count: usize,
	#[serde(default)]
	pub(super) job_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(super) struct OperationalAuthorityRecoveryReport {
	pub(super) drill_count: usize,
	pub(super) drill_pass_count: usize,
	pub(super) topology_reported_count: usize,
	pub(super) failure_injection_count: usize,
	pub(super) degraded_read_labeled_count: usize,
	pub(super) source_of_truth_visible_count: usize,
	pub(super) backup_pitr_restored_count: usize,
	pub(super) rpo_target_count: usize,
	pub(super) rpo_met_count: usize,
	pub(super) rto_target_count: usize,
	pub(super) rto_met_count: usize,
	pub(super) authority_plane_count: usize,
	pub(super) record_count_preserved_count: usize,
	pub(super) source_ref_preserved_count: usize,
	pub(super) lifecycle_history_preserved_count: usize,
	pub(super) idempotent_outbox_replay_count: usize,
	pub(super) qdrant_rebuild_complete_count: usize,
	pub(super) migration_repair_count: usize,
	pub(super) dead_letter_handled_count: usize,
	#[serde(default)]
	pub(super) job_ids: Vec<String>,
}
