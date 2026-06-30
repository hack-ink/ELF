use crate::{
	BTreeSet, JobReport, OperationalAuthorityRecoveryReport, TypedStatus,
	recovery::{self},
};

pub(in crate::operational) fn operational_authority_recovery(
	reports: &[JobReport],
) -> OperationalAuthorityRecoveryReport {
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
