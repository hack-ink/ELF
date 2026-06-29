use super::*;

pub(super) fn job_report(job: &RealWorldJob, scoring: JobScoring) -> JobReport {
	let answer = produced_answer(job);
	let metrics = job_metrics(job, answer);
	let retrieval_quality = retrieval_quality_report(job, answer);

	JobReport {
		suite_id: job.suite.clone(),
		job_id: job.job_id.clone(),
		title: job.title.clone(),
		status: scoring.status,
		operational_evidence_tier: operational_evidence_tier(job).to_string(),
		answer_type: job.expected_answer.answer_type.clone(),
		requires_caveat: job.expected_answer.requires_caveat,
		requires_refusal: job.expected_answer.requires_refusal,
		can_answer_unknown: job.allowed_uncertainty.can_answer_unknown,
		normalized_score: round3(scoring.normalized_score),
		hard_fail_hits: scoring.hard_fail_hits,
		expected_evidence: expected_evidence_report(job),
		produced_answer: answer.content.clone(),
		produced_evidence: produced_evidence_ids(answer).into_iter().collect(),
		unsupported_claim_count: scoring.unsupported_claims.len(),
		wrong_result_count: scoring.wrong_result_count,
		stale_answer_count: scoring
			.evolution
			.as_ref()
			.map_or(0, |report| report.stale_answer_count),
		conflict_detection_count: scoring
			.evolution
			.as_ref()
			.map_or(0, |report| report.conflict_detection_count),
		update_rationale_available: scoring
			.evolution
			.as_ref()
			.is_some_and(|report| report.update_rationale_available),
		temporal_validity_not_encoded: scoring
			.evolution
			.as_ref()
			.is_some_and(|report| report.temporal_validity_not_encoded),
		history_readback_encoded: scoring
			.evolution
			.as_ref()
			.is_some_and(|report| report.history_readback_encoded),
		retrieval_quality,
		latency_ms: answer.latency_ms,
		cost: answer.cost.clone(),
		trace_explainability: answer.trace_explainability.clone(),
		knowledge: scoring.knowledge,
		memory_summary: scoring.memory_summary,
		proactive_brief: scoring.proactive_brief,
		scheduled_memory: scoring.scheduled_memory,
		work_continuity: scoring.work_continuity,
		recovery_drills: answer.recovery_drills.clone(),
		trap_ids_used: scoring.trap_ids_used,
		dimension_scores: scoring.dimension_scores,
		reason: scoring.reason,
		evidence_required_count: metrics.evidence_required_count,
		evidence_covered_count: metrics.evidence_covered_count,
		source_ref_required_count: metrics.source_ref_required_count,
		source_ref_covered_count: metrics.source_ref_covered_count,
		quote_required_count: metrics.quote_required_count,
		quote_covered_count: metrics.quote_covered_count,
		stale_retrieval_count: metrics.stale_retrieval_count,
		scope_check_count: metrics.scope_check_count,
		scope_correct_count: metrics.scope_correct_count,
		scope_violation_count: metrics.scope_violation_count,
		redaction_leak_count: metrics.redaction_leak_count,
		qdrant_rebuild_case: metrics.qdrant_rebuild_case,
		operator_debug: job.operator_debug.clone(),
		evolution: scoring.evolution,
		consolidation: scoring.consolidation,
	}
}

fn job_metrics(job: &RealWorldJob, answer: &ProducedAnswer) -> JobMetrics {
	let produced_evidence = produced_evidence_ids(answer);
	let source_ref_by_evidence = source_ref_by_evidence(job);
	let evidence_required_count =
		job.required_evidence.iter().filter(|evidence| is_required_use(evidence)).count();
	let evidence_covered_count = job
		.required_evidence
		.iter()
		.filter(|evidence| is_required_use(evidence))
		.filter(|evidence| produced_evidence.contains(&evidence.evidence_id))
		.count();
	let source_ref_required_count = evidence_required_count;
	let source_ref_covered_count = job
		.required_evidence
		.iter()
		.filter(|evidence| is_required_use(evidence))
		.filter(|evidence| produced_evidence.contains(&evidence.evidence_id))
		.filter(|evidence| {
			source_ref_by_evidence.get(evidence.evidence_id.as_str()).is_some_and(|source_ref| {
				source_ref.as_object().is_some_and(|object| !object.is_empty())
			})
		})
		.count();
	let quote_required_count = job
		.required_evidence
		.iter()
		.filter(|evidence| is_required_use(evidence) && evidence.quote.is_some())
		.count();
	let quote_covered_count = job
		.required_evidence
		.iter()
		.filter(|evidence| is_required_use(evidence) && evidence.quote.is_some())
		.filter(|evidence| produced_evidence.contains(&evidence.evidence_id))
		.count();
	let stale_retrieval_count = trap_use_count(job, &produced_evidence, "stale_fact", answer);
	let scope_violation_count = ["near_duplicate", "scope_leak"]
		.into_iter()
		.map(|trap_type| trap_use_count(job, &produced_evidence, trap_type, answer))
		.sum();
	let scope_check_count = job
		.negative_traps
		.iter()
		.filter(|trap| is_scope_trap_type(trap.trap_type.as_str()))
		.count();
	let redaction_leak_count = trap_use_count(job, &produced_evidence, "privacy_leak", answer);
	let scope_correct_count = scope_check_count.saturating_sub(scope_violation_count);
	let qdrant_rebuild_case = job.tags.iter().any(|tag| tag == "qdrant_rebuild");

	JobMetrics {
		evidence_required_count,
		evidence_covered_count,
		source_ref_required_count,
		source_ref_covered_count,
		quote_required_count,
		quote_covered_count,
		stale_retrieval_count,
		scope_check_count,
		scope_correct_count,
		scope_violation_count,
		redaction_leak_count,
		qdrant_rebuild_case,
	}
}

fn source_ref_by_evidence(job: &RealWorldJob) -> BTreeMap<&str, &Value> {
	job.corpus.items.iter().map(|item| (item.evidence_id.as_str(), &item.source_ref)).collect()
}

fn is_scope_trap_type(trap_type: &str) -> bool {
	matches!(trap_type, "near_duplicate" | "scope_leak")
}

fn trap_use_count(
	job: &RealWorldJob,
	produced_evidence: &BTreeSet<String>,
	trap_type: &str,
	answer: &ProducedAnswer,
) -> usize {
	job.negative_traps
		.iter()
		.filter(|trap| trap.failure_if_used && trap.trap_type == trap_type)
		.filter(|trap| trap_was_used(job, trap, produced_evidence, answer))
		.count()
}

fn trap_was_used(
	job: &RealWorldJob,
	trap: &NegativeTrap,
	produced_evidence: &BTreeSet<String>,
	answer: &ProducedAnswer,
) -> bool {
	trap.evidence_ids.iter().any(|evidence_id| {
		produced_evidence.contains(evidence_id)
			|| answer_contains_corpus_item(job, evidence_id, answer)
	})
}

fn answer_contains_corpus_item(
	job: &RealWorldJob,
	evidence_id: &str,
	answer: &ProducedAnswer,
) -> bool {
	job.corpus
		.items
		.iter()
		.find(|item| item.evidence_id == evidence_id)
		.and_then(|item| item.text.as_deref())
		.is_some_and(|text| !text.trim().is_empty() && answer.content.contains(text))
}

fn retrieval_quality_report(job: &RealWorldJob, answer: &ProducedAnswer) -> RetrievalQualityReport {
	let expected = expected_evidence_ids(job);
	let allowed = allowed_evidence_ids(job);
	let produced = produced_evidence_ids(answer);
	let trap_evidence = trap_evidence_ids(job);
	let expected_evidence_matched =
		expected.iter().filter(|evidence_id| produced.contains(evidence_id.as_str())).count();
	let irrelevant_context_count =
		produced.iter().filter(|evidence_id| !allowed.contains(evidence_id.as_str())).count();
	let trap_context_count =
		produced.iter().filter(|evidence_id| trap_evidence.contains(evidence_id.as_str())).count();

	RetrievalQualityReport {
		expected_evidence_total: expected.len(),
		expected_evidence_matched,
		expected_evidence_recall: ratio_or(expected_evidence_matched, expected.len(), 1.0),
		produced_evidence_total: produced.len(),
		irrelevant_context_count,
		irrelevant_context_ratio: ratio_or(irrelevant_context_count, produced.len(), 0.0),
		trap_context_count,
	}
}

fn expected_evidence_ids(job: &RealWorldJob) -> BTreeSet<String> {
	job.required_evidence
		.iter()
		.filter(|evidence| is_required_use(evidence))
		.map(|evidence| evidence.evidence_id.clone())
		.collect()
}

fn allowed_evidence_ids(job: &RealWorldJob) -> BTreeSet<String> {
	let mut allowed = expected_evidence_ids(job);

	for link in job.expected_answer.evidence_links.values() {
		allowed.extend(link.ids());
	}

	allowed
}

fn trap_evidence_ids(job: &RealWorldJob) -> BTreeSet<String> {
	job.negative_traps.iter().flat_map(|trap| trap.evidence_ids.iter().cloned()).collect()
}

fn expected_evidence_report(job: &RealWorldJob) -> Vec<ExpectedEvidenceReport> {
	job.required_evidence
		.iter()
		.map(|evidence| ExpectedEvidenceReport {
			evidence_id: evidence.evidence_id.clone(),
			claim_id: evidence.claim_id.clone(),
			requirement: evidence.requirement.clone(),
		})
		.collect()
}
