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
	let status = if input.failure.is_some() {
		MaterializationStatus::Incomplete
	} else if !required_evidence_satisfied {
		MaterializationStatus::WrongResult
	} else {
		MaterializationStatus::Pass
	};
	let failure_stage = if input.failure.is_some() {
		Some("live_adapter.retrieve".to_string())
	} else if capture_failure.is_some() {
		Some("live_adapter.capture_policy".to_string())
	} else {
		None
	};
	let failure_reason = input.failure.clone().or(capture_failure);
	let stage_notes = if let Some(reason) = &failure_reason {
		reason.clone()
	} else if !required_evidence_satisfied {
		"Adapter did not return all required mapped evidence for this job.".to_string()
	} else {
		"Adapter returned mapped evidence through its live retrieval path.".to_string()
	};
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

pub(super) fn declared_encoding_job(
	adapter_id: &str,
	loaded: &LoadedJob,
) -> Option<MaterializedJob> {
	if is_operator_debug_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if is_elf_consolidation_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if is_elf_knowledge_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if is_elf_capture_live_adapter(adapter_id, loaded.job.suite.as_str()) {
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
	if is_operator_debug_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if is_elf_consolidation_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if is_elf_knowledge_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if is_elf_capture_live_adapter(adapter_id, loaded.job.suite.as_str()) {
		return None;
	}
	if is_elf_dreaming_readback_live_adapter(adapter_id, loaded.job.suite.as_str()) {
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

pub(super) fn is_elf_dreaming_readback_live_adapter(adapter_id: &str, suite: &str) -> bool {
	matches!(suite, "memory_summary" | "proactive_brief" | "scheduled_memory")
		&& matches!(adapter_id, "elf_service_native_dreaming" | "elf_live_real_world")
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

fn is_operator_debug_live_adapter(adapter_id: &str, suite: &str) -> bool {
	suite == "operator_debugging_ux"
		&& matches!(
			adapter_id,
			"elf_live_real_world"
				| "qmd_live_real_world"
				| "elf_operator_debug_live"
				| "qmd_operator_debug_live"
		)
}

fn is_elf_consolidation_live_adapter(adapter_id: &str, suite: &str) -> bool {
	suite == "consolidation" && adapter_id == "elf_live_real_world"
}

fn is_elf_knowledge_live_adapter(adapter_id: &str, suite: &str) -> bool {
	suite == "knowledge_compilation" && adapter_id == "elf_live_real_world"
}

fn is_elf_capture_live_adapter(adapter_id: &str, suite: &str) -> bool {
	suite == "capture_integration"
		&& matches!(adapter_id, "elf_live_real_world" | "elf_capture_write_policy_live")
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
