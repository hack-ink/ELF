use super::*;

pub(super) fn validate_page_artifact(
	page: &DerivedPageArtifact,
	path: &Path,
	evidence_ids: &BTreeSet<String>,
	event_ids: &BTreeSet<String>,
) -> Result<()> {
	if page.page_id.trim().is_empty()
		|| page.page_type.trim().is_empty()
		|| page.title.trim().is_empty()
	{
		return Err(eyre::eyre!("{} has an incomplete derived page.", path.display()));
	}

	for section in &page.sections {
		if section.section_id.trim().is_empty()
			|| section.heading.trim().is_empty()
			|| section.role.trim().is_empty()
			|| section.content.trim().is_empty()
		{
			return Err(eyre::eyre!(
				"{} page {} has an incomplete section.",
				path.display(),
				page.page_id
			));
		}

		for evidence_id in &section.evidence_ids {
			ensure_known_evidence(path, evidence_ids, evidence_id)?;
		}
		for event_id in &section.timeline_event_ids {
			ensure_known_event(path, event_ids, event_id)?;
		}
	}
	for backlink in &page.backlinks {
		if backlink.trim().is_empty() {
			return Err(eyre::eyre!(
				"{} page {} has an empty backlink.",
				path.display(),
				page.page_id
			));
		}
	}
	for finding in &page.lint_findings {
		if finding.finding_id.trim().is_empty()
			|| finding.finding_type.trim().is_empty()
			|| finding.severity.trim().is_empty()
			|| finding.text.trim().is_empty()
		{
			return Err(eyre::eyre!(
				"{} page {} has an incomplete lint finding.",
				path.display(),
				page.page_id
			));
		}

		for evidence_id in &finding.evidence_ids {
			ensure_known_evidence(path, evidence_ids, evidence_id)?;
		}
	}

	if let Some(rebuild) = &page.rebuild
		&& (rebuild.first_hash.trim().is_empty() || rebuild.second_hash.trim().is_empty())
	{
		return Err(eyre::eyre!(
			"{} page {} has an incomplete rebuild record.",
			path.display(),
			page.page_id
		));
	}
	if let Some(diff) = &page.page_version_diff {
		if !diff.is_object() {
			return Err(eyre::eyre!(
				"{} page {} previous-version diff must be a JSON object.",
				path.display(),
				page.page_id
			));
		}
		if diff.get("schema").and_then(Value::as_str) != Some("elf.knowledge_page.version_diff/v1")
		{
			return Err(eyre::eyre!(
				"{} page {} previous-version diff has an unexpected schema.",
				path.display(),
				page.page_id
			));
		}
	}

	Ok(())
}
