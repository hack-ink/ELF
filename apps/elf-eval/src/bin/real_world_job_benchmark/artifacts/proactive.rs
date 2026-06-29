use crate::{
	Deserialize, Serialize, Value,
	artifacts::memory::{MemorySummaryFreshness, MemorySummarySourceTrace},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ProactiveBriefArtifact {
	pub(crate) brief_id: String,
	pub(crate) contract_schema: String,
	pub(crate) generated_at: String,
	pub(crate) tenant_id: String,
	pub(crate) project_id: String,
	pub(crate) agent_id: String,
	pub(crate) read_profile: String,
	pub(crate) brief_kind: String,
	#[serde(default)]
	pub(crate) suggestions: Vec<ProactiveSuggestion>,
	pub(crate) source_trace: MemorySummarySourceTrace,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ProactiveSuggestion {
	pub(crate) suggestion_id: String,
	pub(crate) suggestion_kind: String,
	pub(crate) title: String,
	pub(crate) body: String,
	#[serde(default)]
	pub(crate) evidence_refs: Vec<String>,
	pub(crate) freshness: MemorySummaryFreshness,
	pub(crate) action: ProactiveSuggestionAction,
	#[serde(default)]
	pub(crate) unsupported_claim_flags: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ProactiveSuggestionAction {
	pub(crate) decision: String,
	pub(crate) reason_code: String,
	pub(crate) reason: String,
}
