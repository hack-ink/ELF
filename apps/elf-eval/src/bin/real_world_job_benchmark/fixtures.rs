use crate::{
	BTreeMap, CaptureIntegrationReport, ConsolidationFixture, CorpusProfile, Deserialize,
	EvidenceLink, ExpectedClaim, OperatorDebugEvidence, ProducedAnswer, Serialize, TypedStatus,
	Value,
};

#[derive(Debug, Deserialize)]
pub(super) struct RealWorldJob {
	pub(super) schema: String,
	pub(super) job_id: String,
	pub(super) suite: String,
	pub(super) title: String,
	pub(super) corpus: Corpus,
	#[serde(default)]
	pub(super) timeline: Vec<TimelineEvent>,
	pub(super) prompt: Prompt,
	pub(super) expected_answer: ExpectedAnswer,
	#[serde(default)]
	pub(super) required_evidence: Vec<RequiredEvidence>,
	#[serde(default)]
	pub(super) negative_traps: Vec<NegativeTrap>,
	pub(super) scoring_rubric: ScoringRubric,
	pub(super) allowed_uncertainty: AllowedUncertainty,
	pub(super) operator_debug: Option<OperatorDebugEvidence>,
	#[serde(default)]
	pub(super) tags: Vec<String>,
	#[serde(default)]
	pub(super) encoding: JobEncoding,
	pub(super) memory_evolution: Option<MemoryEvolution>,
	pub(super) memory_summary: Option<MemorySummaryExpectation>,
	pub(super) proactive_brief: Option<ProactiveBriefExpectation>,
	pub(super) scheduled_memory: Option<ScheduledMemoryExpectation>,
	pub(super) work_continuity: Option<WorkContinuityExpectation>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Corpus {
	pub(super) corpus_id: String,
	pub(super) profile: CorpusProfile,
	#[serde(default)]
	pub(super) items: Vec<CorpusItem>,
	#[serde(default)]
	pub(super) capture_behaviors: CaptureIntegrationReport,

	pub(super) adapter_response: Option<AdapterResponse>,
}

#[derive(Debug, Deserialize)]
pub(super) struct CorpusItem {
	pub(super) evidence_id: String,
	pub(super) kind: String,

	pub(super) text: Option<String>,

	pub(super) local_ref: Option<String>,
	#[serde(default)]
	pub(super) source_ref: Value,

	pub(super) created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TimelineEvent {
	pub(super) event_id: String,
	pub(super) ts: String,
	pub(super) actor: String,
	pub(super) action: String,
	#[serde(default)]
	pub(super) evidence_ids: Vec<String>,
	pub(super) summary: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct Prompt {
	pub(super) role: String,
	pub(super) content: String,
	pub(super) job_mode: String,
	#[serde(default)]
	pub(super) constraints: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ExpectedAnswer {
	#[serde(default)]
	pub(super) must_include: Vec<ExpectedClaim>,
	#[serde(default)]
	pub(super) must_not_include: Vec<String>,
	#[serde(default)]
	pub(super) evidence_links: BTreeMap<String, EvidenceLink>,
	pub(super) answer_type: String,
	#[serde(default)]
	pub(super) accepted_alternates: Vec<Value>,
	#[serde(default)]
	pub(super) requires_caveat: bool,
	#[serde(default)]
	pub(super) requires_refusal: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct RequiredEvidence {
	pub(super) evidence_id: String,
	pub(super) claim_id: String,
	pub(super) requirement: String,

	pub(super) quote: Option<String>,

	pub(super) selector: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct NegativeTrap {
	pub(super) trap_id: String,
	#[serde(rename = "type")]
	pub(super) trap_type: String,
	#[serde(default)]
	pub(super) evidence_ids: Vec<String>,
	#[serde(default)]
	pub(super) failure_if_used: bool,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct JobEncoding {
	pub(super) status: Option<TypedStatus>,
	pub(super) reason: Option<String>,
	pub(super) follow_up: Option<FollowUpInput>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct FollowUpInput {
	pub(super) title: String,
	pub(super) reason: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct MemoryEvolution {
	#[serde(default)]
	pub(super) current_evidence_ids: Vec<String>,
	#[serde(default)]
	pub(super) historical_evidence_ids: Vec<String>,
	#[serde(default)]
	pub(super) tombstone_evidence_ids: Vec<String>,
	#[serde(default)]
	pub(super) invalidation_evidence_ids: Vec<String>,
	#[serde(default)]
	pub(super) stale_trap_ids: Vec<String>,
	#[serde(default)]
	pub(super) conflicts: Vec<EvolutionConflict>,
	pub(super) update_rationale: Option<UpdateRationale>,
	pub(super) temporal_validity: Option<TemporalValidity>,
	pub(super) history_readback: Option<HistoryReadback>,
}

#[derive(Debug, Deserialize)]
pub(super) struct EvolutionConflict {
	pub(super) conflict_id: String,
	pub(super) claim_id: String,
	pub(super) current_evidence_id: String,
	pub(super) historical_evidence_id: String,
	pub(super) resolved_by_evidence_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct UpdateRationale {
	pub(super) claim_id: String,
	#[serde(default)]
	pub(super) evidence_ids: Vec<String>,
	pub(super) available: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct TemporalValidity {
	pub(super) required: bool,
	pub(super) encoded: bool,
	pub(super) follow_up: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct HistoryReadback {
	pub(super) encoded: bool,
	#[serde(default)]
	pub(super) required_event_types: Vec<String>,
	pub(super) requires_note_version_links: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct MemorySummaryExpectation {
	#[serde(default)]
	pub(super) required_categories: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ProactiveBriefExpectation {
	#[serde(default)]
	pub(super) required_suggestion_kinds: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ScheduledMemoryExpectation {
	#[serde(default)]
	pub(super) required_task_kinds: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WorkContinuityExpectation {
	#[serde(default)]
	pub(super) required_reset_resume_entry_ids: Vec<String>,
	#[serde(default)]
	pub(super) required_decision_rationale_evidence_ids: Vec<String>,
	#[serde(default)]
	pub(super) required_rejected_option_ids: Vec<String>,
	#[serde(default)]
	pub(super) required_explicit_next_step_ids: Vec<String>,
	#[serde(default)]
	pub(super) required_inferred_next_step_ids: Vec<String>,
	#[serde(default)]
	pub(super) required_handoff_source_ref_ids: Vec<String>,
	#[serde(default)]
	pub(super) required_redaction_marker_ids: Vec<String>,
	#[serde(default)]
	pub(super) required_janitor_candidate_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ScoringRubric {
	#[serde(default)]
	pub(super) dimensions: BTreeMap<String, RubricDimension>,
	pub(super) pass_threshold: f64,
	#[serde(default)]
	pub(super) hard_fail_rules: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RubricDimension {
	pub(super) weight: f64,
	pub(super) max_points: f64,
	pub(super) criteria: Value,
}

#[derive(Debug, Deserialize)]
pub(super) struct AllowedUncertainty {
	pub(super) can_answer_unknown: bool,
	#[serde(default)]
	pub(super) acceptable_phrases: Vec<String>,
	pub(super) fallback_action: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct AdapterResponse {
	pub(super) adapter_id: Option<String>,
	pub(super) answer: ProducedAnswer,
	pub(super) consolidation: Option<ConsolidationFixture>,
}
