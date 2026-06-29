use crate::{
	access,
	graph_query::{
		self, ElfService, GraphQueryEntity, GraphQueryPredicate, GraphQueryRequest,
		GraphQueryResponse, GraphQueryRowsFetchParams, OffsetDateTime, Result,
	},
	search,
};

impl ElfService {
	/// Resolves a subject and returns active graph facts visible to the caller.
	pub async fn graph_query(&self, req: GraphQueryRequest) -> Result<GraphQueryResponse> {
		let prepared = graph_query::validate_graph_query_request(req)?;
		let allowed_scopes =
			search::resolve_read_profile_scopes(&self.cfg, prepared.read_profile.as_str())?;
		let effective_scopes = graph_query::resolve_effective_scopes(
			&allowed_scopes,
			prepared.requested_scopes.as_slice(),
		)?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope.trim() == "org_shared");
		let mut conn = self.db.pool.acquire().await?;
		let subject = graph_query::resolve_subject(
			&mut conn,
			&prepared.tenant_id,
			&prepared.project_id,
			prepared.subject,
		)
		.await?;
		let predicate = graph_query::resolve_predicate(
			&mut conn,
			&prepared.tenant_id,
			&prepared.project_id,
			prepared.predicate,
		)
		.await?;
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			conn.as_mut(),
			prepared.tenant_id.as_str(),
			prepared.project_id.as_str(),
			prepared.agent_id.as_str(),
			org_shared_allowed,
		)
		.await?;
		let shared_scope_keys: Vec<String> = shared_grants
			.into_iter()
			.map(|item| format!("{}:{}", item.scope, item.space_owner_agent_id))
			.collect();
		let predicate_id = predicate.as_ref().map(|predicate| predicate.id);
		let read_at = OffsetDateTime::now_utc();
		let rows = graph_query::fetch_graph_query_rows(
			&mut conn,
			GraphQueryRowsFetchParams {
				tenant_id: prepared.tenant_id.as_str(),
				project_id: prepared.project_id.as_str(),
				subject_entity_id: subject.entity_id,
				scopes: effective_scopes.as_slice(),
				as_of: prepared.as_of,
				actor: prepared.agent_id.as_str(),
				shared_scope_keys: shared_scope_keys.as_slice(),
				predicate_id,
				limit_plus_one: (prepared.limit as i64) + 1,
			},
		)
		.await?;
		let facts = graph_query::graph_query_facts_from_rows(rows, read_at);
		let queried_rows = facts.len();
		let (facts, truncated) = graph_query::truncate_graph_query_facts(facts, prepared.limit);
		let explain = if prepared.explain {
			Some(graph_query::build_graph_query_explain(
				prepared.as_of,
				&allowed_scopes,
				&effective_scopes,
				prepared.limit,
				queried_rows,
				facts.len(),
				truncated,
			))
		} else {
			None
		};

		Ok(GraphQueryResponse {
			as_of: prepared.as_of,
			subject: GraphQueryEntity {
				entity_id: subject.entity_id,
				canonical: subject.canonical,
				kind: subject.kind,
			},
			predicate: predicate.map(|resolved| GraphQueryPredicate {
				predicate_id: resolved.id,
				canonical: resolved.canonical,
			}),
			scopes: effective_scopes,
			truncated,
			facts,
			explain,
		})
	}
}
