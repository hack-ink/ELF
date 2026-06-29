use crate::validation::{self, Path, RealWorldJob, Result, eyre};

pub(super) fn validate_adapter_response(job: &RealWorldJob, path: &Path) -> Result<()> {
	let Some(adapter_response) = &job.corpus.adapter_response else {
		return Ok(());
	};
	let evidence_ids = validation::corpus_evidence_ids(job);
	let event_ids = validation::timeline_event_ids(job);

	for page in &adapter_response.answer.pages {
		validation::validate_page_artifact(page, path, &evidence_ids, &event_ids)?;
	}
	for summary in &adapter_response.answer.memory_summaries {
		validation::validate_memory_summary_artifact(summary, path, &evidence_ids)?;
	}
	for brief in &adapter_response.answer.proactive_briefs {
		validation::validate_proactive_brief_artifact(brief, path, &evidence_ids)?;
	}
	for task in &adapter_response.answer.scheduled_tasks {
		validation::validate_scheduled_memory_artifact(task, path, &evidence_ids)?;
	}
	for readback in &adapter_response.answer.work_journal_readbacks {
		validation::validate_work_journal_readback_artifact(readback, path, &evidence_ids)?;
	}
	for drill in &adapter_response.answer.recovery_drills {
		validation::validate_authority_recovery_drill_artifact(drill, path, &evidence_ids)?;
	}

	if job.suite == "memory_summary"
		&& adapter_response.answer.memory_summaries.is_empty()
		&& job.encoding.status.is_none()
	{
		return Err(eyre::eyre!(
			"{} memory_summary jobs must provide adapter_response.answer.memory_summaries.",
			path.display()
		));
	}
	if job.suite == "proactive_brief"
		&& adapter_response.answer.proactive_briefs.is_empty()
		&& job.encoding.status.is_none()
	{
		return Err(eyre::eyre!(
			"{} proactive_brief jobs must provide adapter_response.answer.proactive_briefs.",
			path.display()
		));
	}
	if job.suite == "scheduled_memory"
		&& adapter_response.answer.scheduled_tasks.is_empty()
		&& job.encoding.status.is_none()
	{
		return Err(eyre::eyre!(
			"{} scheduled_memory jobs must provide adapter_response.answer.scheduled_tasks.",
			path.display()
		));
	}
	if job.suite == "work_continuity"
		&& adapter_response.answer.work_journal_readbacks.is_empty()
		&& job.encoding.status.is_none()
	{
		return Err(eyre::eyre!(
			"{} work_continuity jobs must provide adapter_response.answer.work_journal_readbacks.",
			path.display()
		));
	}

	Ok(())
}
