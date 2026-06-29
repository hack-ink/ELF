use super::{
	super::*, cost::CostReport, knowledge::DerivedPageArtifact, memory::MemorySummaryArtifact,
	proactive::ProactiveBriefArtifact, recovery::AuthorityRecoveryDrillArtifact,
	scheduled::ScheduledMemoryTaskArtifact, work::WorkJournalReadbackArtifact,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ProducedAnswer {
	pub(crate) content: String,
	#[serde(default)]
	pub(crate) claims: Vec<ProducedClaim>,
	#[serde(default)]
	pub(crate) evidence_ids: Vec<String>,
	#[serde(default)]
	pub(crate) pages: Vec<DerivedPageArtifact>,
	#[serde(default)]
	pub(crate) memory_summaries: Vec<MemorySummaryArtifact>,
	#[serde(default)]
	pub(crate) proactive_briefs: Vec<ProactiveBriefArtifact>,
	#[serde(default)]
	pub(crate) scheduled_tasks: Vec<ScheduledMemoryTaskArtifact>,
	#[serde(default)]
	pub(crate) work_journal_readbacks: Vec<WorkJournalReadbackArtifact>,
	#[serde(default)]
	pub(crate) recovery_drills: Vec<AuthorityRecoveryDrillArtifact>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) latency_ms: Option<f64>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) cost: Option<CostReport>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) trace_explainability: Option<TraceExplainability>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ProducedClaim {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) claim_id: Option<String>,
	pub(crate) text: String,
	#[serde(default)]
	pub(crate) evidence_ids: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) confidence: Option<String>,
}
