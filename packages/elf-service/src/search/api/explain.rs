use crate::search::api::{
	Deserialize, OffsetDateTime, RelationTemporalStatus, SearchRankingExplain,
	SearchTrajectorySummary, Serialize, Uuid, Value,
};

/// Full explanation attached to one search item.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchExplain {
	/// Match-specific explanation.
	pub r#match: SearchMatchExplain,
	/// Ranking-term explanation.
	pub ranking: SearchRankingExplain,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional relation-context snippets supporting the match.
	pub relation_context: Option<Vec<SearchExplainRelationContext>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional diversity-selection explanation.
	pub diversity: Option<SearchDiversityExplain>,
}

/// Relation-context row attached to a search explanation.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchExplainRelationContext {
	/// Fact identifier.
	pub fact_id: Uuid,
	/// Scope key for the fact.
	pub scope: String,
	/// Subject entity reference.
	pub subject: SearchExplainRelationEntityRef,
	/// Predicate surface.
	pub predicate: String,
	/// Object payload.
	pub object: SearchExplainRelationContextObject,
	#[serde(with = "crate::time_serde")]
	/// Start of the fact validity window.
	pub valid_from: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	/// End of the fact validity window, if superseded.
	pub valid_to: Option<OffsetDateTime>,
	#[serde(default)]
	/// Temporal state for the fact relative to the search read timestamp.
	pub temporal_status: RelationTemporalStatus,
	#[serde(default)]
	/// Evidence note identifiers supporting the fact.
	pub evidence_note_ids: Vec<Uuid>,
}

/// Lightweight entity reference used in search explanations.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchExplainRelationEntityRef {
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Canonical entity surface.
	pub canonical: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional entity kind.
	pub kind: Option<String>,
}

/// Object payload used in search explanation relation context.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchExplainRelationContextObject {
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Entity-shaped object value.
	pub entity: Option<SearchExplainRelationEntityRef>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Scalar object value.
	pub value: Option<String>,
}

/// Match-level explanation for a search item.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchMatchExplain {
	/// Query terms matched by the item.
	pub matched_terms: Vec<String>,
	/// Fields that supplied the matches.
	pub matched_fields: Vec<String>,
}

/// Diversity-selection explanation for a search item.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchDiversityExplain {
	/// Whether diversity ranking was enabled.
	pub enabled: bool,
	/// Reason the item was selected.
	pub selected_reason: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Reason the item was skipped, when applicable.
	pub skipped_reason: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Nearest already selected note that influenced the decision.
	pub nearest_selected_note_id: Option<Uuid>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Similarity to the nearest selected note.
	pub similarity: Option<f32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// MMR score used by diversity selection.
	pub mmr_score: Option<f32>,
	#[serde(default)]
	/// Whether the item lacked an embedding needed for diversity scoring.
	pub missing_embedding: bool,
}

/// One ranked search result item.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchItem {
	/// Stable result-handle identifier for explain APIs.
	pub result_handle: Uuid,
	/// Note identifier.
	pub note_id: Uuid,
	/// Chunk identifier.
	pub chunk_id: Uuid,
	/// Zero-based chunk position.
	pub chunk_index: i32,
	/// Inclusive start byte offset of the snippet chunk.
	pub start_offset: i32,
	/// Exclusive end byte offset of the snippet chunk.
	pub end_offset: i32,
	/// Returned snippet text.
	pub snippet: String,
	/// Note type discriminator.
	pub r#type: String,
	/// Optional application-defined key.
	pub key: Option<String>,
	/// Scope key for the note.
	pub scope: String,
	/// Importance score.
	pub importance: f32,
	/// Confidence score.
	pub confidence: f32,
	#[serde(with = "crate::time_serde")]
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	/// Optional expiry timestamp.
	pub expires_at: Option<OffsetDateTime>,
	/// Final ranked score.
	pub final_score: f32,
	/// Structured source reference metadata.
	pub source_ref: Value,
	/// Item-level explanation payload.
	pub explain: SearchExplain,
}

/// Response payload for raw search results.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchResponse {
	/// Search trace identifier.
	pub trace_id: Uuid,
	/// Ranked search items.
	pub items: Vec<SearchItem>,
	/// Optional condensed explain output.
	pub trajectory_summary: Option<SearchTrajectorySummary>,
}
