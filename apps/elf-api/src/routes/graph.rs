mod predicates;
mod query;

pub(super) use self::{
	predicates::{
		__path_admin_graph_predicate_alias_add, __path_admin_graph_predicate_aliases_list,
		__path_admin_graph_predicate_patch, __path_admin_graph_predicates_list,
		admin_graph_predicate_alias_add, admin_graph_predicate_aliases_list,
		admin_graph_predicate_patch, admin_graph_predicates_list,
	},
	query::{__path_graph_query, __path_graph_report, graph_query, graph_report},
};
