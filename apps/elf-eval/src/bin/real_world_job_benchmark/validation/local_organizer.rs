use crate::validation::{Path, RealWorldJob, Result, eyre};

pub(super) fn validate_local_organizer_fixture(job: &RealWorldJob, path: &Path) -> Result<()> {
	let local_organizer =
		job.corpus.adapter_response.as_ref().and_then(|response| response.local_organizer.as_ref());

	if job.suite == "local_background_organizer"
		&& local_organizer.is_none()
		&& job.encoding.status.is_none()
	{
		return Err(eyre::eyre!(
			"{} local_background_organizer jobs must provide adapter_response.local_organizer.",
			path.display()
		));
	}

	let Some(local_organizer) = local_organizer else {
		return Ok(());
	};

	if local_organizer.model_tier.trim().is_empty()
		|| local_organizer.model_profile_id.trim().is_empty()
		|| local_organizer.runtime_id.trim().is_empty()
	{
		return Err(eyre::eyre!(
			"{} has an incomplete local organizer model/runtime identity.",
			path.display()
		));
	}
	if local_organizer.extraction_f1.is_none()
		&& local_organizer.extraction_f1_blocker.as_deref().is_none_or(str::is_empty)
	{
		return Err(eyre::eyre!(
			"{} local organizer fixture must report extraction_f1 or extraction_f1_blocker.",
			path.display()
		));
	}

	for (field, value) in [
		("extraction_f1", local_organizer.extraction_f1),
		("citation_source_ref_coverage", Some(local_organizer.citation_source_ref_coverage)),
		("unsupported_claim_rate", Some(local_organizer.unsupported_claim_rate)),
		("stale_correction_delete_score", Some(local_organizer.stale_correction_delete_score)),
	] {
		let Some(value) = value else {
			continue;
		};

		if !value.is_finite() || !(0.0..=1.0).contains(&value) {
			return Err(eyre::eyre!(
				"{} local organizer metric {field} must be finite and between 0.0 and 1.0.",
				path.display()
			));
		}
	}

	if local_organizer.completed_escalation_count > local_organizer.required_escalation_count {
		return Err(eyre::eyre!(
			"{} local organizer completed_escalation_count cannot exceed required_escalation_count.",
			path.display()
		));
	}
	if !local_organizer.resource_footprint.is_object() {
		return Err(eyre::eyre!(
			"{} local organizer resource_footprint must be a JSON object.",
			path.display()
		));
	}

	Ok(())
}
