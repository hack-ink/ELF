use crate::{
	ElfService, Error, Result,
	admin_graph_predicates::{
		helpers::{self, PredicateAccess, map_storage_error},
		service::auth,
		types::{AdminGraphPredicatePatchRequest, AdminGraphPredicateResponse},
	},
};
use elf_storage::graph;

impl ElfService {
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

		let allow_global_mutation = auth::is_super_admin_token_id(self, req.token_id.as_deref());
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

		let new_status = resolve_new_status(status, old_status.as_str())?;
		let new_cardinality = resolve_new_cardinality(cardinality)?;
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
}

fn resolve_new_status(status: Option<&str>, old_status: &str) -> Result<Option<String>> {
	match status {
		None => Ok(None),
		Some(raw) => {
			let raw = raw.to_string();

			if !matches!(raw.as_str(), "pending" | "active" | "deprecated") {
				return Err(Error::InvalidRequest {
					message: "status must be one of pending|active|deprecated.".to_string(),
				});
			}
			if raw != old_status
				&& !helpers::predicate_status_transition_allowed(old_status, raw.as_str())
			{
				return Err(Error::Conflict {
					message: format!(
						"Invalid graph predicate status transition; from={old_status} to={raw}.",
					),
				});
			}

			Ok(Some(raw))
		},
	}
}

fn resolve_new_cardinality(cardinality: Option<&str>) -> Result<Option<String>> {
	match cardinality {
		None => Ok(None),
		Some(raw) => {
			let raw = raw.to_string();

			if !matches!(raw.as_str(), "single" | "multi") {
				return Err(Error::InvalidRequest {
					message: "cardinality must be one of single|multi.".to_string(),
				});
			}

			Ok(Some(raw))
		},
	}
}
