use super::*;

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct GraphQueryBody {
	pub(in crate::routes) subject: GraphQueryEntityRef,
	pub(in crate::routes) predicate: Option<GraphQueryPredicateRef>,
	pub(in crate::routes) scopes: Option<Vec<String>>,
	pub(in crate::routes) as_of: Option<String>,
	pub(in crate::routes) limit: Option<u32>,
	pub(in crate::routes) explain: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct GraphReportBody {
	pub(in crate::routes) subject: GraphQueryEntityRef,
	pub(in crate::routes) predicate: Option<GraphQueryPredicateRef>,
	pub(in crate::routes) scopes: Option<Vec<String>>,
	pub(in crate::routes) as_of: Option<String>,
	pub(in crate::routes) limit: Option<u32>,
	pub(in crate::routes) explain: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct AdminGraphPredicatesListQuery {
	pub(in crate::routes) scope: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct AdminGraphPredicatePatchBody {
	pub(in crate::routes) status: Option<String>,
	pub(in crate::routes) cardinality: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::routes) struct AdminGraphPredicateAliasAddBody {
	pub(in crate::routes) alias: String,
}
