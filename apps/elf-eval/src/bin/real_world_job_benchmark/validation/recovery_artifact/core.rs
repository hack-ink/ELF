use crate::validation::{
	self, AUTHORITY_RECOVERY_DRILL_SCHEMA, AuthorityRecoveryDrillArtifact, BTreeSet, Path, Result,
	eyre,
	recovery_artifact::{checks, measurements, records},
};

pub(in crate::validation) fn validate_authority_recovery_drill_artifact(
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

	validation::validate_optional_rfc3339(&drill.generated_at, path, drill.drill_id.as_str())?;
	checks::validate_recovery_topology(&drill.topology, path, drill.drill_id.as_str())?;
	checks::validate_recovery_backup_pitr(&drill.backup_pitr, path, evidence_ids)?;
	checks::validate_recovery_degraded_read(&drill.degraded_read, path, evidence_ids)?;
	measurements::validate_recovery_measurement("rpo", &drill.rpo, path, evidence_ids)?;
	measurements::validate_recovery_measurement("rto", &drill.rto, path, evidence_ids)?;
	records::validate_recovery_authority_record_counts(drill, path, evidence_ids)?;
	checks::validate_recovery_outbox_replay(&drill.outbox_replay, path, evidence_ids)?;
	checks::validate_recovery_qdrant_rebuild(&drill.qdrant_rebuild, path, evidence_ids)?;
	checks::validate_recovery_migration_repair(&drill.migration_repair, path, evidence_ids)?;
	checks::validate_recovery_dead_letter(&drill.dead_letter, path, evidence_ids)?;

	validate_recovery_failure_injections(drill, path, evidence_ids)?;

	if drill.failure_injections.is_empty() {
		return Err(eyre::eyre!(
			"{} authority recovery drill {} must include failure injection evidence.",
			path.display(),
			drill.drill_id
		));
	}

	Ok(())
}

fn validate_recovery_failure_injections(
	drill: &AuthorityRecoveryDrillArtifact,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
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

		validation::validate_optional_rfc3339(
			&injection.started_at,
			path,
			injection.injection_id.as_str(),
		)?;
		validation::validate_optional_rfc3339(
			&injection.completed_at,
			path,
			injection.injection_id.as_str(),
		)?;
		validation::ensure_known_evidence_refs(path, evidence_ids, &injection.evidence_refs)?;
	}

	Ok(())
}
