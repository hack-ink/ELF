use crate::validation::{
	BTreeMap, BTreeSet, OffsetDateTime, Path, RealWorldJob, Result, Rfc3339, eyre,
};

pub(super) fn validate_required_rfc3339(value: &str, path: &Path, id: &str) -> Result<()> {
	if OffsetDateTime::parse(value, &Rfc3339).is_err() {
		return Err(eyre::eyre!("{} has invalid RFC3339 timestamp for {}.", path.display(), id));
	}

	Ok(())
}

pub(super) fn validate_optional_rfc3339(value: &str, path: &Path, id: &str) -> Result<()> {
	if !value.trim().is_empty() {
		validate_required_rfc3339(value, path, id)?;
	}

	Ok(())
}

pub(super) fn ensure_known_evidence(
	path: &Path,
	known: &BTreeSet<String>,
	evidence_id: &str,
) -> Result<()> {
	if !known.contains(evidence_id) {
		return Err(eyre::eyre!(
			"{} references unknown evidence id {}.",
			path.display(),
			evidence_id
		));
	}

	Ok(())
}

pub(super) fn ensure_known_evidence_refs(
	path: &Path,
	evidence_ids: &BTreeSet<String>,
	refs: &[String],
) -> Result<()> {
	for evidence_ref in refs {
		ensure_known_evidence(path, evidence_ids, evidence_ref)?;
	}

	Ok(())
}

pub(super) fn ensure_known_event(
	path: &Path,
	known: &BTreeSet<String>,
	event_id: &str,
) -> Result<()> {
	if !known.contains(event_id) {
		return Err(eyre::eyre!(
			"{} references unknown timeline event id {}.",
			path.display(),
			event_id
		));
	}

	Ok(())
}

pub(super) fn validate_optional_summary_time(
	path: &Path,
	value: Option<&str>,
	id: &str,
) -> Result<()> {
	if let Some(value) = value {
		validate_optional_rfc3339(value, path, id)?;
	}

	Ok(())
}

pub(super) fn corpus_evidence_ids(job: &RealWorldJob) -> BTreeSet<String> {
	job.corpus.items.iter().map(|item| item.evidence_id.clone()).collect()
}

pub(super) fn corpus_text_by_id(job: &RealWorldJob) -> BTreeMap<&str, &str> {
	job.corpus
		.items
		.iter()
		.filter_map(|item| item.text.as_deref().map(|text| (item.evidence_id.as_str(), text)))
		.collect()
}

pub(super) fn timeline_event_ids(job: &RealWorldJob) -> BTreeSet<String> {
	job.timeline.iter().map(|event| event.event_id.clone()).collect()
}

pub(super) fn is_memory_summary_category(category: &str) -> bool {
	matches!(
		category,
		"top_of_mind"
			| "background"
			| "stale" | "superseded"
			| "tombstone"
			| "derived_project_profile"
	)
}

pub(super) fn is_memory_summary_freshness_status(status: &str) -> bool {
	matches!(
		status,
		"current"
			| "background"
			| "historical"
			| "stale" | "superseded"
			| "tombstoned"
			| "unsupported"
	)
}

pub(super) fn is_memory_summary_rationale_decision(decision: &str) -> bool {
	matches!(decision, "included" | "downgraded" | "excluded")
}

pub(super) fn is_proactive_suggestion_kind(kind: &str) -> bool {
	matches!(
		kind,
		"daily_project_brief"
			| "resume_work"
			| "stale_decision_audit"
			| "stale_plan_preference_warning"
			| "private_corpus_refresh"
	)
}

pub(super) fn is_scheduled_task_kind(kind: &str) -> bool {
	matches!(
		kind,
		"weekly_project_status_summary"
			| "stale_preference_plan_audit"
			| "stale_decision_audit"
			| "knowledge_page_refresh_suggestion"
			| "private_provider_scheduler"
	)
}

pub(super) fn is_proactive_action_decision(decision: &str) -> bool {
	matches!(decision, "recommend" | "defer" | "reject")
}
