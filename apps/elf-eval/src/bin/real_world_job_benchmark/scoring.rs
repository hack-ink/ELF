use super::{
	feature_metrics::{
		forbidden_diff_key_count, knowledge_metrics, memory_summary_metrics,
		missed_stale_finding_count, page_usefulness_failure_count, proactive_brief_metrics,
		scheduled_memory_metrics, unsupported_memory_summary_claims, unsupported_page_claims,
		unsupported_proactive_suggestions, unsupported_scheduled_outputs, work_continuity_metrics,
	},
	formatting::{bounded_text, round3},
	operational::operational_evidence_tier,
	summary::{mean_proposal_metric, ratio_or},
	*,
};

#[path = "scoring/answers.rs"] mod answers;
#[path = "scoring/claims.rs"] mod claims;
#[path = "scoring/consolidation.rs"] mod consolidation;
#[path = "scoring/counts.rs"] mod counts;
#[path = "scoring/dimensions.rs"] mod dimensions;
#[path = "scoring/evolution.rs"] mod evolution;
#[path = "scoring/reports.rs"] mod reports;

use self::{answers::*, claims::*, consolidation::*, counts::*, dimensions::*, evolution::*};

pub(super) fn job_report(job: &RealWorldJob, scoring: JobScoring) -> JobReport {
	reports::job_report(job, scoring)
}

pub(super) fn score_job(job: &RealWorldJob) -> JobScoring {
	let answer = produced_answer(job);
	let produced_evidence = produced_evidence_ids(answer);
	let trap_ids_used = trap_ids_used(job, &produced_evidence);
	let consolidation = consolidation_job_report(job);

	if let Some(status) = job.encoding.status {
		let evolution = evolution_job_report(job, answer, &trap_ids_used, 0);

		return score_declared_job(job, status, trap_ids_used, evolution, consolidation);
	}

	let missing_claims = missing_required_claims(job, answer);
	let forbidden_claims = forbidden_claim_hits(job, answer);
	let missing_evidence = missing_required_evidence(job, &produced_evidence);
	let knowledge = knowledge_metrics(job, answer);
	let memory_summary = memory_summary_metrics(job, answer);
	let proactive_brief = proactive_brief_metrics(job, answer);
	let scheduled_memory = scheduled_memory_metrics(job, answer);
	let work_continuity = work_continuity_metrics(job, answer);
	let mut unsupported_claims = unsupported_claims(job, answer);

	unsupported_claims.extend(unsupported_page_claims(answer));
	unsupported_claims.extend(unsupported_memory_summary_claims(job, answer));
	unsupported_claims.extend(unsupported_proactive_suggestions(job, answer));
	unsupported_claims.extend(unsupported_scheduled_outputs(job, answer));

	let operator_counts = operator_debug_failure_counts(job);
	let latency_violations = latency_violations(job, answer);
	let hard_fail_hits = hard_fail_hits(job, &unsupported_claims, &trap_ids_used);
	let evolution = evolution_job_report(job, answer, &trap_ids_used, forbidden_claims.len());
	let stale_answers = evolution.as_ref().map_or(0, |report| report.stale_answer_count);
	let conflict_detection_missing = evolution
		.as_ref()
		.map_or(0, |report| report.conflict_count - report.conflict_detection_count);
	let update_rationale_missing = evolution.as_ref().map_or(0, update_rationale_missing_count);
	let mut counts = FailureCounts {
		missing_claims: missing_claims.len(),
		forbidden_claims: forbidden_claims.len(),
		missing_evidence: missing_evidence.len(),
		trap_uses: trap_ids_used.len(),
		unsupported_claims: unsupported_claims.len(),
		operator_debug_missing: operator_counts.operator_debug_missing,
		operator_debug_raw_sql: operator_counts.operator_debug_raw_sql,
		operator_debug_trace_gaps: operator_counts.operator_debug_trace_gaps,
		operator_debug_repair_unclear: operator_counts.operator_debug_repair_unclear,
		stale_answers,
		conflict_detection_missing,
		update_rationale_missing,
		latency_violations,
		proposal_usefulness_failures: proposal_usefulness_failures(consolidation.as_ref()),
		lineage_failures: lineage_failures(consolidation.as_ref()),
		review_action_failures: review_action_failures(consolidation.as_ref()),
		source_mutations: consolidation.as_ref().map_or(0, |report| report.source_mutation_count),
		blocking_executable_gaps: blocking_executable_gaps(consolidation.as_ref()),
		untraced_page_sections: knowledge
			.as_ref()
			.map_or(0, |metrics| metrics.untraced_section_count),
		missed_stale_findings: knowledge.as_ref().map_or(0, missed_stale_finding_count),
		rebuild_failures: knowledge.as_ref().map_or(0, |metrics| metrics.rebuild_failure_count),
		page_usefulness_failures: knowledge.as_ref().map_or(0, page_usefulness_failure_count),
		..FailureCounts::default()
	};

	apply_memory_summary_failure_counts(&mut counts, memory_summary.as_ref());
	apply_proactive_brief_failure_counts(&mut counts, proactive_brief.as_ref());
	apply_scheduled_memory_failure_counts(&mut counts, scheduled_memory.as_ref());
	apply_work_continuity_failure_counts(&mut counts, work_continuity.as_ref());

	let dimension_scores = dimension_scores(job, &counts);
	let normalized_score = normalized_score(&dimension_scores);
	let wrong_result_count = wrong_result_count(&counts);
	let status = job_status(
		normalized_score,
		job.scoring_rubric.pass_threshold,
		wrong_result_count,
		unsupported_claims.len(),
		counts.source_mutations,
		counts.blocking_executable_gaps,
	);
	let reason = job_reason(status, &counts, normalized_score);

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
		memory_summary,
		proactive_brief,
		scheduled_memory,
		work_continuity,
	}
}
