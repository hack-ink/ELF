use crate::{Deserialize, Serialize, Value};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct MemorySummaryArtifact {
	pub(crate) summary_id: String,
	pub(crate) contract_schema: String,
	pub(crate) generated_at: String,
	pub(crate) tenant_id: String,
	pub(crate) project_id: String,
	pub(crate) agent_id: String,
	pub(crate) read_profile: String,
	#[serde(default)]
	pub(crate) entries: Vec<MemorySummaryEntry>,
	pub(crate) source_trace: MemorySummarySourceTrace,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct MemorySummaryEntry {
	pub(crate) entry_id: String,
	pub(crate) category: String,
	pub(crate) text: String,
	#[serde(default)]
	pub(crate) source_refs: Vec<String>,
	pub(crate) freshness: MemorySummaryFreshness,
	pub(crate) rationale: MemorySummaryRationale,
	#[serde(default)]
	pub(crate) unsupported_claim_flags: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct MemorySummaryFreshness {
	pub(crate) status: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) observed_at: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) valid_from: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) valid_to: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) last_confirmed_at: Option<String>,
	#[serde(default)]
	pub(crate) superseded_by: Vec<String>,
	#[serde(default)]
	pub(crate) tombstone_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct MemorySummaryRationale {
	pub(crate) decision: String,
	pub(crate) reason_code: String,
	pub(crate) reason: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct MemorySummarySourceTrace {
	#[serde(default)]
	pub(crate) selected_source_refs: Vec<MemorySummarySourceTraceItem>,
	#[serde(default)]
	pub(crate) dropped_source_refs: Vec<MemorySummarySourceTraceItem>,
	#[serde(default)]
	pub(crate) stale_source_refs: Vec<MemorySummarySourceTraceItem>,
	#[serde(default)]
	pub(crate) superseded_source_refs: Vec<MemorySummarySourceTraceItem>,
	#[serde(default)]
	pub(crate) tombstone_source_refs: Vec<MemorySummarySourceTraceItem>,
	#[serde(default)]
	pub(crate) unsupported_claim_flags: Vec<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct MemorySummarySourceTraceItem {
	pub(crate) evidence_id: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) status: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) reason: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) superseded_by: Option<String>,
}
