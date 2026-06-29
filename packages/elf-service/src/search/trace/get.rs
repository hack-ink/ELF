use crate::{
	Error,
	search::{
		self, ElfService, Result, SearchExplain, SearchExplainItem, SearchTrace,
		SearchTraceItemRow, SearchTraceRow, TraceGetRequest, TraceGetResponse, ranking,
	},
};

impl ElfService {
	/// Loads trace metadata and explained items for one trace.
	pub async fn trace_get(&self, req: TraceGetRequest) -> Result<TraceGetResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();

		if req.agent_id.trim().is_empty() {
			return Err(Error::InvalidRequest { message: "agent_id is required.".to_string() });
		}
		if tenant_id.is_empty() || project_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id and project_id are required.".to_string(),
			});
		}

		let row = sqlx::query_as::<_, SearchTraceRow>(
			"\
SELECT
	trace_id,
	tenant_id,
	project_id,
	agent_id,
	read_profile,
	query,
	expansion_mode,
	expanded_queries,
	allowed_scopes,
	candidate_count,
	top_k,
	config_snapshot,
	trace_version,
	created_at
FROM search_traces
WHERE trace_id = $1 AND tenant_id = $2 AND project_id = $3",
		)
		.bind(req.trace_id)
		.bind(tenant_id)
		.bind(project_id)
		.fetch_optional(&self.db.pool)
		.await?;
		let Some(row) = row else {
			return Err(Error::InvalidRequest { message: "Unknown trace_id.".to_string() });
		};
		let expanded_queries: Vec<String> =
			ranking::decode_json(row.expanded_queries, "expanded_queries")?;
		let allowed_scopes: Vec<String> =
			ranking::decode_json(row.allowed_scopes, "allowed_scopes")?;
		let config_snapshot = row.config_snapshot;
		let trace = SearchTrace {
			trace_id: row.trace_id,
			tenant_id: row.tenant_id,
			project_id: row.project_id,
			agent_id: row.agent_id,
			read_profile: row.read_profile,
			query: row.query,
			expansion_mode: row.expansion_mode,
			expanded_queries,
			allowed_scopes,
			candidate_count: row.candidate_count as u32,
			top_k: row.top_k as u32,
			config_snapshot,
			created_at: row.created_at,
			trace_version: row.trace_version,
		};
		let item_rows = sqlx::query_as::<_, SearchTraceItemRow>(
			"\
SELECT
	item_id,
	note_id,
	chunk_id,
	rank,
	explain
FROM search_trace_items
WHERE trace_id = $1
ORDER BY rank ASC",
		)
		.bind(req.trace_id)
		.fetch_all(&self.db.pool)
		.await?;
		let mut items = Vec::with_capacity(item_rows.len());

		for row in item_rows {
			let explain: SearchExplain = ranking::decode_json(row.explain, "explain")?;

			items.push(SearchExplainItem {
				result_handle: row.item_id,
				note_id: row.note_id,
				chunk_id: row.chunk_id,
				rank: row.rank as u32,
				explain,
			});
		}

		let trajectory_summary =
			search::load_trace_trajectory_summary(&self.db.pool, req.trace_id).await?;

		Ok(TraceGetResponse { trace, items, trajectory_summary })
	}
}
