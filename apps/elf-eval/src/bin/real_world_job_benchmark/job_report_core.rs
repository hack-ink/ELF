use crate::{
	AuthorityRecoveryDrillArtifact, CostReport, Deserialize, OperatorDebugEvidence, Serialize,
	TraceExplainability, TypedStatus,
	job_reports::{
		ConsolidationJobReport, EvolutionJobReport, KnowledgeJobMetrics, MemorySummaryJobMetrics,
		ProactiveBriefJobMetrics, ScheduledMemoryJobMetrics, WorkContinuityJobMetrics,
	},
};

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct JobReport {
	pub(crate) suite_id: String,
	pub(crate) job_id: String,
	pub(crate) title: String,
	pub(crate) status: TypedStatus,
	pub(crate) operational_evidence_tier: String,
	pub(crate) answer_type: String,
	pub(crate) requires_caveat: bool,
	pub(crate) requires_refusal: bool,
	pub(crate) can_answer_unknown: bool,
	pub(crate) normalized_score: f64,
	pub(crate) hard_fail_hits: Vec<String>,
	pub(crate) expected_evidence: Vec<ExpectedEvidenceReport>,
	pub(crate) produced_answer: String,
	pub(crate) produced_evidence: Vec<String>,
	pub(crate) unsupported_claim_count: usize,
	pub(crate) wrong_result_count: usize,
	#[serde(default)]
	pub(crate) stale_answer_count: usize,
	#[serde(default)]
	pub(crate) conflict_detection_count: usize,
	#[serde(default)]
	pub(crate) update_rationale_available: bool,
	#[serde(default)]
	pub(crate) temporal_validity_not_encoded: bool,
	#[serde(default)]
	pub(crate) history_readback_encoded: bool,
	pub(crate) retrieval_quality: RetrievalQualityReport,
	pub(crate) latency_ms: Option<f64>,
	pub(crate) cost: Option<CostReport>,
	pub(crate) trace_explainability: Option<TraceExplainability>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) knowledge: Option<KnowledgeJobMetrics>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) memory_summary: Option<MemorySummaryJobMetrics>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) proactive_brief: Option<ProactiveBriefJobMetrics>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) scheduled_memory: Option<ScheduledMemoryJobMetrics>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) work_continuity: Option<WorkContinuityJobMetrics>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub(crate) recovery_drills: Vec<AuthorityRecoveryDrillArtifact>,
	pub(crate) trap_ids_used: Vec<String>,
	pub(crate) dimension_scores: Vec<DimensionScoreReport>,
	pub(crate) reason: String,
	#[serde(default)]
	pub(crate) evidence_required_count: usize,
	#[serde(default)]
	pub(crate) evidence_covered_count: usize,
	#[serde(default)]
	pub(crate) source_ref_required_count: usize,
	#[serde(default)]
	pub(crate) source_ref_covered_count: usize,
	#[serde(default)]
	pub(crate) quote_required_count: usize,
	#[serde(default)]
	pub(crate) quote_covered_count: usize,
	#[serde(default)]
	pub(crate) stale_retrieval_count: usize,
	#[serde(default)]
	pub(crate) scope_check_count: usize,
	#[serde(default)]
	pub(crate) scope_correct_count: usize,
	#[serde(default)]
	pub(crate) scope_violation_count: usize,
	#[serde(default)]
	pub(crate) redaction_leak_count: usize,
	#[serde(default)]
	pub(crate) qdrant_rebuild_case: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) operator_debug: Option<OperatorDebugEvidence>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) evolution: Option<EvolutionJobReport>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) consolidation: Option<ConsolidationJobReport>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ExpectedEvidenceReport {
	pub(crate) evidence_id: String,
	pub(crate) claim_id: String,
	pub(crate) requirement: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct DimensionScoreReport {
	pub(crate) dimension: String,
	pub(crate) score: f64,
	pub(crate) max_points: f64,
	pub(crate) weight: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct RetrievalQualityReport {
	pub(crate) expected_evidence_total: usize,
	pub(crate) expected_evidence_matched: usize,
	pub(crate) expected_evidence_recall: f64,
	pub(crate) produced_evidence_total: usize,
	pub(crate) irrelevant_context_count: usize,
	pub(crate) irrelevant_context_ratio: f64,
	pub(crate) trap_context_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct UnsupportedClaimReport {
	pub(crate) suite_id: String,
	pub(crate) job_id: String,
	pub(crate) claim_id: Option<String>,
	pub(crate) claim_text: String,
	pub(crate) reason: String,
	pub(crate) evidence_ids: Vec<String>,
}
