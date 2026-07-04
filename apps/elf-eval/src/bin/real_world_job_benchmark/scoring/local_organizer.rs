use crate::{
	LocalOrganizerJobReport, RealWorldJob, formatting,
	scoring::{self, answers},
};

pub(super) fn local_organizer_job_report(job: &RealWorldJob) -> Option<LocalOrganizerJobReport> {
	let answer = answers::produced_answer(job);
	let fixture = job.corpus.adapter_response.as_ref()?.local_organizer.as_ref()?;
	let escalation_rate = scoring::ratio_or(
		fixture.completed_escalation_count,
		fixture.required_escalation_count,
		1.0,
	);

	Some(LocalOrganizerJobReport {
		model_tier: fixture.model_tier.clone(),
		model_profile_id: fixture.model_profile_id.clone(),
		runtime_id: fixture.runtime_id.clone(),
		runtime_commit: fixture.runtime_commit.clone(),
		reproducibility_provenance: fixture.reproducibility_provenance.clone(),
		extraction_f1: fixture.extraction_f1.map(crate::scoring::round3),
		extraction_f1_blocker: fixture.extraction_f1_blocker.clone(),
		json_schema_valid: fixture.json_schema_valid,
		citation_source_ref_coverage: formatting::round3(fixture.citation_source_ref_coverage),
		unsupported_claim_rate: formatting::round3(fixture.unsupported_claim_rate),
		stale_correction_delete_score: formatting::round3(fixture.stale_correction_delete_score),
		source_mutation_count: fixture.source_mutation_count,
		silent_memory_authority_mutation_count: fixture.silent_memory_authority_mutation_count,
		required_escalation_count: fixture.required_escalation_count,
		completed_escalation_count: fixture.completed_escalation_count,
		escalation_rate,
		latency_ms: answer.latency_ms.map(crate::scoring::round3),
		cost: answer.cost.clone(),
		resource_footprint: fixture.resource_footprint.clone(),
	})
}

pub(super) fn schema_failures(local_organizer: Option<&LocalOrganizerJobReport>) -> usize {
	local_organizer.map_or(0, |report| usize::from(!report.json_schema_valid))
}

pub(super) fn provenance_failures(local_organizer: Option<&LocalOrganizerJobReport>) -> usize {
	local_organizer.map_or(0, |report| usize::from(report.citation_source_ref_coverage < 1.0))
}

pub(super) fn unsupported_claim_failures(
	local_organizer: Option<&LocalOrganizerJobReport>,
) -> usize {
	local_organizer.map_or(0, |report| usize::from(report.unsupported_claim_rate > 0.0))
}

pub(super) fn lifecycle_failures(local_organizer: Option<&LocalOrganizerJobReport>) -> usize {
	local_organizer.map_or(0, |report| usize::from(report.stale_correction_delete_score < 1.0))
}

pub(super) fn authority_mutations(local_organizer: Option<&LocalOrganizerJobReport>) -> usize {
	local_organizer.map_or(0, |report| {
		report.source_mutation_count + report.silent_memory_authority_mutation_count
	})
}

pub(super) fn escalation_failures(local_organizer: Option<&LocalOrganizerJobReport>) -> usize {
	local_organizer.map_or(0, |report| {
		usize::from(
			report.completed_escalation_count < report.required_escalation_count
				|| (report.model_tier == "L2" && report.required_escalation_count == 0),
		)
	})
}
