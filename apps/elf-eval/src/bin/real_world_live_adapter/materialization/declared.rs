use crate::{
	AdapterResponseOutput, AnswerOutput, CostOutput, LoadedJob, MaterializationStatus,
	MaterializedJob, MaterializedJobEvidence, TraceExplainabilityOutput, TraceStageOutput,
	materialization::support,
};

pub(super) fn declared_encoding_job(
	adapter_id: &str,
	loaded: &LoadedJob,
) -> Option<MaterializedJob> {
	if support::is_operator_debug_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if support::is_elf_consolidation_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if support::is_elf_knowledge_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if support::is_elf_capture_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}

	let status = loaded.job.encoding.status?;
	let reason = loaded.job.encoding.reason.clone().unwrap_or_else(|| {
		format!("Fixture declares {} for this live adapter job.", status.as_str())
	});

	Some(materialized_declared_status_job(
		adapter_id,
		loaded,
		status.materialization_status(),
		reason,
	))
}

pub(super) fn not_encoded_job(adapter_id: &str, loaded: &LoadedJob) -> Option<MaterializedJob> {
	if support::is_operator_debug_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if support::is_elf_consolidation_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if support::is_elf_knowledge_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if support::is_elf_capture_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if support::is_elf_dreaming_readback_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}

	not_encoded_reason(loaded.job.suite.as_str()).map(|reason| {
		materialized_declared_status_job(
			adapter_id,
			loaded,
			MaterializationStatus::NotEncoded,
			reason.to_string(),
		)
	})
}

pub(super) fn materialized_declared_status_job(
	adapter_id: &str,
	loaded: &LoadedJob,
	status: MaterializationStatus,
	reason: String,
) -> MaterializedJob {
	let failure = match status {
		MaterializationStatus::Pass | MaterializationStatus::WrongResult => None,
		MaterializationStatus::Blocked
		| MaterializationStatus::Incomplete
		| MaterializationStatus::NotEncoded => Some(reason.clone()),
	};

	MaterializedJob {
		response: AdapterResponseOutput {
			adapter_id: adapter_id.to_string(),
			answer: AnswerOutput {
				content: String::new(),
				evidence_ids: Vec::new(),
				claims: Vec::new(),
				pages: Vec::new(),
				memory_summaries: Vec::new(),
				proactive_briefs: Vec::new(),
				scheduled_tasks: Vec::new(),
				latency_ms: 0.0,
				cost: CostOutput {
					currency: "USD".to_string(),
					amount: 0.0,
					input_tokens: 0,
					output_tokens: 0,
				},
				trace_explainability: TraceExplainabilityOutput {
					trace_id: None,
					failure_stage: Some("live_adapter.suite_support".to_string()),
					failure_reason: failure.clone(),
					stages: vec![TraceStageOutput {
						stage_name: "live_adapter.suite_support".to_string(),
						kept_evidence: Vec::new(),
						dropped_evidence: Vec::new(),
						demoted_evidence: Vec::new(),
						distractor_evidence: Vec::new(),
						notes: reason.clone(),
					}],
				},
			},
			consolidation: None,
		},
		evidence: MaterializedJobEvidence {
			job_id: loaded.job.job_id.clone(),
			suite: loaded.job.suite.clone(),
			title: loaded.job.title.clone(),
			status,
			query: loaded.job.prompt.content.clone(),
			evidence_ids: Vec::new(),
			returned_count: 0,
			indexing_latency_ms: None,
			latency_ms: 0.0,
			trace_id: None,
			failure,
			source_mappings: Vec::new(),
			operator_debug: None,
			capture: None,
			consolidation: None,
			knowledge: None,
			temporal_reconciliation: None,
			dreaming_readback: None,
		},
		operator_debug: None,
	}
}

fn not_encoded_reason(suite: &str) -> Option<&'static str> {
	match suite {
		"trust_source_of_truth"
		| "work_resume"
		| "project_decisions"
		| "retrieval"
		| "memory_evolution"
		| "personalization" => None,
		"consolidation" => Some(
			"The live adapter sweep retrieves evidence-linked answers but does not generate or review consolidation proposals.",
		),
		"knowledge_compilation" => Some(
			"The live adapter sweep retrieves evidence-linked answers but does not generate derived knowledge pages.",
		),
		"operator_debugging_ux" => Some(
			"The full live adapter sweep keeps operator trace/viewer diagnostics in a focused operator-debug slice.",
		),
		"capture_integration" => Some(
			"The live adapter sweep does not exercise capture integrations or write-policy redaction boundaries.",
		),
		"production_ops" => Some(
			"The live adapter sweep does not run backup/restore, private corpus, provider credential, or backfill operations.",
		),
		_ => Some("The live adapter sweep has no encoded runtime path for this suite."),
	}
}
