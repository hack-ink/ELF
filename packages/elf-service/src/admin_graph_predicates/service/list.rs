use crate::{
	ElfService, Error, Result,
	admin_graph_predicates::{
		helpers::{self, AdminGraphPredicateScope, map_storage_error},
		types::{AdminGraphPredicatesListRequest, AdminGraphPredicatesListResponse},
	},
};
use elf_storage::graph;

impl ElfService {
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
}
