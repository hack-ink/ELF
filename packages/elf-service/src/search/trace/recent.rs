use crate::{
	Error,
	search::{
		DEFAULT_RECENT_TRACES_LIMIT, ElfService, MAX_RECENT_TRACES_LIMIT, RECENT_TRACES_SCHEMA_V1,
		RecentTraceHeader, Result, SearchRecentTraceRow, TraceRecentCursor, TraceRecentListRequest,
		TraceRecentListResponse,
	},
};

impl ElfService {
	/// Lists recent traces with cursor-based pagination.
	pub async fn trace_recent_list(
		&self,
		req: TraceRecentListRequest,
	) -> Result<TraceRecentListResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let caller_agent_id = req.agent_id.trim();
		let cursor_created_at = req.cursor_created_at;
		let cursor_trace_id = req.cursor_trace_id;
		let agent_id_filter = req.agent_id_filter.map(|value| value.trim().to_string());
		let read_profile = req.read_profile.map(|value| value.trim().to_string());
		let limit = req.limit.unwrap_or(DEFAULT_RECENT_TRACES_LIMIT);

		if cursor_created_at.is_some() != cursor_trace_id.is_some() {
			return Err(Error::InvalidRequest {
				message: "cursor_created_at and cursor_trace_id must be both set or both omitted."
					.to_string(),
			});
		}
		if caller_agent_id.is_empty() {
			return Err(Error::InvalidRequest { message: "agent_id is required.".to_string() });
		}
		if tenant_id.is_empty() || project_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id and project_id are required.".to_string(),
			});
		}
		if limit == 0 || limit > MAX_RECENT_TRACES_LIMIT {
			return Err(Error::InvalidRequest {
				message: format!("limit must be between 1 and {MAX_RECENT_TRACES_LIMIT}."),
			});
		}

		if let (Some(created_after), Some(created_before)) = (req.created_after, req.created_before)
			&& created_after >= created_before
		{
			return Err(Error::InvalidRequest {
				message: "created_after must be before created_before.".to_string(),
			});
		}

		let agent_id_filter = agent_id_filter.as_deref();
		let read_profile = read_profile.as_deref();
		let fetch_limit = (limit + 1).min(MAX_RECENT_TRACES_LIMIT + 1);
		let rows = sqlx::query_as::<_, SearchRecentTraceRow>(
			"\
SELECT
	trace_id,
	tenant_id,
	project_id,
	agent_id,
	read_profile,
	query,
	created_at
FROM search_traces
WHERE tenant_id = $1
	AND project_id = $2
	AND ($3::text IS NULL OR agent_id = $3)
	AND ($4::text IS NULL OR read_profile = $4)
	AND ($5::timestamptz IS NULL OR created_at > $5)
	AND ($6::timestamptz IS NULL OR created_at < $6)
	AND ($7::timestamptz IS NULL OR $8::uuid IS NULL OR (created_at, trace_id) < ($7, $8))
ORDER BY created_at DESC, trace_id DESC
LIMIT $9
",
		)
		.bind(tenant_id)
		.bind(project_id)
		.bind(agent_id_filter)
		.bind(read_profile)
		.bind(req.created_after)
		.bind(req.created_before)
		.bind(cursor_created_at)
		.bind(cursor_trace_id)
		.bind(fetch_limit as i64)
		.fetch_all(&self.db.pool)
		.await?;
		let next_cursor = if rows.len() > limit as usize {
			let cursor_row = &rows[limit as usize - 1];

			Some(TraceRecentCursor {
				created_at: cursor_row.created_at,
				trace_id: cursor_row.trace_id,
			})
		} else {
			None
		};
		let mut response_rows = rows;

		response_rows.truncate(limit as usize);

		let mut traces = Vec::with_capacity(response_rows.len());

		for row in response_rows {
			traces.push(RecentTraceHeader {
				trace_id: row.trace_id,
				tenant_id: row.tenant_id,
				project_id: row.project_id,
				agent_id: row.agent_id,
				read_profile: row.read_profile,
				query: row.query,
				created_at: row.created_at,
			});
		}

		Ok(TraceRecentListResponse {
			schema: RECENT_TRACES_SCHEMA_V1.to_string(),
			traces,
			next_cursor,
		})
	}
}
