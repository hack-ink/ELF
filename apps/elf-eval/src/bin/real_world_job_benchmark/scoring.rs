mod answers;
mod claims;
mod consolidation;
mod counts;
mod dimensions;
mod evolution;
mod local_organizer;
mod reports;

use self::{counts::wrong_result_signal_count, evolution::update_rationale_missing_count};
use crate::{
	BTreeMap, BTreeSet, ConsolidationExecutableGapReport, ConsolidationJobReport,
	ConsolidationProposalFixture, ConsolidationProposalReport, DimensionScoreReport,
	EvolutionConflict, EvolutionJobReport, ExpectedClaim, ExpectedEvidenceReport, FailureCounts,
	JobMetrics, JobReport, JobScoring, KnowledgeJobMetrics, LocalOrganizerJobReport,
	MemoryEvolution, MemorySummaryJobMetrics, NegativeTrap, ProactiveBriefJobMetrics,
	ProducedAnswer, RealWorldJob, RequiredEvidence, RetrievalQualityReport,
	ScheduledMemoryJobMetrics, TypedStatus, UpdateRationale, Value, WorkContinuityJobMetrics,
	feature_metrics::{
		self, forbidden_diff_key_count, missed_stale_finding_count, page_usefulness_failure_count,
	},
	formatting::round3,
	operational::operational_evidence_tier,
	summary::{mean_proposal_metric, ratio_or},
};

struct FailureCountContext<'a> {
	job: &'a RealWorldJob,
	missing_claims_len: usize,
	forbidden_claims_len: usize,
	missing_evidence_len: usize,
	trap_ids_used_len: usize,
	unsupported_claims_len: usize,
	latency_violations: usize,
	evolution: Option<&'a EvolutionJobReport>,
	consolidation: Option<&'a ConsolidationJobReport>,
	local_organizer: Option<&'a LocalOrganizerJobReport>,
	knowledge: Option<&'a KnowledgeJobMetrics>,
	memory_summary: Option<&'a MemorySummaryJobMetrics>,
	proactive_brief: Option<&'a ProactiveBriefJobMetrics>,
	scheduled_memory: Option<&'a ScheduledMemoryJobMetrics>,
	work_continuity: Option<&'a WorkContinuityJobMetrics>,
}

pub(super) fn job_report(job: &RealWorldJob, scoring: JobScoring) -> JobReport {
	reports::job_report(job, scoring)
}

pub(super) fn produced_evidence_order(job: &RealWorldJob) -> Vec<String> {
	self::answers::ordered_produced_evidence_ids(self::answers::produced_answer(job))
}

pub(super) fn score_job(job: &RealWorldJob) -> JobScoring {
	let answer = self::answers::produced_answer(job);
	let produced_evidence = self::answers::produced_evidence_ids(answer);
	let trap_ids_used = self::answers::trap_ids_used(job, &produced_evidence);
	let consolidation = self::consolidation::consolidation_job_report(job);
	let local_organizer = self::local_organizer::local_organizer_job_report(job);

	if let Some(status) = job.encoding.status {
		let evolution = self::evolution::evolution_job_report(job, answer, &trap_ids_used, 0);

		return self::counts::score_declared_job(
			job,
			status,
			trap_ids_used,
			evolution,
			consolidation,
			local_organizer,
		);
	}

	let missing_claims = self::answers::missing_required_claims(job, answer);
	let forbidden_claims = self::answers::forbidden_claim_hits(job, answer);
	let missing_evidence = self::answers::missing_required_evidence(job, &produced_evidence);
	let knowledge = feature_metrics::knowledge_metrics(job, answer);
	let memory_summary = feature_metrics::memory_summary_metrics(job, answer);
	let proactive_brief = feature_metrics::proactive_brief_metrics(job, answer);
	let scheduled_memory = feature_metrics::scheduled_memory_metrics(job, answer);
	let work_continuity = feature_metrics::work_continuity_metrics(job, answer);
	let mut unsupported_claims = self::claims::unsupported_claims(job, answer);

	unsupported_claims.extend(feature_metrics::unsupported_page_claims(answer));
	unsupported_claims.extend(feature_metrics::unsupported_memory_summary_claims(job, answer));
	unsupported_claims.extend(feature_metrics::unsupported_proactive_suggestions(job, answer));
	unsupported_claims.extend(feature_metrics::unsupported_scheduled_outputs(job, answer));

	let latency_violations = self::dimensions::latency_violations(job, answer);
	let hard_fail_hits = self::claims::hard_fail_hits(job, &unsupported_claims, &trap_ids_used);
	let evolution =
		self::evolution::evolution_job_report(job, answer, &trap_ids_used, forbidden_claims.len());
	let counts = failure_counts(FailureCountContext {
		job,
		missing_claims_len: missing_claims.len(),
		forbidden_claims_len: forbidden_claims.len(),
		missing_evidence_len: missing_evidence.len(),
		trap_ids_used_len: trap_ids_used.len(),
		unsupported_claims_len: unsupported_claims.len(),
		latency_violations,
		evolution: evolution.as_ref(),
		consolidation: consolidation.as_ref(),
		local_organizer: local_organizer.as_ref(),
		knowledge: knowledge.as_ref(),
		memory_summary: memory_summary.as_ref(),
		proactive_brief: proactive_brief.as_ref(),
		scheduled_memory: scheduled_memory.as_ref(),
		work_continuity: work_continuity.as_ref(),
	});
	let dimension_scores = self::dimensions::dimension_scores(job, &counts);
	let normalized_score = self::dimensions::normalized_score(&dimension_scores);
	let wrong_result_count = self::counts::wrong_result_count(&counts);
	let status = self::dimensions::job_status(
		normalized_score,
		job.scoring_rubric.pass_threshold,
		wrong_result_count,
		unsupported_claims.len(),
		counts.source_mutations,
		counts.blocking_executable_gaps,
	);
	let reason = self::dimensions::job_reason(status, &counts, normalized_score);

	for claim in &mut unsupported_claims {
		claim.suite_id = job.suite.clone();
		claim.job_id = job.job_id.clone();
	}

	JobScoring {
		status,
		normalized_score,
		hard_fail_hits,
		unsupported_claims,
		wrong_result_count,
		knowledge,
		trap_ids_used,
		dimension_scores,
		reason,
		evolution,
		consolidation,
		local_organizer,
		memory_summary,
		proactive_brief,
		scheduled_memory,
		work_continuity,
	}
}

fn failure_counts(ctx: FailureCountContext<'_>) -> FailureCounts {
	let operator_counts = self::counts::operator_debug_failure_counts(ctx.job);
	let stale_answers = ctx.evolution.map_or(0, |report| report.stale_answer_count);
	let conflict_detection_missing =
		ctx.evolution.map_or(0, |report| report.conflict_count - report.conflict_detection_count);
	let update_rationale_missing = ctx.evolution.map_or(0, update_rationale_missing_count);
	let mut counts = FailureCounts {
		missing_claims: ctx.missing_claims_len,
		forbidden_claims: ctx.forbidden_claims_len,
		missing_evidence: ctx.missing_evidence_len,
		trap_uses: ctx.trap_ids_used_len,
		unsupported_claims: ctx.unsupported_claims_len,
		operator_debug_missing: operator_counts.operator_debug_missing,
		operator_debug_raw_sql: operator_counts.operator_debug_raw_sql,
		operator_debug_trace_gaps: operator_counts.operator_debug_trace_gaps,
		operator_debug_repair_unclear: operator_counts.operator_debug_repair_unclear,
		stale_answers,
		conflict_detection_missing,
		update_rationale_missing,
		latency_violations: ctx.latency_violations,
		proposal_usefulness_failures: self::consolidation::proposal_usefulness_failures(
			ctx.consolidation,
		),
		lineage_failures: self::consolidation::lineage_failures(ctx.consolidation),
		review_action_failures: self::consolidation::review_action_failures(ctx.consolidation),
		source_mutations: ctx.consolidation.map_or(0, |report| report.source_mutation_count)
			+ self::local_organizer::authority_mutations(ctx.local_organizer),
		blocking_executable_gaps: self::consolidation::blocking_executable_gaps(ctx.consolidation),
		local_organizer_schema_failures: self::local_organizer::schema_failures(
			ctx.local_organizer,
		),
		local_organizer_provenance_failures: self::local_organizer::provenance_failures(
			ctx.local_organizer,
		),
		local_organizer_unsupported_claims: self::local_organizer::unsupported_claim_failures(
			ctx.local_organizer,
		),
		local_organizer_lifecycle_failures: self::local_organizer::lifecycle_failures(
			ctx.local_organizer,
		),
		local_organizer_authority_mutations: self::local_organizer::authority_mutations(
			ctx.local_organizer,
		),
		local_organizer_escalation_failures: self::local_organizer::escalation_failures(
			ctx.local_organizer,
		),
		untraced_page_sections: ctx.knowledge.map_or(0, |metrics| metrics.untraced_section_count),
		missed_stale_findings: ctx.knowledge.map_or(0, missed_stale_finding_count),
		rebuild_failures: ctx.knowledge.map_or(0, |metrics| metrics.rebuild_failure_count),
		page_usefulness_failures: ctx.knowledge.map_or(0, page_usefulness_failure_count),
		..FailureCounts::default()
	};

	self::counts::apply_memory_summary_failure_counts(&mut counts, ctx.memory_summary);
	self::counts::apply_proactive_brief_failure_counts(&mut counts, ctx.proactive_brief);
	self::counts::apply_scheduled_memory_failure_counts(&mut counts, ctx.scheduled_memory);
	self::counts::apply_work_continuity_failure_counts(&mut counts, ctx.work_continuity);

	counts
}
