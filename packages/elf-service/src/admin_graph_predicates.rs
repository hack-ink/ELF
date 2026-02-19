use serde::Serialize;
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, Error, Result};
use elf_storage::{
	Error as StorageError, graph as storage_graph,
	models::{GraphPredicate, GraphPredicateAlias},
};

const GRAPH_PREDICATE_SCOPE_GLOBAL: &str = "__global__";
const GRAPH_PREDICATE_SCOPE_PROJECT_PREFIX: &str = "__project__:";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AdminGraphPredicateScope {
	TenantProject,
	Project,
	Global,
	All,
}
impl AdminGraphPredicateScope {
	fn parse(raw: &str) -> Option<Self> {
		match raw.trim() {
			"tenant_project" => Some(Self::TenantProject),
			"project" => Some(Self::Project),
			"global" => Some(Self::Global),
			"all" => Some(Self::All),
			_ => None,
		}
	}
}

#[derive(Clone, Debug)]
pub struct AdminGraphPredicatesListRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub scope: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AdminGraphPredicatePatchRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub predicate_id: Uuid,
	pub status: Option<String>,
	pub cardinality: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AdminGraphPredicateAliasAddRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub predicate_id: Uuid,
	pub alias: String,
}

#[derive(Clone, Debug)]
pub struct AdminGraphPredicateAliasesListRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub predicate_id: Uuid,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminGraphPredicateResponse {
	pub predicate_id: Uuid,
	pub scope_key: String,
	pub tenant_id: Option<String>,
	pub project_id: Option<String>,
	pub canonical: String,
	pub canonical_norm: String,
	pub cardinality: String,
	pub status: String,
	#[serde(with = "crate::time_serde")]
	pub created_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminGraphPredicateAliasResponse {
	pub alias_id: Uuid,
	pub predicate_id: Uuid,
	pub scope_key: String,
	pub alias: String,
	pub alias_norm: String,
	#[serde(with = "crate::time_serde")]
	pub created_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminGraphPredicatesListResponse {
	pub predicates: Vec<AdminGraphPredicateResponse>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminGraphPredicateAliasesResponse {
	pub predicate_id: Uuid,
	pub aliases: Vec<AdminGraphPredicateAliasResponse>,
}

impl ElfService {
	pub async fn admin_graph_predicates_list(
		&self,
		req: AdminGraphPredicatesListRequest,
	) -> Result<AdminGraphPredicatesListResponse> {
		let raw = req.scope.as_deref().unwrap_or("all");
		let scope = AdminGraphPredicateScope::parse(raw).ok_or_else(|| Error::InvalidRequest {
			message: "scope must be one of tenant_project|project|global|all".to_string(),
		})?;
		let scope_keys =
			graph_predicate_scope_keys(req.tenant_id.as_str(), req.project_id.as_str(), scope);

		let mut conn = self.db.pool.acquire().await?;
		let predicates = storage_graph::list_predicates_by_scope_keys(&mut conn, &scope_keys)
			.await
			.map_err(map_storage_error)?;
		let predicates = predicates.into_iter().map(to_predicate_response).collect();

		Ok(AdminGraphPredicatesListResponse { predicates })
	}

	pub async fn admin_graph_predicate_patch(
		&self,
		req: AdminGraphPredicatePatchRequest,
	) -> Result<AdminGraphPredicateResponse> {
		if req.status.is_none() && req.cardinality.is_none() {
			return Err(Error::InvalidRequest {
				message: "At least one of status or cardinality is required.".to_string(),
			});
		}

		let status = req.status.as_deref().map(str::trim);
		if status.is_some_and(str::is_empty) {
			return Err(Error::InvalidRequest { message: "status must be non-empty.".to_string() });
		}
		let cardinality = req.cardinality.as_deref().map(str::trim);
		if cardinality.is_some_and(str::is_empty) {
			return Err(Error::InvalidRequest {
				message: "cardinality must be non-empty.".to_string(),
			});
		}

		let mut conn = self.db.pool.acquire().await?;
		let existing = load_predicate_in_context(
			&mut conn,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.predicate_id,
			PredicateAccess::Mutate,
		)
		.await?;

		let old_status = existing.status.clone();
		let old_cardinality = existing.cardinality.clone();

		if old_status == "deprecated" {
			return Err(Error::Conflict {
				message: "graph predicate is deprecated and cannot be modified.".to_string(),
			});
		}

		let new_status = match status {
			None => None,
			Some(raw) => {
				let raw = raw.to_string();

				if !matches!(raw.as_str(), "pending" | "active" | "deprecated") {
					return Err(Error::InvalidRequest {
						message: "status must be one of pending|active|deprecated.".to_string(),
					});
				}

				if raw != old_status
					&& !predicate_status_transition_allowed(old_status.as_str(), raw.as_str())
				{
					return Err(Error::Conflict {
						message: format!(
							"Invalid graph predicate status transition; from={old_status} to={raw}.",
						),
					});
				}

				Some(raw)
			},
		};

		let new_cardinality = match cardinality {
			None => None,
			Some(raw) => {
				let raw = raw.to_string();

				if !matches!(raw.as_str(), "single" | "multi") {
					return Err(Error::InvalidRequest {
						message: "cardinality must be one of single|multi.".to_string(),
					});
				}

				Some(raw)
			},
		};

		let updated = storage_graph::update_predicate(
			&mut conn,
			req.predicate_id,
			new_status.as_deref(),
			new_cardinality.as_deref(),
		)
		.await
		.map_err(map_storage_error)?;

		tracing::info!(
			actor_agent_id = %req.agent_id,
			predicate_id = %req.predicate_id,
			old_status = %old_status,
			new_status = %updated.status,
			old_cardinality = %old_cardinality,
			new_cardinality = %updated.cardinality,
			"Admin graph predicate patched."
		);

		Ok(to_predicate_response(updated))
	}

	pub async fn admin_graph_predicate_alias_add(
		&self,
		req: AdminGraphPredicateAliasAddRequest,
	) -> Result<AdminGraphPredicateAliasesResponse> {
		let alias = req.alias.trim();
		if alias.is_empty() {
			return Err(Error::InvalidRequest { message: "alias must be non-empty.".to_string() });
		}

		let mut conn = self.db.pool.acquire().await?;
		let predicate = load_predicate_in_context(
			&mut conn,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.predicate_id,
			PredicateAccess::Mutate,
		)
		.await?;

		if predicate.status == "deprecated" {
			return Err(Error::Conflict {
				message: "graph predicate is deprecated and cannot be modified.".to_string(),
			});
		}

		storage_graph::add_predicate_alias(&mut conn, req.predicate_id, alias)
			.await
			.map_err(map_storage_error)?;

		tracing::info!(
			actor_agent_id = %req.agent_id,
			predicate_id = %req.predicate_id,
			alias = %alias,
			"Admin graph predicate alias added."
		);

		let mut aliases = storage_graph::list_predicate_aliases(&mut conn, req.predicate_id)
			.await
			.map_err(map_storage_error)?;
		stable_sort_aliases(&mut aliases);

		let aliases = aliases.into_iter().map(to_alias_response).collect();

		Ok(AdminGraphPredicateAliasesResponse { predicate_id: req.predicate_id, aliases })
	}

	pub async fn admin_graph_predicate_aliases_list(
		&self,
		req: AdminGraphPredicateAliasesListRequest,
	) -> Result<AdminGraphPredicateAliasesResponse> {
		let mut conn = self.db.pool.acquire().await?;
		load_predicate_in_context(
			&mut conn,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.predicate_id,
			PredicateAccess::Read,
		)
		.await?;

		let mut aliases = storage_graph::list_predicate_aliases(&mut conn, req.predicate_id)
			.await
			.map_err(map_storage_error)?;
		stable_sort_aliases(&mut aliases);
		let aliases = aliases.into_iter().map(to_alias_response).collect();

		Ok(AdminGraphPredicateAliasesResponse { predicate_id: req.predicate_id, aliases })
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PredicateAccess {
	Read,
	Mutate,
}

async fn load_predicate_in_context(
	conn: &mut PgConnection,
	tenant_id: &str,
	project_id: &str,
	predicate_id: Uuid,
	access: PredicateAccess,
) -> Result<GraphPredicate> {
	let predicate = storage_graph::get_predicate_by_id(conn, predicate_id)
		.await
		.map_err(map_storage_error)?
		.ok_or_else(|| Error::NotFound {
			message: format!("graph predicate not found; predicate_id={predicate_id}"),
		})?;

	let tenant_project_key = format!("{tenant_id}:{project_id}");
	let project_key = format!("{GRAPH_PREDICATE_SCOPE_PROJECT_PREFIX}{project_id}");

	let is_in_context =
		predicate.scope_key == tenant_project_key || predicate.scope_key == project_key;
	let is_global = predicate.scope_key == GRAPH_PREDICATE_SCOPE_GLOBAL;

	if !is_in_context && !is_global {
		return Err(Error::NotFound {
			message: format!("graph predicate not found; predicate_id={predicate_id}"),
		});
	}

	if access == PredicateAccess::Mutate && is_global {
		return Err(Error::ScopeDenied {
			message: "Global graph predicates are immutable.".to_string(),
		});
	}
	if access == PredicateAccess::Mutate && !is_in_context {
		return Err(Error::NotFound {
			message: format!("graph predicate not found; predicate_id={predicate_id}"),
		});
	}

	Ok(predicate)
}

fn graph_predicate_scope_keys(
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

fn predicate_status_transition_allowed(old: &str, new: &str) -> bool {
	matches!(
		(old, new),
		("pending", "active") | ("pending", "deprecated") | ("active", "deprecated")
	)
}

fn stable_sort_aliases(aliases: &mut [GraphPredicateAlias]) {
	aliases.sort_by(|a, b| {
		a.created_at
			.cmp(&b.created_at)
			.then_with(|| a.alias_norm.cmp(&b.alias_norm))
			.then_with(|| a.alias.cmp(&b.alias))
	});
}

fn to_predicate_response(predicate: GraphPredicate) -> AdminGraphPredicateResponse {
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

fn to_alias_response(alias: GraphPredicateAlias) -> AdminGraphPredicateAliasResponse {
	AdminGraphPredicateAliasResponse {
		alias_id: alias.alias_id,
		predicate_id: alias.predicate_id,
		scope_key: alias.scope_key,
		alias: alias.alias,
		alias_norm: alias.alias_norm,
		created_at: alias.created_at,
	}
}

fn map_storage_error(err: StorageError) -> Error {
	match err {
		StorageError::InvalidArgument(message) => Error::InvalidRequest { message },
		StorageError::NotFound(message) => Error::NotFound { message },
		StorageError::Conflict(message) => Error::Conflict { message },
		StorageError::Sqlx(err) => Error::Storage { message: err.to_string() },
		StorageError::Qdrant(err) => Error::Qdrant { message: err.to_string() },
	}
}
