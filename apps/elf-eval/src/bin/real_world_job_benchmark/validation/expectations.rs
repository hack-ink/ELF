use crate::validation::{
	self, BTreeSet, EvolutionConflict, Path, RealWorldJob, Result, TemporalValidity, TypedStatus,
	UpdateRationale, eyre,
};

pub(super) fn validate_memory_evolution(job: &RealWorldJob, path: &Path) -> Result<()> {
	let Some(evolution) = &job.memory_evolution else {
		return Ok(());
	};
	let evidence_ids = validation::corpus_evidence_ids(job);
	let trap_ids =
		job.negative_traps.iter().map(|trap| trap.trap_id.as_str()).collect::<BTreeSet<_>>();

	for evidence_id in evolution
		.current_evidence_ids
		.iter()
		.chain(evolution.historical_evidence_ids.iter())
		.chain(evolution.tombstone_evidence_ids.iter())
		.chain(evolution.invalidation_evidence_ids.iter())
	{
		validation::ensure_known_evidence(path, &evidence_ids, evidence_id)?;
	}
	for trap_id in &evolution.stale_trap_ids {
		if !trap_ids.contains(trap_id.as_str()) {
			return Err(eyre::eyre!(
				"{} job {} references unknown stale trap id {}.",
				path.display(),
				job.job_id,
				trap_id
			));
		}
	}
	for conflict in &evolution.conflicts {
		validate_evolution_conflict(path, &evidence_ids, conflict)?;
	}

	if let Some(rationale) = &evolution.update_rationale {
		validate_update_rationale(path, &evidence_ids, rationale)?;
	}
	if let Some(temporal) = &evolution.temporal_validity {
		validate_temporal_validity(job, path, temporal)?;
	}

	Ok(())
}

pub(super) fn validate_memory_summary_expectation(job: &RealWorldJob, path: &Path) -> Result<()> {
	let Some(summary) = &job.memory_summary else {
		if job.suite == "memory_summary" && job.encoding.status.is_none() {
			return Err(eyre::eyre!(
				"{} memory_summary jobs must provide memory_summary expectations.",
				path.display()
			));
		}

		return Ok(());
	};

	for category in &summary.required_categories {
		if !validation::is_memory_summary_category(category.as_str()) {
			return Err(eyre::eyre!(
				"{} memory_summary expectation references unknown category {}.",
				path.display(),
				category
			));
		}
	}

	Ok(())
}

pub(super) fn validate_proactive_brief_expectation(job: &RealWorldJob, path: &Path) -> Result<()> {
	let Some(brief) = &job.proactive_brief else {
		if job.suite == "proactive_brief" && job.encoding.status.is_none() {
			return Err(eyre::eyre!(
				"{} proactive_brief jobs must provide proactive_brief expectations.",
				path.display()
			));
		}

		return Ok(());
	};

	for kind in &brief.required_suggestion_kinds {
		if !validation::is_proactive_suggestion_kind(kind.as_str()) {
			return Err(eyre::eyre!(
				"{} proactive_brief expectation references unknown suggestion kind {}.",
				path.display(),
				kind
			));
		}
	}

	Ok(())
}

pub(super) fn validate_scheduled_memory_expectation(job: &RealWorldJob, path: &Path) -> Result<()> {
	let Some(scheduled) = &job.scheduled_memory else {
		if job.suite == "scheduled_memory" && job.encoding.status.is_none() {
			return Err(eyre::eyre!(
				"{} scheduled_memory jobs must provide scheduled_memory expectations.",
				path.display()
			));
		}

		return Ok(());
	};

	for kind in &scheduled.required_task_kinds {
		if !validation::is_scheduled_task_kind(kind.as_str()) {
			return Err(eyre::eyre!(
				"{} scheduled_memory expectation references unknown task kind {}.",
				path.display(),
				kind
			));
		}
	}

	Ok(())
}

pub(super) fn validate_work_continuity_expectation(job: &RealWorldJob, path: &Path) -> Result<()> {
	let Some(work_continuity) = &job.work_continuity else {
		if job.suite == "work_continuity" && job.encoding.status.is_none() {
			return Err(eyre::eyre!(
				"{} work_continuity jobs must provide work_continuity expectations.",
				path.display()
			));
		}

		return Ok(());
	};
	let evidence_ids = validation::corpus_evidence_ids(job);

	for value in work_continuity
		.required_reset_resume_entry_ids
		.iter()
		.chain(work_continuity.required_rejected_option_ids.iter())
		.chain(work_continuity.required_explicit_next_step_ids.iter())
		.chain(work_continuity.required_inferred_next_step_ids.iter())
		.chain(work_continuity.required_redaction_marker_ids.iter())
		.chain(work_continuity.required_janitor_candidate_ids.iter())
	{
		if value.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} work_continuity expectations contain an empty required id.",
				path.display()
			));
		}
	}
	for evidence_ref in work_continuity
		.required_decision_rationale_evidence_ids
		.iter()
		.chain(work_continuity.required_handoff_source_ref_ids.iter())
	{
		validation::ensure_known_evidence(path, &evidence_ids, evidence_ref)?;
	}

	Ok(())
}

fn validate_evolution_conflict(
	path: &Path,
	evidence_ids: &BTreeSet<String>,
	conflict: &EvolutionConflict,
) -> Result<()> {
	if conflict.conflict_id.trim().is_empty() || conflict.claim_id.trim().is_empty() {
		return Err(eyre::eyre!("{} has an incomplete evolution conflict.", path.display()));
	}

	validation::ensure_known_evidence(path, evidence_ids, conflict.current_evidence_id.as_str())?;
	validation::ensure_known_evidence(
		path,
		evidence_ids,
		conflict.historical_evidence_id.as_str(),
	)?;

	if let Some(evidence_id) = &conflict.resolved_by_evidence_id {
		validation::ensure_known_evidence(path, evidence_ids, evidence_id)?;
	}

	Ok(())
}

fn validate_update_rationale(
	path: &Path,
	evidence_ids: &BTreeSet<String>,
	rationale: &UpdateRationale,
) -> Result<()> {
	if rationale.claim_id.trim().is_empty() {
		return Err(eyre::eyre!(
			"{} has an update rationale with an empty claim_id.",
			path.display()
		));
	}

	for evidence_id in &rationale.evidence_ids {
		validation::ensure_known_evidence(path, evidence_ids, evidence_id)?;
	}

	Ok(())
}

fn validate_temporal_validity(
	job: &RealWorldJob,
	path: &Path,
	temporal: &TemporalValidity,
) -> Result<()> {
	if temporal.follow_up.as_deref().is_some_and(|follow_up| follow_up.trim().is_empty()) {
		return Err(eyre::eyre!(
			"{} job {} has an empty temporal validity follow-up.",
			path.display(),
			job.job_id
		));
	}
	if temporal.required
		&& !temporal.encoded
		&& !matches!(job.encoding.status, Some(TypedStatus::NotEncoded | TypedStatus::Blocked))
	{
		return Err(eyre::eyre!(
			"{} job {} requires temporal validity but does not declare a not_encoded or blocked encoding status.",
			path.display(),
			job.job_id
		));
	}

	Ok(())
}
