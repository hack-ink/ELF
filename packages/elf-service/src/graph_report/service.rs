use crate::{
	access, graph_query,
	graph_report::{
		self, ELF_GRAPH_REPORT_SCHEMA_V1, ElfService, GraphReportEntity, GraphReportExplain,
		GraphReportPredicate, GraphReportRequest, GraphReportResponse, GraphReportRowsFetchParams,
		Result,
	},
	search,
};

impl ElfService {
	/// Builds a source-backed graph report for one subject entity.
	pub async fn graph_report(&self, req: GraphReportRequest) -> Result<GraphReportResponse> {
		let prepared = graph_report::validate_graph_report_request(req)?;
		let allowed_scopes =
			search::resolve_read_profile_scopes(&self.cfg, prepared.read_profile.as_str())?;
		let effective_scopes = graph_query::resolve_effective_scopes(
			&allowed_scopes,
			prepared.requested_scopes.as_slice(),
		)?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope.trim() == "org_shared");
		let mut conn = self.db.pool.acquire().await?;
		let subject = graph_report::resolve_subject(
			&mut conn,
			&prepared.tenant_id,
			&prepared.project_id,
			prepared.subject,
		)
		.await?;
		let predicate = graph_report::resolve_predicate(
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
		let rows = graph_report::fetch_graph_report_rows(
			&mut conn,
			GraphReportRowsFetchParams {
				tenant_id: prepared.tenant_id.as_str(),
				project_id: prepared.project_id.as_str(),
				subject_entity_id: subject.entity_id,
				scopes: effective_scopes.as_slice(),
				actor: prepared.agent_id.as_str(),
				shared_scope_keys: shared_scope_keys.as_slice(),
				predicate_id,
				limit_plus_one: (prepared.limit as i64) + 1,
			},
		)
		.await?;
		let queried_rows = rows.len();
		let (rows, truncated) = graph_report::truncate_report_rows(rows, prepared.limit);
		let facts = graph_report::build_report_facts(rows, prepared.as_of);
		let summary = graph_report::summarize_report_facts(&facts);
		let topic_map = graph_report::build_topic_map(&subject, &facts);
		let explain = if prepared.explain {
			Some(GraphReportExplain {
				schema: ELF_GRAPH_REPORT_SCHEMA_V1.to_string(),
				as_of: prepared.as_of,
				requested_limit: prepared.limit as u32,
				allowed_scopes,
				effective_scopes: effective_scopes.clone(),
				queried_rows,
				returned_rows: facts.len(),
				truncated,
			})
		} else {
			None
		};

		Ok(GraphReportResponse {
			schema: ELF_GRAPH_REPORT_SCHEMA_V1.to_string(),
			as_of: prepared.as_of,
			subject: GraphReportEntity {
				entity_id: subject.entity_id,
				canonical: subject.canonical,
				kind: subject.kind,
			},
			predicate: predicate.map(|resolved| GraphReportPredicate {
				predicate_id: resolved.id,
				canonical: resolved.canonical,
			}),
			scopes: effective_scopes,
			summary,
			topic_map,
			facts,
			explain,
		})
	}
}
