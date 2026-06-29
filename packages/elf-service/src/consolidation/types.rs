mod proposals;
mod runs;

pub use self::{
	proposals::{
		ConsolidationProposalGetRequest, ConsolidationProposalInput, ConsolidationProposalResponse,
		ConsolidationProposalReviewEventResponse, ConsolidationProposalReviewRequest,
		ConsolidationProposalsListRequest, ConsolidationProposalsListResponse,
	},
	runs::{
		ConsolidationRunCreateRequest, ConsolidationRunCreateResponse, ConsolidationRunGetRequest,
		ConsolidationRunResponse, ConsolidationRunsListRequest, ConsolidationRunsListResponse,
	},
};

use serde::Deserialize;
use serde_json::{Map, Value};

pub(super) const DEFAULT_LIST_LIMIT: i64 = 50;
pub(super) const MAX_LIST_LIMIT: i64 = 200;

#[derive(Clone, Debug, Deserialize)]
pub(super) struct PromotedMemoryPayload {
	#[serde(rename = "type")]
	pub(super) note_type: String,
	pub(super) text: String,
	pub(super) scope: Option<String>,
	pub(super) key: Option<String>,
	pub(super) importance: Option<f32>,
	pub(super) confidence: Option<f32>,
	pub(super) ttl_days: Option<i64>,
	#[serde(default = "empty_object")]
	pub(super) source_ref: Value,
}

pub(super) fn empty_object() -> Value {
	Value::Object(Map::new())
}
