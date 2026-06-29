use super::*;

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct RecallDebugPanelBody {
	pub(in crate::routes) trace_id: Option<Uuid>,
	pub(in crate::routes) query: Option<String>,
	pub(in crate::routes) docs_query: Option<String>,
	pub(in crate::routes) knowledge_query: Option<String>,
	pub(in crate::routes) graph_subject: Option<GraphQueryEntityRef>,
	pub(in crate::routes) graph_predicate: Option<GraphQueryPredicateRef>,
	pub(in crate::routes) include_dreaming: Option<bool>,
	pub(in crate::routes) limit: Option<u32>,
}
