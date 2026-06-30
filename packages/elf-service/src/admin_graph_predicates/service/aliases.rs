use sqlx::PgConnection;
use uuid::Uuid;

use crate::{
	ElfService, Error, Result,
	admin_graph_predicates::{
		helpers::{self, PredicateAccess, map_storage_error, to_alias_response},
		service::auth,
		types::{
			AdminGraphPredicateAliasAddRequest, AdminGraphPredicateAliasesListRequest,
			AdminGraphPredicateAliasesResponse,
		},
	},
};
use elf_storage::graph;

impl ElfService {
	/// Adds an alias to a mutable graph predicate.
	pub async fn admin_graph_predicate_alias_add(
		&self,
		req: AdminGraphPredicateAliasAddRequest,
	) -> Result<AdminGraphPredicateAliasesResponse> {
		let alias = req.alias.trim();

		if alias.is_empty() {
			return Err(Error::InvalidRequest { message: "alias must be non-empty.".to_string() });
		}

		let allow_global_mutation = auth::is_super_admin_token_id(self, req.token_id.as_deref());
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

		list_aliases(&mut conn, req.predicate_id).await
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

		list_aliases(&mut conn, req.predicate_id).await
	}
}

async fn list_aliases(
	conn: &mut PgConnection,
	predicate_id: Uuid,
) -> Result<AdminGraphPredicateAliasesResponse> {
	let mut aliases =
		graph::list_predicate_aliases(conn, predicate_id).await.map_err(map_storage_error)?;

	helpers::stable_sort_aliases(&mut aliases);

	let aliases = aliases.into_iter().map(to_alias_response).collect();

	Ok(AdminGraphPredicateAliasesResponse { predicate_id, aliases })
}
