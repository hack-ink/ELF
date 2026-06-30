mod recovery_report;
mod resources;
mod tags;
mod tiers;
mod timings;

use crate::{
	CorpusProfile, JobReport, OPERATIONAL_EVIDENCE_SCHEMA, OperationalEvidenceReport, RealWorldJob,
};

const OPERATIONAL_EVIDENCE_TIERS: &[&str] =
	&["local_fixture", "public_proxy", "private_corpus", "provider_backed"];

pub(super) fn operational_evidence_report(
	jobs: &[RealWorldJob],
	reports: &[JobReport],
) -> OperationalEvidenceReport {
	let paired = jobs.iter().zip(reports.iter()).collect::<Vec<_>>();
	let tiers = OPERATIONAL_EVIDENCE_TIERS
		.iter()
		.map(|tier| tiers::operational_evidence_tier_report(tier, paired.as_slice()))
		.collect::<Vec<_>>();
	let private_tier = tiers.iter().find(|tier| tier.tier == "private_corpus");
	let provider_tier = tiers.iter().find(|tier| tier.tier == "provider_backed");
	let private_corpus_pass_claim_allowed =
		private_tier.is_some_and(|tier| tier.pass_claim_allowed);
	let provider_backed_pass_claim_allowed =
		provider_tier.is_some_and(|tier| tier.pass_claim_allowed);
	let missing_private_provider_inputs_are_typed_blockers = private_tier
		.is_some_and(tiers::operational_tier_has_typed_blocker)
		&& provider_tier.is_some_and(tiers::operational_tier_has_typed_blocker);

	OperationalEvidenceReport {
		schema: OPERATIONAL_EVIDENCE_SCHEMA.to_string(),
		tiers,
		latency: timings::operational_latency_report(reports),
		cost: timings::operational_cost_summary(reports),
		resource: resources::operational_resource_summary(paired.as_slice()),
		cold_start_restore_rebuild: resources::operational_cold_start_restore_rebuild(
			paired.as_slice(),
		),
		authority_recovery: recovery_report::operational_authority_recovery(reports),
		missing_private_provider_inputs_are_typed_blockers,
		private_corpus_pass_claim_allowed,
		provider_backed_pass_claim_allowed,
		claim_boundary: "Operational evidence tiers are separate: local fixture and public-proxy passes do not prove private-corpus or provider-backed production quality.".to_string(),
	}
}

pub(super) fn operational_evidence_tier(job: &RealWorldJob) -> &'static str {
	if tags::job_has_tag(job, "provider_backed") {
		"provider_backed"
	} else if tags::job_has_tag(job, "private_corpus")
		|| matches!(job.corpus.profile, CorpusProfile::PrivateSanitized)
	{
		"private_corpus"
	} else if tags::job_has_tag(job, "public_proxy") {
		"public_proxy"
	} else {
		"local_fixture"
	}
}
