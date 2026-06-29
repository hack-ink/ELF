use super::*;

/// Request payload for a graph topic-map report.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GraphReportRequest {
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
	/// Optional predicate selector used to narrow the report.
	pub predicate: Option<GraphQueryPredicateRef>,
	/// Optional requested scopes.
	pub scopes: Option<Vec<String>>,
	#[serde(with = "crate::time_serde::option")]
	/// Point-in-time used for current, historical, and future classification.
	pub as_of: Option<OffsetDateTime>,
	/// Optional maximum number of returned facts.
	pub limit: Option<u32>,
	/// When true, includes explain metadata.
	pub explain: Option<bool>,
}

/// Response payload for a graph topic-map report.
#[derive(Clone, Debug, Serialize)]
pub struct GraphReportResponse {
	/// Report schema identifier.
	pub schema: String,
	#[serde(with = "crate::time_serde")]
	/// Effective point-in-time view used for temporal classification.
	pub as_of: OffsetDateTime,
	/// Resolved subject entity.
	pub subject: GraphReportEntity,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Resolved predicate, when the request filtered by predicate.
	pub predicate: Option<GraphReportPredicate>,
	/// Effective scopes used for the report.
	pub scopes: Vec<String>,
	/// Aggregate report counters.
	pub summary: GraphReportSummary,
	/// Topic map projection of the graph facts.
	pub topic_map: GraphTopicMap,
	/// Returned fact rows.
	pub facts: Vec<GraphReportFact>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional explain metadata.
	pub explain: Option<GraphReportExplain>,
}

/// Resolved graph entity reference.
#[derive(Clone, Debug, Serialize)]
pub struct GraphReportEntity {
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
pub struct GraphReportPredicate {
	/// Predicate identifier.
	pub predicate_id: Uuid,
	/// Canonical predicate surface.
	pub canonical: String,
}

/// Aggregate counters for graph reports.
#[derive(Clone, Debug, Default, Serialize)]
pub struct GraphReportSummary {
	/// Number of returned facts.
	pub fact_count: usize,
	/// Number of facts current at `as_of`.
	pub current_count: usize,
	/// Number of facts historical at `as_of`.
	pub historical_count: usize,
	/// Number of facts whose validity starts after `as_of`.
	pub future_count: usize,
	/// Number of facts with at least one evidence note link.
	pub sourced_count: usize,
	/// Number of facts still backed by pending or unresolved predicate vocabulary.
	pub inferred_count: usize,
	/// Number of facts that conflict under a single-cardinality predicate.
	pub ambiguous_count: usize,
	/// Number of stale facts, currently equivalent to historical facts.
	pub stale_count: usize,
	/// Number of facts linked to a superseding replacement.
	pub superseded_count: usize,
	/// Total evidence note links returned with the facts.
	pub evidence_link_count: usize,
}

/// One graph fact returned by a graph report.
#[derive(Clone, Debug, Serialize)]
pub struct GraphReportFact {
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
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Predicate registry status, when available.
	pub predicate_status: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Predicate registry cardinality, when available.
	pub predicate_cardinality: Option<String>,
	#[serde(with = "crate::time_serde")]
	/// Start of the fact validity window.
	pub valid_from: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	/// End of the fact validity window, if superseded or explicitly bounded.
	pub valid_to: Option<OffsetDateTime>,
	/// Temporal state for the fact relative to report `as_of`.
	pub temporal_status: RelationTemporalStatus,
	/// Object payload for the fact.
	pub object: GraphQueryObject,
	/// Evidence note identifiers supporting the fact.
	pub evidence_note_ids: Vec<Uuid>,
	/// Replacement fact ids that supersede this fact.
	pub superseded_by_fact_ids: Vec<Uuid>,
	/// Older fact ids superseded by this fact.
	pub supersedes_fact_ids: Vec<Uuid>,
	/// Source-backed report status markers.
	pub status_markers: Vec<String>,
}

/// Topic-map projection for graph reports.
#[derive(Clone, Debug, Serialize)]
pub struct GraphTopicMap {
	/// Topic-map nodes.
	pub nodes: Vec<GraphTopicNode>,
	/// Topic-map edges, one per returned fact.
	pub edges: Vec<GraphTopicEdge>,
}

/// Topic-map node.
#[derive(Clone, Debug, Serialize)]
pub struct GraphTopicNode {
	/// Stable node identifier.
	pub node_id: String,
	/// Human-readable node label.
	pub label: String,
	/// Node type such as subject, entity, or value.
	pub node_type: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// Optional entity kind.
	pub kind: Option<String>,
}

/// Topic-map edge.
#[derive(Clone, Debug, Serialize)]
pub struct GraphTopicEdge {
	/// Backing fact identifier.
	pub fact_id: Uuid,
	/// Source topic node identifier.
	pub source_node_id: String,
	/// Target topic node identifier.
	pub target_node_id: String,
	/// Predicate label.
	pub predicate: String,
	/// Temporal state for the edge.
	pub temporal_status: RelationTemporalStatus,
	/// Source-backed report status markers.
	pub status_markers: Vec<String>,
	/// Evidence note identifiers supporting the edge.
	pub evidence_note_ids: Vec<Uuid>,
}

/// Explain metadata for graph reports.
#[derive(Clone, Debug, Serialize)]
pub struct GraphReportExplain {
	/// Explain schema identifier.
	pub schema: String,
	#[serde(with = "crate::time_serde")]
	/// Effective point-in-time used for classification.
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
