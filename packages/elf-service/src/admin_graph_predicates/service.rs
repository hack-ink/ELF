use crate::{
	ElfService, Error, Result,
	admin_graph_predicates::{
		helpers::{
			self, AdminGraphPredicateScope, PredicateAccess, map_storage_error, to_alias_response,
		},
		types::{
			AdminGraphPredicateAliasAddRequest, AdminGraphPredicateAliasesListRequest,
			AdminGraphPredicateAliasesResponse, AdminGraphPredicatePatchRequest,
			AdminGraphPredicateResponse, AdminGraphPredicatesListRequest,
			AdminGraphPredicatesListResponse,
		},
	},
};
use elf_config::SecurityAuthRole;
use elf_storage::graph;

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
		let scope = AdminGraphPredicateScope::parse(raw).ok_or_else(|| Error::InvalidRequest {
			message: "scope must be one of tenant_project|project|global|all".to_string(),
		})?;
		let scope_keys = helpers::graph_predicate_scope_keys(
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			scope,
		);
		let mut conn = self.db.pool.acquire().await?;
		let predicates = graph::list_predicates_by_scope_keys(&mut conn, &scope_keys)
			.await
			.map_err(map_storage_error)?;
		let predicates = predicates
			.into_iter()
			.map(crate::admin_graph_predicates::helpers::to_predicate_response)
			.collect();

		Ok(AdminGraphPredicatesListResponse { predicates })
	}

	/// Updates a mutable graph predicate field inside the allowed admin scope.
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

		let allow_global_mutation = self.is_super_admin_token_id(req.token_id.as_deref());
		let mut conn = self.db.pool.acquire().await?;
		let existing = helpers::load_predicate_in_context(
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
					&& !helpers::predicate_status_transition_allowed(
						old_status.as_str(),
						raw.as_str(),
					) {
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

		Ok(helpers::to_predicate_response(updated))
	}

	/// Adds an alias to a mutable graph predicate.
	pub async fn admin_graph_predicate_alias_add(
		&self,
		req: AdminGraphPredicateAliasAddRequest,
	) -> Result<AdminGraphPredicateAliasesResponse> {
		let alias = req.alias.trim();

		if alias.is_empty() {
			return Err(Error::InvalidRequest { message: "alias must be non-empty.".to_string() });
		}

		let allow_global_mutation = self.is_super_admin_token_id(req.token_id.as_deref());
		let mut conn = self.db.pool.acquire().await?;
		let predicate = helpers::load_predicate_in_context(
			&mut conn,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.predicate_id,
			PredicateAccess::Mutate,
			allow_global_mutation,
		)
		.await?;

		if predicate.status == "deprecated" {
			return Err(Error::Conflict {
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

		helpers::stable_sort_aliases(&mut aliases);

		let aliases = aliases.into_iter().map(to_alias_response).collect();

		Ok(AdminGraphPredicateAliasesResponse { predicate_id: req.predicate_id, aliases })
	}

	/// Lists aliases for a graph predicate visible in admin scope.
	pub async fn admin_graph_predicate_aliases_list(
		&self,
		req: AdminGraphPredicateAliasesListRequest,
	) -> Result<AdminGraphPredicateAliasesResponse> {
		let mut conn = self.db.pool.acquire().await?;

		helpers::load_predicate_in_context(
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

		helpers::stable_sort_aliases(&mut aliases);

		let aliases = aliases.into_iter().map(to_alias_response).collect();

		Ok(AdminGraphPredicateAliasesResponse { predicate_id: req.predicate_id, aliases })
	}
}
