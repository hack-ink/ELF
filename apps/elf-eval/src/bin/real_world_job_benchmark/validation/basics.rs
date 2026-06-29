use crate::validation::{self, BTreeSet, Path, RealWorldJob, Result, eyre};

pub(super) fn validate_job_identity(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.job_id.trim().is_empty()
		|| job.suite.trim().is_empty()
		|| job.title.trim().is_empty()
		|| job.corpus.corpus_id.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete job identity.", path.display()));
	}

	for tag in &job.tags {
		if tag.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty tag.", path.display()));
		}
	}

	if let Some(adapter_response) = &job.corpus.adapter_response
		&& adapter_response.adapter_id.as_deref().is_some_and(str::is_empty)
	{
		return Err(eyre::eyre!("{} has an empty adapter_response adapter_id.", path.display()));
	}

	Ok(())
}

pub(super) fn validate_corpus_items(job: &RealWorldJob, path: &Path) -> Result<()> {
	let mut evidence_ids = BTreeSet::new();

	for item in &job.corpus.items {
		if item.evidence_id.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} has a corpus item with an empty evidence_id.",
				path.display()
			));
		}
		if item.kind.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} has corpus item {} with an empty kind.",
				path.display(),
				item.evidence_id
			));
		}
		if item.text.is_none() && item.local_ref.is_none() {
			return Err(eyre::eyre!(
				"{} corpus item {} must provide text or local_ref.",
				path.display(),
				item.evidence_id
			));
		}
		if !item.source_ref.is_object() {
			return Err(eyre::eyre!(
				"{} corpus item {} must provide an object source_ref.",
				path.display(),
				item.evidence_id
			));
		}

		if let Some(created_at) = &item.created_at {
			validation::validate_optional_rfc3339(created_at, path, item.evidence_id.as_str())?;
		}

		evidence_ids.insert(item.evidence_id.clone());
	}
	for trap in &job.negative_traps {
		if trap.trap_id.trim().is_empty() || trap.trap_type.trim().is_empty() {
			return Err(eyre::eyre!("{} has an incomplete negative trap.", path.display()));
		}

		for evidence_id in &trap.evidence_ids {
			validation::ensure_known_evidence(path, &evidence_ids, evidence_id)?;
		}
	}

	Ok(())
}

pub(super) fn validate_timeline(job: &RealWorldJob, path: &Path) -> Result<()> {
	let evidence_ids = validation::corpus_evidence_ids(job);

	for event in &job.timeline {
		if event.event_id.trim().is_empty()
			|| event.actor.trim().is_empty()
			|| event.action.trim().is_empty()
			|| event.summary.trim().is_empty()
		{
			return Err(eyre::eyre!("{} has an incomplete timeline event.", path.display()));
		}

		validation::validate_required_rfc3339(event.ts.as_str(), path, event.event_id.as_str())?;

		for evidence_id in &event.evidence_ids {
			validation::ensure_known_evidence(path, &evidence_ids, evidence_id)?;
		}
	}

	Ok(())
}

pub(super) fn validate_prompt(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.prompt.role.trim().is_empty()
		|| job.prompt.content.trim().is_empty()
		|| job.prompt.job_mode.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete prompt.", path.display()));
	}

	for constraint in &job.prompt.constraints {
		if constraint.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty prompt constraint.", path.display()));
		}
	}

	Ok(())
}

pub(super) fn validate_expected_answer(job: &RealWorldJob, path: &Path) -> Result<()> {
	if job.expected_answer.answer_type.trim().is_empty() {
		return Err(eyre::eyre!("{} has an empty expected answer type.", path.display()));
	}

	for claim in &job.expected_answer.must_include {
		if claim.text().trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty expected claim.", path.display()));
		}
	}
	for claim in &job.expected_answer.must_not_include {
		if claim.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty forbidden claim.", path.display()));
		}
	}
	for phrase in &job.expected_answer.accepted_alternates {
		if phrase.is_null() {
			return Err(eyre::eyre!("{} has a null accepted alternate.", path.display()));
		}
	}

	Ok(())
}

pub(super) fn validate_required_evidence(job: &RealWorldJob, path: &Path) -> Result<()> {
	let evidence_ids = validation::corpus_evidence_ids(job);
	let corpus_text = validation::corpus_text_by_id(job);

	for evidence in &job.required_evidence {
		if evidence.claim_id.trim().is_empty() || evidence.requirement.trim().is_empty() {
			return Err(eyre::eyre!("{} has incomplete required evidence.", path.display()));
		}

		validation::ensure_known_evidence(path, &evidence_ids, evidence.evidence_id.as_str())?;

		if evidence.quote.is_none() && evidence.selector.is_none() {
			return Err(eyre::eyre!(
				"{} required evidence {} must provide quote or selector.",
				path.display(),
				evidence.evidence_id
			));
		}

		if let Some(quote) = &evidence.quote
			&& let Some(text) = corpus_text.get(evidence.evidence_id.as_str())
			&& !text.contains(quote)
		{
			return Err(eyre::eyre!(
				"{} required evidence quote for {} is not present in corpus text.",
				path.display(),
				evidence.evidence_id
			));
		}
	}
	for (claim_id, link) in &job.expected_answer.evidence_links {
		if claim_id.trim().is_empty() {
			return Err(eyre::eyre!("{} has an empty evidence link claim id.", path.display()));
		}

		for evidence_id in link.ids() {
			validation::ensure_known_evidence(path, &evidence_ids, evidence_id.as_str())?;
		}
	}

	Ok(())
}
