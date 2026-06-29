use crate::{
	BTreeSet, CorpusProfile, JobReport, OPERATIONAL_EVIDENCE_SCHEMA,
	OperationalAuthorityRecoveryReport, OperationalColdStartRestoreRebuild, OperationalCostSummary,
	OperationalEvidenceReport, OperationalEvidenceTierReport, OperationalLatencyReport,
	OperationalResourceSummary, RealWorldJob, TypedStatus,
	formatting::round3,
	recovery::{self},
	summary::{self},
};

const OPERATIONAL_EVIDENCE_TIERS: &[&str] =
	&["local_fixture", "public_proxy", "private_corpus", "provider_backed"];

pub(super) fn operational_evidence_report(
	jobs: &[RealWorldJob],
	reports: &[JobReport],
) -> OperationalEvidenceReport {
	let paired = jobs.iter().zip(reports.iter()).collect::<Vec<_>>();
	let tiers = OPERATIONAL_EVIDENCE_TIERS
		.iter()
		.map(|tier| operational_evidence_tier_report(tier, paired.as_slice()))
		.collect::<Vec<_>>();
	let private_tier = tiers.iter().find(|tier| tier.tier == "private_corpus");
	let provider_tier = tiers.iter().find(|tier| tier.tier == "provider_backed");
	let private_corpus_pass_claim_allowed =
		private_tier.is_some_and(|tier| tier.pass_claim_allowed);
	let provider_backed_pass_claim_allowed =
		provider_tier.is_some_and(|tier| tier.pass_claim_allowed);
	let missing_private_provider_inputs_are_typed_blockers = private_tier
		.is_some_and(operational_tier_has_typed_blocker)
		&& provider_tier.is_some_and(operational_tier_has_typed_blocker);

	OperationalEvidenceReport {
		schema: OPERATIONAL_EVIDENCE_SCHEMA.to_string(),
		tiers,
		latency: operational_latency_report(reports),
		cost: operational_cost_summary(reports),
		resource: operational_resource_summary(paired.as_slice()),
		cold_start_restore_rebuild: operational_cold_start_restore_rebuild(paired.as_slice()),
		authority_recovery: operational_authority_recovery(reports),
		missing_private_provider_inputs_are_typed_blockers,
		private_corpus_pass_claim_allowed,
		provider_backed_pass_claim_allowed,
		claim_boundary: "Operational evidence tiers are separate: local fixture and public-proxy passes do not prove private-corpus or provider-backed production quality.".to_string(),
	}
}

pub(super) fn operational_evidence_tier(job: &RealWorldJob) -> &'static str {
	if job_has_tag(job, "provider_backed") {
		"provider_backed"
	} else if job_has_tag(job, "private_corpus")
		|| matches!(job.corpus.profile, CorpusProfile::PrivateSanitized)
	{
		"private_corpus"
	} else if job_has_tag(job, "public_proxy") {
		"public_proxy"
	} else {
		"local_fixture"
	}
}

fn operational_evidence_tier_report(
	tier: &str,
	paired: &[(&RealWorldJob, &JobReport)],
) -> OperationalEvidenceTierReport {
	let tier_jobs = paired
		.iter()
		.filter(|(job, _)| operational_evidence_tier(job) == tier)
		.copied()
		.collect::<Vec<_>>();
	let reports = tier_jobs.iter().map(|(_, report)| *report).collect::<Vec<_>>();
	let status = if reports.is_empty() {
		TypedStatus::NotEncoded
	} else {
		summary::aggregate_status(reports.as_slice())
	};
	let job_count = reports.len();
	let pass = reports.iter().filter(|report| report.status == TypedStatus::Pass).count();
	let wrong_result =
		reports.iter().filter(|report| report.status == TypedStatus::WrongResult).count();
	let lifecycle_fail =
		reports.iter().filter(|report| report.status == TypedStatus::LifecycleFail).count();
	let incomplete =
		reports.iter().filter(|report| report.status == TypedStatus::Incomplete).count();
	let blocked = reports.iter().filter(|report| report.status == TypedStatus::Blocked).count();
	let not_encoded = usize::from(reports.is_empty())
		+ reports.iter().filter(|report| report.status == TypedStatus::NotEncoded).count();
	let unsupported_claim =
		reports.iter().filter(|report| report.status == TypedStatus::UnsupportedClaim).count();

	OperationalEvidenceTierReport {
		tier: tier.to_string(),
		status,
		job_count,
		pass,
		wrong_result,
		lifecycle_fail,
		incomplete,
		blocked,
		not_encoded,
		unsupported_claim,
		mean_latency_ms: summary::mean_latency_for_reports(reports.as_slice()),
		total_cost: summary::total_cost_for_reports(reports.as_slice()),
		resource_evidence_count: tier_jobs
			.iter()
			.filter(|(job, _)| job_has_tag(job, "resource_envelope"))
			.count(),
		cold_start_evidence_count: tier_jobs
			.iter()
			.filter(|(job, _)| job_has_tag(job, "cold_start"))
			.count(),
		restore_evidence_count: tier_jobs
			.iter()
			.filter(|(job, _)| job_has_tag(job, "restore"))
			.count(),
		qdrant_rebuild_evidence_count: tier_jobs
			.iter()
			.filter(|(job, report)| {
				job_has_tag(job, "qdrant_rebuild") || report.qdrant_rebuild_case
			})
			.count(),
		pass_claim_allowed: job_count > 0 && status == TypedStatus::Pass,
		blocker_reasons: reports
			.iter()
			.filter(|report| report.status != TypedStatus::Pass)
			.map(|report| report.reason.clone())
			.collect(),
		job_ids: reports.iter().map(|report| report.job_id.clone()).collect(),
	}
}

fn operational_tier_has_typed_blocker(tier: &OperationalEvidenceTierReport) -> bool {
	tier.blocked + tier.incomplete + tier.not_encoded > 0 && !tier.pass_claim_allowed
}

fn operational_latency_report(reports: &[JobReport]) -> OperationalLatencyReport {
	let latencies = reports.iter().filter_map(|report| report.latency_ms).collect::<Vec<_>>();

	OperationalLatencyReport {
		measured_job_count: latencies.len(),
		missing_latency_job_count: reports.len().saturating_sub(latencies.len()),
		mean_ms: summary::mean_latency_for_values(latencies.as_slice()),
		max_ms: latencies.iter().copied().reduce(f64::max).map(round3),
	}
}

fn operational_cost_summary(reports: &[JobReport]) -> OperationalCostSummary {
	let costs = reports.iter().filter_map(|report| report.cost.as_ref()).collect::<Vec<_>>();
	let zero_cost_job_count =
		costs.iter().filter(|cost| cost.amount.is_some_and(|amount| amount == 0.0)).count();

	OperationalCostSummary {
		jobs_with_cost_report: costs.len(),
		missing_cost_job_count: reports.len().saturating_sub(costs.len()),
		zero_cost_job_count,
		total: summary::total_cost(reports),
		claim_boundary: "Fixture and local-provider zero-cost reports are execution-accounting evidence only; they do not prove hosted provider spend.".to_string(),
	}
}

fn operational_resource_summary(
	paired: &[(&RealWorldJob, &JobReport)],
) -> OperationalResourceSummary {
	let resource_jobs =
		paired.iter().filter(|(job, _)| job_has_tag(job, "resource_envelope")).collect::<Vec<_>>();
	let latency_resource_dimension_job_count = paired
		.iter()
		.filter(|(_, report)| {
			report.dimension_scores.iter().any(|score| score.dimension == "latency_resource")
		})
		.count();

	OperationalResourceSummary {
		resource_envelope_job_count: resource_jobs.len(),
		resource_envelope_pass_count: resource_jobs
			.iter()
			.filter(|(_, report)| report.status == TypedStatus::Pass)
			.count(),
		latency_resource_dimension_job_count,
		job_ids: resource_jobs.iter().map(|(_, report)| report.job_id.clone()).collect(),
	}
}

fn operational_cold_start_restore_rebuild(
	paired: &[(&RealWorldJob, &JobReport)],
) -> OperationalColdStartRestoreRebuild {
	let cold_start_jobs =
		paired.iter().filter(|(job, _)| job_has_tag(job, "cold_start")).collect::<Vec<_>>();
	let restore_jobs =
		paired.iter().filter(|(job, _)| job_has_tag(job, "restore")).collect::<Vec<_>>();
	let qdrant_rebuild_jobs = paired
		.iter()
		.filter(|(job, report)| job_has_tag(job, "qdrant_rebuild") || report.qdrant_rebuild_case)
		.collect::<Vec<_>>();
	let mut job_ids = cold_start_jobs
		.iter()
		.chain(restore_jobs.iter())
		.chain(qdrant_rebuild_jobs.iter())
		.map(|(_, report)| report.job_id.clone())
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect::<Vec<_>>();

	job_ids.sort();
	OperationalColdStartRestoreRebuild {
		cold_start_job_count: cold_start_jobs.len(),
		cold_start_pass_count: cold_start_jobs
			.iter()
			.filter(|(_, report)| report.status == TypedStatus::Pass)
			.count(),
		restore_job_count: restore_jobs.len(),
		restore_pass_count: restore_jobs
			.iter()
			.filter(|(_, report)| report.status == TypedStatus::Pass)
			.count(),
		qdrant_rebuild_job_count: qdrant_rebuild_jobs.len(),
		qdrant_rebuild_pass_count: qdrant_rebuild_jobs
			.iter()
			.filter(|(_, report)| report.status == TypedStatus::Pass)
			.count(),
		job_ids,
	}
}

fn operational_authority_recovery(reports: &[JobReport]) -> OperationalAuthorityRecoveryReport {
	let recovery_jobs =
		reports.iter().filter(|report| !report.recovery_drills.is_empty()).collect::<Vec<_>>();
	let drills =
		recovery_jobs.iter().flat_map(|report| report.recovery_drills.iter()).collect::<Vec<_>>();
	let authority_counts =
		drills.iter().flat_map(|drill| drill.authority_record_counts.iter()).collect::<Vec<_>>();
	let mut job_ids = recovery_jobs
		.iter()
		.map(|report| report.job_id.clone())
		.collect::<BTreeSet<_>>()
		.into_iter()
		.collect::<Vec<_>>();

	job_ids.sort();
	OperationalAuthorityRecoveryReport {
		drill_count: drills.len(),
		drill_pass_count: recovery_jobs
			.iter()
			.filter(|report| report.status == TypedStatus::Pass)
			.flat_map(|report| report.recovery_drills.iter())
			.filter(|drill| recovery::recovery_drill_succeeded(drill))
			.count(),
		topology_reported_count: drills
			.iter()
			.filter(|drill| !drill.topology.authority_store.trim().is_empty())
			.count(),
		failure_injection_count: drills.iter().map(|drill| drill.failure_injections.len()).sum(),
		degraded_read_labeled_count: drills
			.iter()
			.filter(|drill| !drill.degraded_read.unavailable_labels.is_empty())
			.count(),
		source_of_truth_visible_count: drills
			.iter()
			.filter(|drill| drill.degraded_read.source_of_truth_visible)
			.count(),
		backup_pitr_restored_count: drills
			.iter()
			.filter(|drill| drill.backup_pitr.restored)
			.count(),
		rpo_target_count: drills.len(),
		rpo_met_count: drills
			.iter()
			.filter(|drill| recovery::recovery_measurement_met(&drill.rpo))
			.count(),
		rto_target_count: drills.len(),
		rto_met_count: drills
			.iter()
			.filter(|drill| recovery::recovery_measurement_met(&drill.rto))
			.count(),
		authority_plane_count: authority_counts.len(),
		record_count_preserved_count: authority_counts
			.iter()
			.filter(|count| recovery::authority_record_count_balanced(count))
			.count(),
		source_ref_preserved_count: authority_counts
			.iter()
			.filter(|count| count.source_refs_preserved)
			.count(),
		lifecycle_history_preserved_count: authority_counts
			.iter()
			.filter(|count| count.lifecycle_history_preserved)
			.count(),
		idempotent_outbox_replay_count: drills
			.iter()
			.filter(|drill| recovery::recovery_outbox_replay_succeeded(&drill.outbox_replay))
			.count(),
		qdrant_rebuild_complete_count: drills
			.iter()
			.filter(|drill| recovery::recovery_qdrant_rebuild_succeeded(&drill.qdrant_rebuild))
			.count(),
		migration_repair_count: drills
			.iter()
			.filter(|drill| recovery::recovery_migration_repair_succeeded(&drill.migration_repair))
			.count(),
		dead_letter_handled_count: drills
			.iter()
			.filter(|drill| recovery::recovery_dead_letter_succeeded(&drill.dead_letter))
			.count(),
		job_ids,
	}
}

fn job_has_tag(job: &RealWorldJob, tag: &str) -> bool {
	job.tags.iter().any(|candidate| candidate == tag)
}
