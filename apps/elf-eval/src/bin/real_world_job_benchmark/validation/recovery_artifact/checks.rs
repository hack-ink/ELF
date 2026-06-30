use crate::validation::{
	self, BTreeSet, Path, RecoveryBackupPitr, RecoveryDeadLetterHandling, RecoveryDegradedRead,
	RecoveryDrillTopology, RecoveryMigrationRepair, RecoveryOutboxReplay, RecoveryQdrantRebuild,
	Result, eyre,
};

pub(in crate::validation::recovery_artifact) fn validate_recovery_topology(
	topology: &RecoveryDrillTopology,
	path: &Path,
	drill_id: &str,
) -> Result<()> {
	if topology.authority_store.trim().is_empty()
		|| topology.derived_indexes.is_empty()
		|| topology.failover.trim().is_empty()
	{
		return Err(eyre::eyre!(
			"{} authority recovery drill {} has incomplete topology.",
			path.display(),
			drill_id
		));
	}

	Ok(())
}

pub(in crate::validation::recovery_artifact) fn validate_recovery_backup_pitr(
	backup_pitr: &RecoveryBackupPitr,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if backup_pitr.backup_ref.trim().is_empty()
		|| backup_pitr.pitr_target.trim().is_empty()
		|| backup_pitr.evidence_refs.is_empty()
		|| !backup_pitr.restored
	{
		return Err(eyre::eyre!("{} has incomplete backup/PITR drill evidence.", path.display()));
	}

	validation::validate_optional_rfc3339(
		&backup_pitr.pitr_target,
		path,
		backup_pitr.backup_ref.as_str(),
	)?;

	validation::ensure_known_evidence_refs(path, evidence_ids, &backup_pitr.evidence_refs)
}

pub(in crate::validation::recovery_artifact) fn validate_recovery_degraded_read(
	degraded_read: &RecoveryDegradedRead,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if degraded_read.unavailable_labels.is_empty() || degraded_read.evidence_refs.is_empty() {
		return Err(eyre::eyre!("{} has incomplete degraded-read drill evidence.", path.display()));
	}
	if !degraded_read.source_of_truth_visible {
		return Err(eyre::eyre!(
			"{} has hidden source-of-truth records during degraded read.",
			path.display()
		));
	}

	validation::ensure_known_evidence_refs(path, evidence_ids, &degraded_read.evidence_refs)
}

pub(in crate::validation::recovery_artifact) fn validate_recovery_outbox_replay(
	replay: &RecoveryOutboxReplay,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if replay.evidence_refs.is_empty() || !validation::recovery_outbox_replay_succeeded(replay) {
		return Err(eyre::eyre!("{} has incomplete outbox replay drill evidence.", path.display()));
	}

	validation::ensure_known_evidence_refs(path, evidence_ids, &replay.evidence_refs)
}

pub(in crate::validation::recovery_artifact) fn validate_recovery_qdrant_rebuild(
	rebuild: &RecoveryQdrantRebuild,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if rebuild.evidence_refs.is_empty() || !validation::recovery_qdrant_rebuild_succeeded(rebuild) {
		return Err(eyre::eyre!(
			"{} has incomplete Qdrant rebuild drill evidence.",
			path.display()
		));
	}

	validation::ensure_known_evidence_refs(path, evidence_ids, &rebuild.evidence_refs)
}

pub(in crate::validation::recovery_artifact) fn validate_recovery_migration_repair(
	repair: &RecoveryMigrationRepair,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if repair.evidence_refs.is_empty() || !validation::recovery_migration_repair_succeeded(repair) {
		return Err(eyre::eyre!(
			"{} has incomplete migration repair drill evidence.",
			path.display()
		));
	}

	validation::ensure_known_evidence_refs(path, evidence_ids, &repair.evidence_refs)
}

pub(in crate::validation::recovery_artifact) fn validate_recovery_dead_letter(
	dead_letter: &RecoveryDeadLetterHandling,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if dead_letter.evidence_refs.is_empty()
		|| !validation::recovery_dead_letter_succeeded(dead_letter)
	{
		return Err(eyre::eyre!(
			"{} has incomplete dead-letter handling drill evidence.",
			path.display()
		));
	}

	validation::ensure_known_evidence_refs(path, evidence_ids, &dead_letter.evidence_refs)
}
