use super::*;

pub(super) fn validate_proactive_brief_artifact(
	brief: &ProactiveBriefArtifact,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if brief.brief_id.trim().is_empty()
		|| brief.contract_schema != "elf.proactive_project_brief/v1"
		|| brief.generated_at.trim().is_empty()
		|| brief.tenant_id.trim().is_empty()
		|| brief.project_id.trim().is_empty()
		|| brief.agent_id.trim().is_empty()
		|| brief.read_profile.trim().is_empty()
		|| brief.brief_kind.trim().is_empty()
		|| brief.suggestions.is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete proactive brief.", path.display()));
	}

	validate_optional_rfc3339(&brief.generated_at, path, brief.brief_id.as_str())?;

	for suggestion in &brief.suggestions {
		validate_proactive_suggestion(suggestion, path, evidence_ids)?;
	}

	validate_memory_summary_source_trace(&brief.source_trace, path, evidence_ids)?;

	Ok(())
}

fn validate_proactive_suggestion(
	suggestion: &ProactiveSuggestion,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
) -> Result<()> {
	if suggestion.suggestion_id.trim().is_empty()
		|| suggestion.suggestion_kind.trim().is_empty()
		|| suggestion.title.trim().is_empty()
		|| suggestion.body.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete proactive suggestion.", path.display()));
	}
	if !is_proactive_suggestion_kind(suggestion.suggestion_kind.as_str()) {
		return Err(eyre::eyre!(
			"{} has unknown proactive suggestion kind {}.",
			path.display(),
			suggestion.suggestion_kind
		));
	}
	if !is_memory_summary_freshness_status(suggestion.freshness.status.as_str()) {
		return Err(eyre::eyre!(
			"{} has unknown proactive freshness status {}.",
			path.display(),
			suggestion.freshness.status
		));
	}
	if !is_proactive_action_decision(suggestion.action.decision.as_str()) {
		return Err(eyre::eyre!(
			"{} has unknown proactive action decision {}.",
			path.display(),
			suggestion.action.decision
		));
	}
	if suggestion.action.reason_code.trim().is_empty() || suggestion.action.reason.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has incomplete proactive action rationale.", path.display()));
	}

	for evidence_id in &suggestion.evidence_refs {
		ensure_known_evidence(path, evidence_ids, evidence_id)?;
	}
	for evidence_id in &suggestion.freshness.tombstone_refs {
		ensure_known_evidence(path, evidence_ids, evidence_id)?;
	}
	for flag in &suggestion.unsupported_claim_flags {
		if !flag.is_object() {
			return Err(eyre::eyre!(
				"{} proactive unsupported-claim flags must be JSON objects.",
				path.display()
			));
		}
	}

	validate_optional_summary_time(
		path,
		suggestion.freshness.observed_at.as_deref(),
		suggestion.suggestion_id.as_str(),
	)?;
	validate_optional_summary_time(
		path,
		suggestion.freshness.valid_from.as_deref(),
		suggestion.suggestion_id.as_str(),
	)?;
	validate_optional_summary_time(
		path,
		suggestion.freshness.valid_to.as_deref(),
		suggestion.suggestion_id.as_str(),
	)?;
	validate_optional_summary_time(
		path,
		suggestion.freshness.last_confirmed_at.as_deref(),
		suggestion.suggestion_id.as_str(),
	)?;

	Ok(())
}
