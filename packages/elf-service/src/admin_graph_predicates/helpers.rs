use sqlx::PgConnection;
use uuid::Uuid;

use crate::{
	Result,
	admin_graph_predicates::types::{
		AdminGraphPredicateAliasResponse, AdminGraphPredicateResponse,
	},
};
use elf_storage::{
	graph,
	models::{GraphPredicate, GraphPredicateAlias},
};

const GRAPH_PREDICATE_SCOPE_GLOBAL: &str = "__global__";
const GRAPH_PREDICATE_SCOPE_PROJECT_PREFIX: &str = "__project__:";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AdminGraphPredicateScope {
	TenantProject,
	Project,
	Global,
	All,
}
impl AdminGraphPredicateScope {
	pub(super) fn parse(raw: &str) -> Option<Self> {
		match raw.trim() {
			"tenant_project" => Some(Self::TenantProject),
			"project" => Some(Self::Project),
			"global" => Some(Self::Global),
			"all" => Some(Self::All),
			_ => None,
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PredicateAccess {
	Read,
	Mutate,
}

pub(super) fn graph_predicate_scope_keys(
	tenant_id: &str,
	project_id: &str,
	scope: AdminGraphPredicateScope,
) -> Vec<String> {
	let tenant_project_key = format!("{tenant_id}:{project_id}");
	let project_key = format!("{GRAPH_PREDICATE_SCOPE_PROJECT_PREFIX}{project_id}");
	let global_key = GRAPH_PREDICATE_SCOPE_GLOBAL.to_string();

	match scope {
		AdminGraphPredicateScope::TenantProject => vec![tenant_project_key],
		AdminGraphPredicateScope::Project => vec![project_key],
		AdminGraphPredicateScope::Global => vec![global_key],
		AdminGraphPredicateScope::All => vec![tenant_project_key, project_key, global_key],
	}
}

pub(super) fn predicate_status_transition_allowed(old: &str, new: &str) -> bool {
	matches!(
		(old, new),
		("pending", "active") | ("pending", "deprecated") | ("active", "deprecated")
	)
}

pub(super) fn stable_sort_aliases(aliases: &mut [GraphPredicateAlias]) {
	aliases.sort_by(|a, b| {
		a.created_at
			.cmp(&b.created_at)
			.then_with(|| a.alias_norm.cmp(&b.alias_norm))
			.then_with(|| a.alias.cmp(&b.alias))
	});
}

pub(super) fn to_predicate_response(predicate: GraphPredicate) -> AdminGraphPredicateResponse {
	AdminGraphPredicateResponse {
		predicate_id: predicate.predicate_id,
		scope_key: predicate.scope_key,
		tenant_id: predicate.tenant_id,
		project_id: predicate.project_id,
		canonical: predicate.canonical,
		canonical_norm: predicate.canonical_norm,
		cardinality: predicate.cardinality,
		status: predicate.status,
		created_at: predicate.created_at,
		updated_at: predicate.updated_at,
	}
}

pub(super) fn to_alias_response(alias: GraphPredicateAlias) -> AdminGraphPredicateAliasResponse {
	AdminGraphPredicateAliasResponse {
		alias_id: alias.alias_id,
		predicate_id: alias.predicate_id,
		scope_key: alias.scope_key,
		alias: alias.alias,
		alias_norm: alias.alias_norm,
		created_at: alias.created_at,
	}
}

pub(super) fn map_storage_error(err: elf_storage::Error) -> crate::Error {
	match err {
		elf_storage::Error::InvalidArgument(message) => crate::Error::InvalidRequest { message },
		elf_storage::Error::NotFound(message) => crate::Error::NotFound { message },
		elf_storage::Error::Conflict(message) => crate::Error::Conflict { message },
		elf_storage::Error::Sqlx(err) => crate::Error::Storage { message: err.to_string() },
		elf_storage::Error::Qdrant(err) => crate::Error::Qdrant { message: err.to_string() },
	}
}

pub(super) async fn load_predicate_in_context(
	conn: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	predicate_id: Uuid,
	access: PredicateAccess,
	allow_global_mutation: bool,
) -> Result<GraphPredicate> {
	let predicate = graph::get_predicate_by_id(conn, predicate_id)
		.await
		.map_err(map_storage_error)?
		.ok_or_else(|| crate::Error::NotFound {
			message: format!("graph predicate not found; predicate_id={predicate_id}"),
		})?;
	let tenant_project_key = format!("{tenant_id}:{project_id}");
	let project_key = format!("{GRAPH_PREDICATE_SCOPE_PROJECT_PREFIX}{project_id}");
	let is_in_context =
		predicate.scope_key == tenant_project_key || predicate.scope_key == project_key;
	let is_global = predicate.scope_key == GRAPH_PREDICATE_SCOPE_GLOBAL;

	if !is_in_context && !is_global {
		return Err(crate::Error::NotFound {
			message: format!("graph predicate not found; predicate_id={predicate_id}"),
		});
	}
	if access == PredicateAccess::Mutate && is_global && !allow_global_mutation {
		return Err(crate::Error::ScopeDenied {
			message: "Super-admin token required to modify global graph predicates.".to_string(),
		});
	}

	Ok(predicate)
}
