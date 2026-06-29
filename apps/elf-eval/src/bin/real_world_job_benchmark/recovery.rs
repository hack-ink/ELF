use super::*;

pub(super) const REQUIRED_AUTHORITY_PLANES: [&str; 7] =
	["source", "journal", "memory", "knowledge", "proposal", "trace", "audit"];

pub(super) fn recovery_drill_succeeded(drill: &AuthorityRecoveryDrillArtifact) -> bool {
	drill.backup_pitr.restored
		&& drill.degraded_read.source_of_truth_visible
		&& recovery_measurement_met(&drill.rpo)
		&& recovery_measurement_met(&drill.rto)
		&& recovery_authority_record_counts_succeeded(drill)
		&& recovery_outbox_replay_succeeded(&drill.outbox_replay)
		&& recovery_qdrant_rebuild_succeeded(&drill.qdrant_rebuild)
		&& recovery_migration_repair_succeeded(&drill.migration_repair)
		&& recovery_dead_letter_succeeded(&drill.dead_letter)
}

pub(super) fn recovery_measurement_met(measurement: &RecoveryMeasurement) -> bool {
	measurement.measured_seconds <= measurement.target_seconds
}

fn recovery_authority_record_counts_succeeded(drill: &AuthorityRecoveryDrillArtifact) -> bool {
	let present_planes = drill
		.authority_record_counts
		.iter()
		.map(|count| count.plane.as_str())
		.collect::<BTreeSet<_>>();

	REQUIRED_AUTHORITY_PLANES.iter().all(|plane| present_planes.contains(*plane))
		&& drill.authority_record_counts.iter().all(authority_record_count_succeeded)
}

fn authority_record_count_succeeded(count: &AuthorityRecordCount) -> bool {
	authority_record_count_balanced(count)
		&& count.source_refs_preserved
		&& count.lifecycle_history_preserved
}

pub(super) fn authority_record_count_balanced(count: &AuthorityRecordCount) -> bool {
	count.before_count == count.after_count
}

pub(super) fn recovery_outbox_replay_succeeded(replay: &RecoveryOutboxReplay) -> bool {
	replay.idempotent && replay.duplicate_write_count == 0
}

pub(super) fn recovery_qdrant_rebuild_succeeded(rebuild: &RecoveryQdrantRebuild) -> bool {
	rebuild.complete && rebuild.missing_vector_count == 0 && rebuild.error_count == 0
}

pub(super) fn recovery_migration_repair_succeeded(repair: &RecoveryMigrationRepair) -> bool {
	repair.applied
}

pub(super) fn recovery_dead_letter_succeeded(dead_letter: &RecoveryDeadLetterHandling) -> bool {
	dead_letter.handled_count >= dead_letter.dead_letter_count
}
