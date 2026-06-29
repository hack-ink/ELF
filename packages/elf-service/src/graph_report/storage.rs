use super::*;

pub(super) async fn fetch_graph_report_rows(
	conn: &mut PgConnection,
	params: GraphReportRowsFetchParams<'_>,
) -> Result<Vec<GraphReportFactRow>> {
	let rows = sqlx::query_as::<_, GraphReportFactRow>(GRAPH_REPORT_FACTS_SQL)
		.bind(params.tenant_id)
		.bind(params.project_id)
		.bind(params.subject_entity_id)
		.bind(params.scopes)
		.bind(OffsetDateTime::now_utc())
		.bind(params.actor)
		.bind(params.shared_scope_keys)
		.bind(params.limit_plus_one)
		.bind(GRAPH_REPORT_EVIDENCE_LIMIT)
		.bind(ORG_PROJECT_ID)
		.bind(params.predicate_id)
		.fetch_all(conn)
		.await?;

	Ok(rows)
}
