use super::*;

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct SearchCreateRequest {
	pub(in crate::routes) mode: SearchMode,
	pub(in crate::routes) query: String,
	pub(in crate::routes) top_k: Option<u32>,
	pub(in crate::routes) candidate_k: Option<u32>,

	pub(in crate::routes) filter: Option<Value>,
	pub(in crate::routes) payload_level: Option<PayloadLevel>,
	pub(in crate::routes) ranking: Option<RankingRequestOverride>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::routes) struct SearchIndexResponseV2 {
	pub(in crate::routes) mode: SearchMode,
	pub(in crate::routes) trace_id: Uuid,
	pub(in crate::routes) search_id: Uuid,
	#[serde(with = "elf_service::time_serde")]
	pub(in crate::routes) expires_at: OffsetDateTime,
	pub(in crate::routes) items: Vec<SearchIndexItem>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(in crate::routes) trajectory_summary: Option<SearchTrajectorySummary>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(in crate::routes) query_plan: Option<QueryPlan>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::routes) struct SearchCreateResponseV2 {
	pub(in crate::routes) mode: SearchMode,
	pub(in crate::routes) trace_id: Uuid,
	pub(in crate::routes) search_id: Uuid,
	#[serde(with = "elf_service::time_serde")]
	pub(in crate::routes) expires_at: OffsetDateTime,
	pub(in crate::routes) items: Vec<SearchIndexItem>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(in crate::routes) trajectory_summary: Option<SearchTrajectorySummary>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub(in crate::routes) query_plan: Option<QueryPlan>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct SearchSessionGetQuery {
	pub(in crate::routes) payload_level: Option<PayloadLevel>,
	pub(in crate::routes) top_k: Option<u32>,
	pub(in crate::routes) touch: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct SearchTimelineQuery {
	pub(in crate::routes) payload_level: Option<PayloadLevel>,
	pub(in crate::routes) group_by: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::routes) struct SearchTimelineResponseV2 {
	pub(in crate::routes) search_id: Uuid,
	#[serde(with = "elf_service::time_serde")]
	pub(in crate::routes) expires_at: OffsetDateTime,
	pub(in crate::routes) groups: Vec<SearchTimelineGroup>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct SearchDetailsBody {
	pub(in crate::routes) note_ids: Vec<Uuid>,
	pub(in crate::routes) payload_level: Option<PayloadLevel>,
	pub(in crate::routes) record_hits: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::routes) struct SearchDetailsResponseV2 {
	pub(in crate::routes) search_id: Uuid,
	#[serde(with = "elf_service::time_serde")]
	pub(in crate::routes) expires_at: OffsetDateTime,
	pub(in crate::routes) results: Vec<SearchDetailsResult>,
}
