use crate::{
	Error,
	search::{
		self, ElfService, Result, SearchExplain, SearchExplainItem, SearchExplainRequest,
		SearchExplainResponse, SearchExplainTraceRow, SearchTrace, ranking,
	},
};

impl ElfService {
	/// Loads the explain payload for one result handle.
	pub async fn search_explain(&self, req: SearchExplainRequest) -> Result<SearchExplainResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id and project_id are required.".to_string(),
			});
		}

		let row = sqlx::query_as::<_, SearchExplainTraceRow>(
			"\
SELECT
	t.trace_id,
	t.tenant_id,
	t.project_id,
	t.agent_id,
	t.read_profile,
	t.query,
	t.expansion_mode,
	t.expanded_queries,
	t.allowed_scopes,
	t.candidate_count,
	t.top_k,
	t.config_snapshot,
	t.trace_version,
	t.created_at,
	i.item_id,
	i.note_id,
	i.chunk_id,
	i.rank,
	i.explain
FROM search_trace_items i
JOIN search_traces t ON i.trace_id = t.trace_id

WHERE i.item_id = $1 AND t.tenant_id = $2 AND t.project_id = $3",
		)
		.bind(req.result_handle)
		.bind(tenant_id)
		.bind(project_id)
		.fetch_optional(&self.db.pool)
		.await?;
		let Some(row) = row else {
			return Err(Error::InvalidRequest {
				message: "Unknown result_handle or trace not yet persisted.".to_string(),
			});
		};
		let expanded_queries: Vec<String> =
			ranking::decode_json(row.expanded_queries, "expanded_queries")?;
		let allowed_scopes: Vec<String> =
			ranking::decode_json(row.allowed_scopes, "allowed_scopes")?;
		let config_snapshot = row.config_snapshot;
		let explain: SearchExplain = ranking::decode_json(row.explain, "explain")?;
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
		let item = SearchExplainItem {
			result_handle: row.item_id,
			note_id: row.note_id,
			chunk_id: row.chunk_id,
			rank: row.rank as u32,
			explain,
		};
		let trajectory = search::load_item_trajectory(
			&self.db.pool,
			row.trace_id,
			row.item_id,
			row.note_id,
			row.chunk_id,
		)
		.await?;

		Ok(SearchExplainResponse { trace, item, trajectory })
	}
}
