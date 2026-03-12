//! Administrative graph-predicate APIs.

use serde::Serialize;
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{ElfService, Result};
use elf_config::SecurityAuthRole;
use elf_storage::{
	graph,
	models::{GraphPredicate, GraphPredicateAlias},
};

const GRAPH_PREDICATE_SCOPE_GLOBAL: &str = "__global__";
const GRAPH_PREDICATE_SCOPE_PROJECT_PREFIX: &str = "__project__:";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

/// Request payload for listing graph predicates visible in admin scope.
#[derive(Clone, Debug)]
pub struct AdminGraphPredicatesListRequest {
	/// Tenant to query within.
	pub tenant_id: String,
	/// Project to query within.
	pub project_id: String,
	/// Agent requesting the list.
	pub agent_id: String,
	/// Optional admin scope filter.
	pub scope: Option<String>,
}

/// Request payload for patching a graph predicate.
#[derive(Clone, Debug)]
pub struct AdminGraphPredicatePatchRequest {
	/// Tenant to query within.
	pub tenant_id: String,
	/// Project to query within.
	pub project_id: String,
	/// Agent requesting the mutation.
	pub agent_id: String,
	/// Optional auth token identifier used for super-admin checks.
	pub token_id: Option<String>,
	/// Predicate identifier to mutate.
	pub predicate_id: Uuid,
	/// Optional new predicate status.
	pub status: Option<String>,
	/// Optional new cardinality value.
	pub cardinality: Option<String>,
}

/// Request payload for adding a graph predicate alias.
#[derive(Clone, Debug)]
pub struct AdminGraphPredicateAliasAddRequest {
	/// Tenant to query within.
	pub tenant_id: String,
	/// Project to query within.
	pub project_id: String,
	/// Agent requesting the mutation.
	pub agent_id: String,
	/// Optional auth token identifier used for super-admin checks.
	pub token_id: Option<String>,
	/// Predicate identifier to extend.
	pub predicate_id: Uuid,
	/// Alias surface to add.
	pub alias: String,
}

/// Request payload for listing graph predicate aliases.
#[derive(Clone, Debug)]
pub struct AdminGraphPredicateAliasesListRequest {
	/// Tenant to query within.
	pub tenant_id: String,
	/// Project to query within.
	pub project_id: String,
	/// Agent requesting the list.
	pub agent_id: String,
	/// Predicate identifier to inspect.
	pub predicate_id: Uuid,
}

/// Serialized graph predicate returned by admin APIs.
#[derive(Clone, Debug, Serialize)]
pub struct AdminGraphPredicateResponse {
	/// Predicate identifier.
	pub predicate_id: Uuid,
	/// Predicate scope key.
	pub scope_key: String,
	/// Tenant scope when tenant-specific.
	pub tenant_id: Option<String>,
	/// Project scope when project-specific.
	pub project_id: Option<String>,
	/// Canonical predicate surface.
	pub canonical: String,
	/// Normalized canonical predicate surface.
	pub canonical_norm: String,
	/// Cardinality policy.
	pub cardinality: String,
	/// Lifecycle status.
	pub status: String,
	#[serde(with = "crate::time_serde")]
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
	#[serde(with = "crate::time_serde")]
	/// Last update timestamp.
	pub updated_at: OffsetDateTime,
}

/// Serialized graph predicate alias returned by admin APIs.
#[derive(Clone, Debug, Serialize)]
pub struct AdminGraphPredicateAliasResponse {
	/// Alias identifier.
	pub alias_id: Uuid,
	/// Predicate identifier that owns the alias.
	pub predicate_id: Uuid,
	/// Scope key where the alias resolves.
	pub scope_key: String,
	/// Alias surface.
	pub alias: String,
	/// Normalized alias surface.
	pub alias_norm: String,
	#[serde(with = "crate::time_serde")]
	/// Creation timestamp.
	pub created_at: OffsetDateTime,
}

/// Response payload for listing graph predicates.
#[derive(Clone, Debug, Serialize)]
pub struct AdminGraphPredicatesListResponse {
	/// Returned predicates.
	pub predicates: Vec<AdminGraphPredicateResponse>,
}

/// Response payload for graph predicate alias operations.
#[derive(Clone, Debug, Serialize)]
pub struct AdminGraphPredicateAliasesResponse {
	/// Predicate identifier.
	pub predicate_id: Uuid,
	/// Returned aliases.
	pub aliases: Vec<AdminGraphPredicateAliasResponse>,
}

impl ElfService {
	fn is_super_admin_token_id(&self, token_id: Option<&str>) -> bool {
		if self.cfg.security.auth_mode.trim() != "static_keys" {
			return false;
		}

		let Some(token_id) = token_id.map(str::trim).filter(|value| !value.is_empty()) else {
			return false;
		};

		self.cfg
			.security
			.auth_keys
			.iter()
			.any(|key| key.token_id == token_id && matches!(key.role, SecurityAuthRole::SuperAdmin))
	}

	/// Lists graph predicates visible to the caller's admin context.
	pub async fn admin_graph_predicates_list(
		&self,
		req: AdminGraphPredicatesListRequest,
	) -> Result<AdminGraphPredicatesListResponse> {
		let raw = req.scope.as_deref().unwrap_or("all");
		let scope =
			AdminGraphPredicateScope::parse(raw).ok_or_else(|| crate::Error::InvalidRequest {
				message: "scope must be one of tenant_project|project|global|all".to_string(),
			})?;
		let scope_keys =
			graph_predicate_scope_keys(req.tenant_id.as_str(), req.project_id.as_str(), scope);
		let mut conn = self.db.pool.acquire().await?;
		let predicates = graph::list_predicates_by_scope_keys(&mut conn, &scope_keys)
			.await
			.map_err(map_storage_error)?;
		let predicates = predicates.into_iter().map(to_predicate_response).collect();

		Ok(AdminGraphPredicatesListResponse { predicates })
	}

	/// Updates a mutable graph predicate field inside the allowed admin scope.
	pub async fn admin_graph_predicate_patch(
		&self,
		req: AdminGraphPredicatePatchRequest,
	) -> Result<AdminGraphPredicateResponse> {
		if req.status.is_none() && req.cardinality.is_none() {
			return Err(crate::Error::InvalidRequest {
				message: "At least one of status or cardinality is required.".to_string(),
			});
		}

		let status = req.status.as_deref().map(str::trim);

		if status.is_some_and(str::is_empty) {
			return Err(crate::Error::InvalidRequest {
				message: "status must be non-empty.".to_string(),
			});
		}

		let cardinality = req.cardinality.as_deref().map(str::trim);

		if cardinality.is_some_and(str::is_empty) {
			return Err(crate::Error::InvalidRequest {
				message: "cardinality must be non-empty.".to_string(),
			});
		}

		let allow_global_mutation = self.is_super_admin_token_id(req.token_id.as_deref());
		let mut conn = self.db.pool.acquire().await?;
		let existing = load_predicate_in_context(
			&mut conn,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.predicate_id,
			PredicateAccess::Mutate,
			allow_global_mutation,
		)
		.await?;
		let old_status = existing.status.clone();
		let old_cardinality = existing.cardinality.clone();

		if old_status == "deprecated" {
			return Err(crate::Error::Conflict {
				message: "graph predicate is deprecated and cannot be modified.".to_string(),
			});
		}

		let new_status = match status {
			None => None,
			Some(raw) => {
				let raw = raw.to_string();

				if !matches!(raw.as_str(), "pending" | "active" | "deprecated") {
					return Err(crate::Error::InvalidRequest {
						message: "status must be one of pending|active|deprecated.".to_string(),
					});
				}
				if raw != old_status
					&& !predicate_status_transition_allowed(old_status.as_str(), raw.as_str())
				{
					return Err(crate::Error::Conflict {
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
					return Err(crate::Error::InvalidRequest {
						message: "cardinality must be one of single|multi.".to_string(),
					});
				}

				Some(raw)
			},
		};
		let updated = graph::update_predicate_guarded(
			&mut conn,
			req.predicate_id,
			old_status.as_str(),
			old_cardinality.as_str(),
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

	/// Adds an alias to a mutable graph predicate.
	pub async fn admin_graph_predicate_alias_add(
		&self,
		req: AdminGraphPredicateAliasAddRequest,
	) -> Result<AdminGraphPredicateAliasesResponse> {
		let alias = req.alias.trim();

		if alias.is_empty() {
			return Err(crate::Error::InvalidRequest {
				message: "alias must be non-empty.".to_string(),
			});
		}

		let allow_global_mutation = self.is_super_admin_token_id(req.token_id.as_deref());
		let mut conn = self.db.pool.acquire().await?;
		let predicate = load_predicate_in_context(
			&mut conn,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.predicate_id,
			PredicateAccess::Mutate,
			allow_global_mutation,
		)
		.await?;

		if predicate.status == "deprecated" {
			return Err(crate::Error::Conflict {
				message: "graph predicate is deprecated and cannot be modified.".to_string(),
			});
		}

		graph::add_predicate_alias(&mut conn, req.predicate_id, alias)
			.await
			.map_err(map_storage_error)?;

		tracing::info!(
			actor_agent_id = %req.agent_id,
			predicate_id = %req.predicate_id,
			alias = %alias,
			"Admin graph predicate alias added."
		);

		let mut aliases = graph::list_predicate_aliases(&mut conn, req.predicate_id)
			.await
			.map_err(map_storage_error)?;

		stable_sort_aliases(&mut aliases);

		let aliases = aliases.into_iter().map(to_alias_response).collect();

		Ok(AdminGraphPredicateAliasesResponse { predicate_id: req.predicate_id, aliases })
	}

	/// Lists aliases for a graph predicate visible in admin scope.
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
			false,
		)
		.await?;

		let mut aliases = graph::list_predicate_aliases(&mut conn, req.predicate_id)
			.await
			.map_err(map_storage_error)?;

		stable_sort_aliases(&mut aliases);

		let aliases = aliases.into_iter().map(to_alias_response).collect();

		Ok(AdminGraphPredicateAliasesResponse { predicate_id: req.predicate_id, aliases })
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PredicateAccess {
	Read,
	Mutate,
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

fn map_storage_error(err: elf_storage::Error) -> crate::Error {
	match err {
		elf_storage::Error::InvalidArgument(message) => crate::Error::InvalidRequest { message },
		elf_storage::Error::NotFound(message) => crate::Error::NotFound { message },
		elf_storage::Error::Conflict(message) => crate::Error::Conflict { message },
		elf_storage::Error::Sqlx(err) => crate::Error::Storage { message: err.to_string() },
		elf_storage::Error::Qdrant(err) => crate::Error::Qdrant { message: err.to_string() },
	}
}

async fn load_predicate_in_context(
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
