use super::*;

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct ConsolidationRunCreateBody {
	pub(in crate::routes) job_kind: String,
	pub(in crate::routes) input_refs: Vec<ConsolidationInputRef>,
	#[serde(default = "empty_json_object")]
	pub(in crate::routes) source_snapshot: Value,
	pub(in crate::routes) lineage: ConsolidationLineage,
	#[serde(default)]
	pub(in crate::routes) proposals: Vec<ConsolidationProposalInput>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct ConsolidationRunsListQuery {
	pub(in crate::routes) limit: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct ConsolidationProposalsListQuery {
	pub(in crate::routes) run_id: Option<Uuid>,
	pub(in crate::routes) review_state: Option<ConsolidationReviewState>,
	pub(in crate::routes) limit: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct ConsolidationProposalReviewBody {
	pub(in crate::routes) action: ConsolidationReviewAction,
	pub(in crate::routes) review_comment: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct DreamingReviewQueueQuery {
	pub(in crate::routes) run_id: Option<Uuid>,
	pub(in crate::routes) review_state: Option<ConsolidationReviewState>,
	pub(in crate::routes) limit: Option<u32>,
}
