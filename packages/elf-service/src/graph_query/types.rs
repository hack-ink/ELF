use crate::graph_query::{Deserialize, OffsetDateTime, RelationTemporalStatus, Serialize, Uuid};

/// Subject selector used by graph-query APIs.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GraphQueryEntityRef {
	/// Resolve the subject by entity identifier.
	EntityId {
		/// Entity identifier to resolve.
		entity_id: Uuid,
	},
	/// Resolve the subject by canonical or alias surface.
	Surface {
		/// Canonical or alias surface to resolve.
		surface: String,
	},
}

/// Predicate selector used by graph-query APIs.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GraphQueryPredicateRef {
	/// Resolve the predicate by predicate identifier.
	PredicateId {
		/// Predicate identifier to resolve.
		predicate_id: Uuid,
	},
	/// Resolve the predicate by canonical or alias surface.
	Surface {
		/// Canonical or alias surface to resolve.
		surface: String,
	},
}

/// Request payload for graph-query lookups.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GraphQueryRequest {
	/// Tenant to query within.
	pub tenant_id: String,
	/// Project to query within.
	pub project_id: String,
	/// Agent requesting the read.
	pub agent_id: String,
	/// Read profile that determines visible scopes.
	pub read_profile: String,
	/// Subject entity selector.
	pub subject: GraphQueryEntityRef,

	/// Optional predicate selector used to narrow the results.
	pub predicate: Option<GraphQueryPredicateRef>,

	/// Optional requested scopes.
	pub scopes: Option<Vec<String>>,
	#[serde(with = "crate::time_serde::option")]
	/// Point-in-time view for temporal facts.
	pub as_of: Option<OffsetDateTime>,
	/// Optional maximum number of returned facts.
	pub limit: Option<u32>,
	/// When true, includes explain metadata.
	pub explain: Option<bool>,
}

/// Response payload for graph-query lookups.
#[derive(Clone, Debug, Serialize)]
pub struct GraphQueryResponse {
	#[serde(with = "crate::time_serde")]
	/// Effective point-in-time view used for the query.
	pub as_of: OffsetDateTime,
	/// Resolved subject entity.
	pub subject: GraphQueryEntity,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Resolved predicate, when the request filtered by predicate.
	pub predicate: Option<GraphQueryPredicate>,
	/// Effective scopes used for the query.
	pub scopes: Vec<String>,
	/// Whether the result set was truncated by the limit.
	pub truncated: bool,
	/// Returned fact rows.
	pub facts: Vec<GraphQueryFact>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional explain metadata.
	pub explain: Option<GraphQueryExplain>,
}

/// Resolved graph entity reference.
#[derive(Clone, Debug, Serialize)]
pub struct GraphQueryEntity {
	/// Entity identifier.
	pub entity_id: Uuid,
	/// Canonical entity surface.
	pub canonical: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional entity kind.
	pub kind: Option<String>,
}

/// Resolved graph predicate reference.
#[derive(Clone, Debug, Serialize)]
pub struct GraphQueryPredicate {
	/// Predicate identifier.
	pub predicate_id: Uuid,
	/// Canonical predicate surface.
	pub canonical: String,
}

/// One graph fact returned by the query.
#[derive(Clone, Debug, Serialize)]
pub struct GraphQueryFact {
	/// Fact identifier.
	pub fact_id: Uuid,
	/// Scope key for the fact.
	pub scope: String,
	/// Agent that emitted the fact.
	pub actor: String,
	/// Predicate surface recorded on the fact.
	pub predicate: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Resolved predicate identifier, when available.
	pub predicate_id: Option<Uuid>,
	#[serde(with = "crate::time_serde")]
	/// Start of the fact validity window.
	pub valid_from: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	/// End of the fact validity window, if superseded.
	pub valid_to: Option<OffsetDateTime>,
	/// Temporal state for the fact relative to the service read timestamp.
	pub temporal_status: RelationTemporalStatus,
	/// Object payload for the fact.
	pub object: GraphQueryObject,
	/// Evidence note identifiers supporting the fact.
	pub evidence_note_ids: Vec<Uuid>,
}

/// Object payload returned for a graph fact.
#[derive(Clone, Debug, Serialize)]
pub struct GraphQueryObject {
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Entity-shaped object value.
	pub entity: Option<GraphQueryObjectEntity>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Scalar object value.
	pub value: Option<String>,
}

/// Resolved entity payload for a graph-fact object.
#[derive(Clone, Debug, Serialize)]
pub struct GraphQueryObjectEntity {
	/// Entity identifier.
	pub entity_id: Uuid,
	/// Canonical entity surface.
	pub canonical: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional entity kind.
	pub kind: Option<String>,
}

/// Explain metadata for a graph-query response.
#[derive(Clone, Debug, Serialize)]
pub struct GraphQueryExplain {
	/// Explain schema identifier.
	pub schema: String,
	#[serde(with = "crate::time_serde")]
	/// Effective point-in-time view used for the query.
	pub as_of: OffsetDateTime,
	/// Requested result limit.
	pub requested_limit: u32,
	/// Scopes allowed by the read profile.
	pub allowed_scopes: Vec<String>,
	/// Scopes effectively queried after request filtering.
	pub effective_scopes: Vec<String>,
	/// Number of rows read from storage.
	pub queried_rows: usize,
	/// Number of rows returned to the caller.
	pub returned_rows: usize,
	/// Whether the result set was truncated by the limit.
	pub truncated: bool,
}
