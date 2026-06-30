use crate::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct EvolutionSummary {
	pub(crate) stale_answer_count: usize,
	pub(crate) conflict_detection_count: usize,
	pub(crate) update_rationale_available_count: usize,
	pub(crate) temporal_validity_not_encoded_count: usize,
	pub(crate) history_readback_encoded_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct EvolutionJobReport {
	pub(crate) current_evidence: Vec<String>,
	pub(crate) historical_evidence: Vec<String>,
	pub(crate) tombstone_evidence: Vec<String>,
	pub(crate) invalidation_evidence: Vec<String>,
	pub(crate) selected_current_evidence: Vec<String>,
	pub(crate) selected_historical_evidence: Vec<String>,
	pub(crate) selected_rationale_evidence: Vec<String>,
	pub(crate) selected_tombstone_evidence: Vec<String>,
	pub(crate) selected_invalidation_evidence: Vec<String>,
	pub(crate) conflict_candidate_evidence: Vec<String>,
	pub(crate) retrieved_but_dropped_evidence: Vec<String>,
	pub(crate) selected_but_not_narrated_evidence: Vec<String>,
	pub(crate) stale_trap_ids_used: Vec<String>,
	pub(crate) stale_answer_count: usize,
	pub(crate) conflict_count: usize,
	pub(crate) conflict_detection_count: usize,
	pub(crate) update_rationale_available: bool,
	pub(crate) temporal_validity_required: bool,
	pub(crate) temporal_validity_encoded: bool,
	pub(crate) temporal_validity_not_encoded: bool,
	pub(crate) history_readback_encoded: bool,
	pub(crate) history_event_types: Vec<String>,
	pub(crate) history_requires_note_version_links: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(crate) follow_up: Option<String>,
}
