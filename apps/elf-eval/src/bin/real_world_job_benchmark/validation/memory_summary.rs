use super::*;

pub(super) fn validate_memory_summary_artifact(
	summary: &MemorySummaryArtifact,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if summary.summary_id.trim().is_empty()
		|| summary.contract_schema != "elf.memory_summary/v1"
		|| summary.generated_at.trim().is_empty()
		|| summary.tenant_id.trim().is_empty()
		|| summary.project_id.trim().is_empty()
		|| summary.agent_id.trim().is_empty()
		|| summary.read_profile.trim().is_empty()
		|| summary.entries.is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete memory summary.", path.display()));
	}

	validate_optional_rfc3339(&summary.generated_at, path, summary.summary_id.as_str())?;

	for entry in &summary.entries {
		validate_memory_summary_entry(entry, path, evidence_ids)?;
	}

	validate_memory_summary_source_trace(&summary.source_trace, path, evidence_ids)?;

	Ok(())
}

pub(super) fn validate_memory_summary_source_trace(
	trace: &MemorySummarySourceTrace,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	for item in trace
		.selected_source_refs
		.iter()
		.chain(trace.dropped_source_refs.iter())
		.chain(trace.stale_source_refs.iter())
		.chain(trace.superseded_source_refs.iter())
		.chain(trace.tombstone_source_refs.iter())
	{
		if item.evidence_id.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty memory summary trace item.", path.display()));
		}

		ensure_known_evidence(path, evidence_ids, item.evidence_id.as_str())?;
	}
	for flag in &trace.unsupported_claim_flags {
		if !flag.is_object() {
			return Err(eyre::eyre!(
				"{} memory summary source-trace unsupported-claim flags must be JSON objects.",
				path.display()
			));
		}
	}

	Ok(())
}

fn validate_memory_summary_entry(
	entry: &MemorySummaryEntry,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if entry.entry_id.trim().is_empty()
		|| entry.category.trim().is_empty()
		|| entry.text.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete memory summary entry.", path.display()));
	}
	if !is_memory_summary_category(entry.category.as_str()) {
		return Err(eyre::eyre!(
			"{} has unknown memory summary category {}.",
			path.display(),
			entry.category
		));
	}
	if !is_memory_summary_freshness_status(entry.freshness.status.as_str()) {
		return Err(eyre::eyre!(
			"{} has unknown memory summary freshness status {}.",
			path.display(),
			entry.freshness.status
		));
	}
	if !is_memory_summary_rationale_decision(entry.rationale.decision.as_str()) {
		return Err(eyre::eyre!(
			"{} has unknown memory summary rationale decision {}.",
			path.display(),
			entry.rationale.decision
		));
	}

	for evidence_id in &entry.source_refs {
		ensure_known_evidence(path, evidence_ids, evidence_id)?;
	}
	for evidence_id in &entry.freshness.tombstone_refs {
		ensure_known_evidence(path, evidence_ids, evidence_id)?;
	}
	for flag in &entry.unsupported_claim_flags {
		if !flag.is_object() {
			return Err(eyre::eyre!(
				"{} memory summary unsupported-claim flags must be JSON objects.",
				path.display()
			));
		}
	}

	validate_optional_summary_time(
		path,
		entry.freshness.observed_at.as_deref(),
		entry.entry_id.as_str(),
	)?;
	validate_optional_summary_time(
		path,
		entry.freshness.valid_from.as_deref(),
		entry.entry_id.as_str(),
	)?;
	validate_optional_summary_time(
		path,
		entry.freshness.valid_to.as_deref(),
		entry.entry_id.as_str(),
	)?;
	validate_optional_summary_time(
		path,
		entry.freshness.last_confirmed_at.as_deref(),
		entry.entry_id.as_str(),
	)?;

	Ok(())
}
