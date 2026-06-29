use crate::graph_query::{
	GRAPH_QUERY_EVIDENCE_LIMIT, GRAPH_QUERY_FACTS_SQL, GraphQueryFactRow,
	GraphQueryRowsFetchParams, ORG_PROJECT_ID, PgConnection, Result,
};

pub(super) async fn fetch_graph_query_rows(
	conn: &mut PgConnection,
	params: GraphQueryRowsFetchParams<'_>,
) -> Result<Vec<GraphQueryFactRow>> {
	let GraphQueryRowsFetchParams {
		tenant_id,
		project_id,
		subject_entity_id,
		scopes,
		as_of,
		actor,
		shared_scope_keys,
		predicate_id,
		limit_plus_one,
	} = params;
	let rows = sqlx::query_as::<_, GraphQueryFactRow>(GRAPH_QUERY_FACTS_SQL)
		.bind(tenant_id)
		.bind(project_id)
		.bind(subject_entity_id)
		.bind(scopes)
		.bind(as_of)
		.bind(actor)
		.bind(shared_scope_keys)
		.bind(limit_plus_one)
		.bind(GRAPH_QUERY_EVIDENCE_LIMIT)
		.bind(ORG_PROJECT_ID)
		.bind(predicate_id)
		.fetch_all(conn)
		.await?;

	Ok(rows)
}
