use crate::{
	AdapterResponseOutput, AnswerOutput, CostOutput, LoadedJob, MaterializationStatus,
	MaterializedJob, MaterializedJobEvidence, MaterializedJobInput, TraceExplainabilityOutput,
	TraceStageOutput,
};

pub(super) fn materialized_job(
	loaded: &LoadedJob,
	adapter_id: &str,
	input: MaterializedJobInput,
) -> MaterializedJob {
	let capture_failure = input.capture_failure.clone();
	let required_evidence_satisfied = capture_failure.is_none()
		&& crate::required_evidence_satisfied(loaded, &input.evidence_ids);
	let status = materialization_status(input.failure.is_some(), required_evidence_satisfied);
	let failure_stage = failure_stage(input.failure.is_some(), capture_failure.is_some());
	let failure_reason = input.failure.clone().or(capture_failure);
	let stage_notes = stage_notes(&failure_reason, required_evidence_satisfied);
	let trace_stages = input.trace_stages.unwrap_or_else(|| {
		vec![TraceStageOutput {
			stage_name: failure_stage
				.clone()
				.unwrap_or_else(|| "live_adapter.retrieve".to_string()),
			kept_evidence: input.evidence_ids.clone(),
			dropped_evidence: Vec::new(),
			demoted_evidence: Vec::new(),
			distractor_evidence: Vec::new(),
			notes: stage_notes,
		}]
	});

	MaterializedJob {
		response: AdapterResponseOutput {
			adapter_id: adapter_id.to_string(),
			answer: AnswerOutput {
				content: input.content,
				evidence_ids: input.evidence_ids.clone(),
				claims: crate::answer_claims(loaded, &input.evidence_ids),
				pages: input.pages,
				memory_summaries: input.memory_summaries,
				proactive_briefs: input.proactive_briefs,
				scheduled_tasks: input.scheduled_tasks,
				latency_ms: input.latency_ms,
				cost: CostOutput {
					currency: "USD".to_string(),
					amount: 0.0,
					input_tokens: 0,
					output_tokens: 0,
				},
				trace_explainability: TraceExplainabilityOutput {
					trace_id: input.trace_id.map(|id| id.to_string()),
					failure_stage: failure_stage.clone(),
					failure_reason: failure_reason.clone(),
					stages: trace_stages,
				},
			},
			consolidation: input.consolidation_response,
		},
		operator_debug: input.operator_debug,
		evidence: MaterializedJobEvidence {
			job_id: loaded.job.job_id.clone(),
			suite: loaded.job.suite.clone(),
			title: loaded.job.title.clone(),
			status,
			query: loaded.job.prompt.content.clone(),
			evidence_ids: input.evidence_ids,
			returned_count: input.returned_count,
			indexing_latency_ms: input.indexing_latency_ms,
			latency_ms: input.latency_ms,
			trace_id: input.trace_id,
			failure: failure_reason,
			source_mappings: input.source_mappings,
			operator_debug: input.operator_debug_evidence,
			capture: input.capture,
			consolidation: input.consolidation,
			knowledge: input.knowledge,
			temporal_reconciliation: input.temporal_reconciliation,
			dreaming_readback: input.dreaming_readback,
		},
	}
}

fn materialization_status(
	has_retrieval_failure: bool,
	required_evidence_satisfied: bool,
) -> MaterializationStatus {
	if has_retrieval_failure {
		MaterializationStatus::Incomplete
	} else if !required_evidence_satisfied {
		MaterializationStatus::WrongResult
	} else {
		MaterializationStatus::Pass
	}
}

fn failure_stage(has_retrieval_failure: bool, has_capture_failure: bool) -> Option<String> {
	if has_retrieval_failure {
		Some("live_adapter.retrieve".to_string())
	} else if has_capture_failure {
		Some("live_adapter.capture_policy".to_string())
	} else {
		None
	}
}

fn stage_notes(failure_reason: &Option<String>, required_evidence_satisfied: bool) -> String {
	if let Some(reason) = failure_reason {
		reason.clone()
	} else if !required_evidence_satisfied {
		"Adapter did not return all required mapped evidence for this job.".to_string()
	} else {
		"Adapter returned mapped evidence through its live retrieval path.".to_string()
	}
}
