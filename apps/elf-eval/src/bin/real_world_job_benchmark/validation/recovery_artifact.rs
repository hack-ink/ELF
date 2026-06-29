use super::*;

pub(super) fn validate_authority_recovery_drill_artifact(
	drill: &AuthorityRecoveryDrillArtifact,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if drill.drill_id.trim().is_empty()
		|| drill.contract_schema != AUTHORITY_RECOVERY_DRILL_SCHEMA
		|| drill.generated_at.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete authority recovery drill.", path.display()));
	}

	validate_optional_rfc3339(&drill.generated_at, path, drill.drill_id.as_str())?;
	validate_recovery_topology(&drill.topology, path, drill.drill_id.as_str())?;
	validate_recovery_backup_pitr(&drill.backup_pitr, path, evidence_ids)?;
	validate_recovery_degraded_read(&drill.degraded_read, path, evidence_ids)?;
	validate_recovery_measurement("rpo", &drill.rpo, path, evidence_ids)?;
	validate_recovery_measurement("rto", &drill.rto, path, evidence_ids)?;
	validate_recovery_authority_record_counts(drill, path, evidence_ids)?;
	validate_recovery_outbox_replay(&drill.outbox_replay, path, evidence_ids)?;
	validate_recovery_qdrant_rebuild(&drill.qdrant_rebuild, path, evidence_ids)?;
	validate_recovery_migration_repair(&drill.migration_repair, path, evidence_ids)?;
	validate_recovery_dead_letter(&drill.dead_letter, path, evidence_ids)?;

	for injection in &drill.failure_injections {
		if injection.injection_id.trim().is_empty()
			|| injection.target.trim().is_empty()
			|| injection.fault.trim().is_empty()
			|| injection.started_at.trim().is_empty()
			|| injection.completed_at.trim().is_empty()
			|| injection.evidence_refs.is_empty()
		{
			return Err(eyre::eyre!(
				"{} authority recovery drill {} has an incomplete failure injection.",
				path.display(),
				drill.drill_id
			));
		}

		validate_optional_rfc3339(&injection.started_at, path, injection.injection_id.as_str())?;
		validate_optional_rfc3339(&injection.completed_at, path, injection.injection_id.as_str())?;
		ensure_known_evidence_refs(path, evidence_ids, &injection.evidence_refs)?;
	}

	if drill.failure_injections.is_empty() {
		return Err(eyre::eyre!(
			"{} authority recovery drill {} must include failure injection evidence.",
			path.display(),
			drill.drill_id
		));
	}

	Ok(())
}

fn validate_recovery_topology(
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

fn validate_recovery_backup_pitr(
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

	validate_optional_rfc3339(&backup_pitr.pitr_target, path, backup_pitr.backup_ref.as_str())?;

	ensure_known_evidence_refs(path, evidence_ids, &backup_pitr.evidence_refs)
}

fn validate_recovery_degraded_read(
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

	ensure_known_evidence_refs(path, evidence_ids, &degraded_read.evidence_refs)
}

fn validate_recovery_measurement(
	label: &str,
	measurement: &RecoveryMeasurement,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if !measurement.target_seconds.is_finite()
		|| !measurement.measured_seconds.is_finite()
		|| measurement.target_seconds < 0.0
		|| measurement.measured_seconds < 0.0
		|| measurement.evidence_refs.is_empty()
	{
		return Err(eyre::eyre!("{} has invalid {label} recovery measurement.", path.display()));
	}
	if !recovery_measurement_met(measurement) {
		return Err(eyre::eyre!("{} exceeded {label} recovery target.", path.display()));
	}

	ensure_known_evidence_refs(path, evidence_ids, &measurement.evidence_refs)
}

fn validate_recovery_authority_record_counts(
	drill: &AuthorityRecoveryDrillArtifact,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	let present_planes = drill
		.authority_record_counts
		.iter()
		.map(|count| count.plane.as_str())
		.collect::<BTreeSet<_>>();

	for plane in REQUIRED_AUTHORITY_PLANES {
		if !present_planes.contains(plane) {
			return Err(eyre::eyre!(
				"{} authority recovery drill {} is missing {} authority counts.",
				path.display(),
				drill.drill_id,
				plane
			));
		}
	}
	for count in &drill.authority_record_counts {
		if count.plane.trim().is_empty() || count.evidence_refs.is_empty() {
			return Err(eyre::eyre!(
				"{} authority recovery drill {} has incomplete authority record counts.",
				path.display(),
				drill.drill_id
			));
		}
		if count.before_count != count.after_count {
			return Err(eyre::eyre!(
				"{} authority recovery drill {} lost or gained {} authority records.",
				path.display(),
				drill.drill_id,
				count.plane
			));
		}
		if !count.source_refs_preserved {
			return Err(eyre::eyre!(
				"{} authority recovery drill {} did not preserve {} authority source refs.",
				path.display(),
				drill.drill_id,
				count.plane
			));
		}
		if !count.lifecycle_history_preserved {
			return Err(eyre::eyre!(
				"{} authority recovery drill {} did not preserve {} authority lifecycle history.",
				path.display(),
				drill.drill_id,
				count.plane
			));
		}

		ensure_known_evidence_refs(path, evidence_ids, &count.evidence_refs)?;
	}

	Ok(())
}

fn validate_recovery_outbox_replay(
	replay: &RecoveryOutboxReplay,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if replay.evidence_refs.is_empty() || !recovery_outbox_replay_succeeded(replay) {
		return Err(eyre::eyre!("{} has incomplete outbox replay drill evidence.", path.display()));
	}

	ensure_known_evidence_refs(path, evidence_ids, &replay.evidence_refs)
}

fn validate_recovery_qdrant_rebuild(
	rebuild: &RecoveryQdrantRebuild,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if rebuild.evidence_refs.is_empty() || !recovery_qdrant_rebuild_succeeded(rebuild) {
		return Err(eyre::eyre!(
			"{} has incomplete Qdrant rebuild drill evidence.",
			path.display()
		));
	}

	ensure_known_evidence_refs(path, evidence_ids, &rebuild.evidence_refs)
}

fn validate_recovery_migration_repair(
	repair: &RecoveryMigrationRepair,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if repair.evidence_refs.is_empty() || !recovery_migration_repair_succeeded(repair) {
		return Err(eyre::eyre!(
			"{} has incomplete migration repair drill evidence.",
			path.display()
		));
	}

	ensure_known_evidence_refs(path, evidence_ids, &repair.evidence_refs)
}

fn validate_recovery_dead_letter(
	dead_letter: &RecoveryDeadLetterHandling,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if dead_letter.evidence_refs.is_empty() || !recovery_dead_letter_succeeded(dead_letter) {
		return Err(eyre::eyre!(
			"{} has incomplete dead-letter handling drill evidence.",
			path.display()
		));
	}

	ensure_known_evidence_refs(path, evidence_ids, &dead_letter.evidence_refs)
}
