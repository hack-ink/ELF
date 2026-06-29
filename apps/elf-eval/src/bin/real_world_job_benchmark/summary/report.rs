use super::{super::formatting::trace_failure_stage, *};

pub(super) fn report_summary_impl(jobs: &[JobReport], suites: &[SuiteReport]) -> ReportSummary {
	let job_refs = jobs.iter().collect::<Vec<_>>();
	let evidence_required_count = jobs.iter().map(|job| job.evidence_required_count).sum();
	let evidence_covered_count = jobs.iter().map(|job| job.evidence_covered_count).sum();
	let source_ref_required_count = jobs.iter().map(|job| job.source_ref_required_count).sum();
	let source_ref_covered_count = jobs.iter().map(|job| job.source_ref_covered_count).sum();
	let quote_required_count = jobs.iter().map(|job| job.quote_required_count).sum();
	let quote_covered_count = jobs.iter().map(|job| job.quote_covered_count).sum();
	let scope_check_count = jobs.iter().map(|job| job.scope_check_count).sum();
	let scope_correct_count = jobs.iter().map(|job| job.scope_correct_count).sum();
	let mut summary = ReportSummary {
		job_count: jobs.len(),
		encoded_suite_count: suites.iter().filter(|suite| suite.encoded_job_count > 0).count(),
		not_encoded: 0,
		unsupported_claim_count: jobs.iter().map(|job| job.unsupported_claim_count).sum(),
		wrong_result_count: jobs.iter().map(|job| job.wrong_result_count).sum(),
		stale_answer_count: jobs.iter().map(|job| job.stale_answer_count).sum(),
		conflict_detection_count: jobs.iter().map(|job| job.conflict_detection_count).sum(),
		update_rationale_available_count: jobs
			.iter()
			.filter(|job| job.update_rationale_available)
			.count(),
		temporal_validity_not_encoded_count: jobs
			.iter()
			.filter(|job| job.temporal_validity_not_encoded)
			.count(),
		history_readback_encoded_count: jobs
			.iter()
			.filter(|job| job.history_readback_encoded)
			.count(),
		expected_evidence_total: jobs
			.iter()
			.map(|job| job.retrieval_quality.expected_evidence_total)
			.sum(),
		expected_evidence_matched: jobs
			.iter()
			.map(|job| job.retrieval_quality.expected_evidence_matched)
			.sum(),
		expected_evidence_recall: expected_evidence_recall_for_jobs(&job_refs),
		irrelevant_context_count: jobs
			.iter()
			.map(|job| job.retrieval_quality.irrelevant_context_count)
			.sum(),
		irrelevant_context_ratio: irrelevant_context_ratio_for_jobs(&job_refs),
		trace_explainability_count: jobs
			.iter()
			.filter(|job| job.trace_explainability.is_some())
			.count(),
		wrong_result_stage_attribution_count: jobs
			.iter()
			.filter(|job| {
				job.status == TypedStatus::WrongResult
					&& trace_failure_stage(job.trace_explainability.as_ref()).is_some()
			})
			.count(),
		mean_score: mean_score(jobs),
		mean_latency_ms: mean_latency(jobs),
		total_cost: total_cost(jobs),
		evidence_required_count,
		evidence_covered_count,
		evidence_coverage: ratio(evidence_covered_count, evidence_required_count),
		source_ref_required_count,
		source_ref_covered_count,
		source_ref_coverage: ratio(source_ref_covered_count, source_ref_required_count),
		quote_required_count,
		quote_covered_count,
		quote_coverage: ratio(quote_covered_count, quote_required_count),
		stale_retrieval_count: jobs.iter().map(|job| job.stale_retrieval_count).sum(),
		scope_check_count,
		scope_correct_count,
		scope_correctness: ratio(scope_correct_count, scope_check_count),
		scope_violation_count: jobs.iter().map(|job| job.scope_violation_count).sum(),
		redaction_leak_count: jobs.iter().map(|job| job.redaction_leak_count).sum(),
		qdrant_rebuild_case_count: jobs.iter().filter(|job| job.qdrant_rebuild_case).count(),
		qdrant_rebuild_pass_count: jobs
			.iter()
			.filter(|job| job.qdrant_rebuild_case && job.status == TypedStatus::Pass)
			.count(),
		operator_debug_job_count: jobs.iter().filter(|job| job.operator_debug.is_some()).count(),
		raw_sql_needed_count: jobs
			.iter()
			.filter_map(|job| job.operator_debug.as_ref())
			.filter(|debug| debug.raw_sql_needed)
			.count(),
		trace_incomplete_count: jobs
			.iter()
			.filter_map(|job| job.operator_debug.as_ref())
			.filter(|debug| debug.trace_completeness != "complete")
			.count(),
		operator_ux_gap_count: jobs
			.iter()
			.filter_map(|job| job.operator_debug.as_ref())
			.map(|debug| debug.ux_gaps.len())
			.sum(),
		consolidation: consolidation_summary(jobs),
		memory_summary: memory_summary_summary(jobs),
		proactive_brief: proactive_brief_summary(jobs),
		scheduled_memory: scheduled_memory_summary(jobs),
		work_continuity: work_continuity_summary(jobs),
		knowledge: knowledge_summary(jobs),
		..ReportSummary::default()
	};

	for job in jobs {
		match job.status {
			TypedStatus::Pass => summary.pass += 1,
			TypedStatus::WrongResult => summary.wrong_result += 1,
			TypedStatus::LifecycleFail => summary.lifecycle_fail += 1,
			TypedStatus::Incomplete => summary.incomplete += 1,
			TypedStatus::Blocked => summary.blocked += 1,
			TypedStatus::NotEncoded => summary.not_encoded += 1,
			TypedStatus::UnsupportedClaim => summary.unsupported_claim += 1,
		}
	}

	summary
}

pub(super) fn evolution_summary_impl(jobs: &[JobReport]) -> EvolutionSummary {
	EvolutionSummary {
		stale_answer_count: jobs.iter().map(|job| job.stale_answer_count).sum(),
		conflict_detection_count: jobs.iter().map(|job| job.conflict_detection_count).sum(),
		update_rationale_available_count: jobs
			.iter()
			.filter(|job| job.update_rationale_available)
			.count(),
		temporal_validity_not_encoded_count: jobs
			.iter()
			.filter(|job| job.temporal_validity_not_encoded)
			.count(),
		history_readback_encoded_count: jobs
			.iter()
			.filter(|job| job.history_readback_encoded)
			.count(),
	}
}

pub(super) fn follow_up_reports_impl(jobs: &[RealWorldJob]) -> Vec<FollowUpReport> {
	jobs.iter()
		.filter_map(|job| {
			job.encoding.follow_up.as_ref().map(|follow_up| FollowUpReport {
				suite_id: job.suite.clone(),
				job_id: job.job_id.clone(),
				title: follow_up.title.clone(),
				reason: follow_up.reason.clone(),
			})
		})
		.collect()
}
