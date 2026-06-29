use crate::validation::{
	self, BTreeSet, Path, Result, WorkJournalEntryArtifact, WorkJournalNextStepArtifact,
	WorkJournalReadbackArtifact, WorkJournalWhereStoppedArtifact, eyre,
};

pub(super) fn validate_work_journal_readback_artifact(
	readback: &WorkJournalReadbackArtifact,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if readback.readback_id.trim().is_empty()
		|| readback.contract_schema != "elf.work_journal/v1"
		|| readback.generated_at.trim().is_empty()
		|| readback.session_id.trim().is_empty()
		|| readback.tenant_id.trim().is_empty()
		|| readback.project_id.trim().is_empty()
		|| readback.agent_id.trim().is_empty()
		|| readback.read_profile.trim().is_empty()
		|| readback.items.is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete Work Journal readback.", path.display()));
	}

	validation::validate_optional_rfc3339(
		&readback.generated_at,
		path,
		readback.readback_id.as_str(),
	)?;

	if readback.promotion_boundary.journal_entry_authority.trim().is_empty() {
		return Err(eyre::eyre!(
			"{} Work Journal readback {} has an incomplete promotion boundary.",
			path.display(),
			readback.readback_id
		));
	}

	for accepted_ref in &readback.promotion_boundary.accepted_refs {
		if accepted_ref.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} Work Journal readback {} has an empty accepted ref.",
				path.display(),
				readback.readback_id
			));
		}
	}
	for item in &readback.items {
		validate_work_journal_entry(item, path, evidence_ids)?;
	}

	if let Some(where_stopped) = &readback.where_stopped {
		validate_work_journal_where_stopped(where_stopped, path, evidence_ids)?;
	}

	for candidate in &readback.janitor_candidates {
		if candidate.candidate_id.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} Work Journal readback {} has an empty janitor candidate id.",
				path.display(),
				readback.readback_id
			));
		}

		for evidence_ref in &candidate.evidence_refs {
			validation::ensure_known_evidence(path, evidence_ids, evidence_ref)?;
		}
	}

	Ok(())
}

fn validate_work_journal_entry(
	entry: &WorkJournalEntryArtifact,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if entry.entry_id.trim().is_empty()
		|| entry.family.trim().is_empty()
		|| entry.title.trim().is_empty()
		|| entry.body.trim().is_empty()
		|| entry.source_refs.is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete Work Journal entry.", path.display()));
	}

	for source_ref in &entry.source_refs {
		validation::ensure_known_evidence(path, evidence_ids, source_ref)?;
	}
	for marker_id in entry
		.redaction_audit
		.required_marker_ids
		.iter()
		.chain(entry.redaction_audit.redacted_marker_ids.iter())
		.chain(entry.redaction_audit.persisted_sensitive_marker_ids.iter())
	{
		if marker_id.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} Work Journal entry {} has an empty redaction marker id.",
				path.display(),
				entry.entry_id
			));
		}
	}
	for step in entry.explicit_next_steps.iter().chain(entry.inferred_next_steps.iter()) {
		validate_work_journal_next_step(step, path, evidence_ids)?;
	}
	for option in &entry.rejected_options {
		if option.option_id.trim().is_empty() || option.text.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} Work Journal entry {} has an incomplete rejected option.",
				path.display(),
				entry.entry_id
			));
		}

		for evidence_ref in &option.evidence_refs {
			validation::ensure_known_evidence(path, evidence_ids, evidence_ref)?;
		}
	}

	Ok(())
}

fn validate_work_journal_next_step(
	step: &WorkJournalNextStepArtifact,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if step.step_id.trim().is_empty() || step.text.trim().is_empty() || step.label.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete Work Journal next step.", path.display()));
	}

	for evidence_ref in &step.evidence_refs {
		validation::ensure_known_evidence(path, evidence_ids, evidence_ref)?;
	}

	Ok(())
}

fn validate_work_journal_where_stopped(
	where_stopped: &WorkJournalWhereStoppedArtifact,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	for evidence_ref in where_stopped
		.decision_rationale_evidence_ids
		.iter()
		.chain(where_stopped.handoff_source_refs.iter())
	{
		validation::ensure_known_evidence(path, evidence_ids, evidence_ref)?;
	}
	for claim in &where_stopped.journal_only_authority_claims {
		if claim.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} has an empty Work Journal journal-only authority claim.",
				path.display()
			));
		}
	}

	Ok(())
}
