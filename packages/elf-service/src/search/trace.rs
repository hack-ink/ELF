use super::*;

impl ElfService {
	/// Loads the explain payload for one result handle.
	pub async fn search_explain(&self, req: SearchExplainRequest) -> Result<SearchExplainResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() {
			return Err(crate::Error::InvalidRequest {
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
			return Err(crate::Error::InvalidRequest {
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
		let trajectory = load_item_trajectory(
			&self.db.pool,
			row.trace_id,
			row.item_id,
			row.note_id,
			row.chunk_id,
		)
		.await?;

		Ok(SearchExplainResponse { trace, item, trajectory })
	}

	/// Loads trace metadata and explained items for one trace.
	pub async fn trace_get(&self, req: TraceGetRequest) -> Result<TraceGetResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();

		if req.agent_id.trim().is_empty() {
			return Err(crate::Error::InvalidRequest {
				message: "agent_id is required.".to_string(),
			});
		}
		if tenant_id.is_empty() || project_id.is_empty() {
			return Err(crate::Error::InvalidRequest {
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
			return Err(crate::Error::InvalidRequest { message: "Unknown trace_id.".to_string() });
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

		let trajectory_summary = load_trace_trajectory_summary(&self.db.pool, req.trace_id).await?;

		Ok(TraceGetResponse { trace, items, trajectory_summary })
	}

	/// Loads full trajectory stages for one trace.
	pub async fn trace_trajectory_get(
		&self,
		req: TraceTrajectoryGetRequest,
	) -> Result<SearchTrajectoryResponse> {
		let base = self
			.trace_get(TraceGetRequest {
				tenant_id: req.tenant_id,
				project_id: req.project_id,
				agent_id: req.agent_id,
				trace_id: req.trace_id,
			})
			.await?;
		let stages = load_trace_trajectory_stages(&self.db.pool, req.trace_id).await?;
		let trajectory = build_trajectory_summary_from_stages(stages.as_slice());

		Ok(SearchTrajectoryResponse { trace: base.trace, trajectory, stages })
	}

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
			return Err(crate::Error::InvalidRequest {
				message: "cursor_created_at and cursor_trace_id must be both set or both omitted."
					.to_string(),
			});
		}
		if caller_agent_id.is_empty() {
			return Err(crate::Error::InvalidRequest {
				message: "agent_id is required.".to_string(),
			});
		}
		if tenant_id.is_empty() || project_id.is_empty() {
			return Err(crate::Error::InvalidRequest {
				message: "tenant_id and project_id are required.".to_string(),
			});
		}
		if limit == 0 || limit > MAX_RECENT_TRACES_LIMIT {
			return Err(crate::Error::InvalidRequest {
				message: format!("limit must be between 1 and {MAX_RECENT_TRACES_LIMIT}."),
			});
		}

		if let (Some(created_after), Some(created_before)) = (req.created_after, req.created_before)
			&& created_after >= created_before
		{
			return Err(crate::Error::InvalidRequest {
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

	/// Loads a trace bundle with optional trajectory and replay candidates.
	pub async fn trace_bundle_get(
		&self,
		req: TraceBundleGetRequest,
	) -> Result<TraceBundleResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();

		if req.agent_id.trim().is_empty() {
			return Err(crate::Error::InvalidRequest {
				message: "agent_id is required.".to_string(),
			});
		}
		if tenant_id.is_empty() || project_id.is_empty() {
			return Err(crate::Error::InvalidRequest {
				message: "tenant_id and project_id are required.".to_string(),
			});
		}

		let base = self
			.trace_get(TraceGetRequest {
				tenant_id: tenant_id.to_string(),
				project_id: project_id.to_string(),
				agent_id: req.agent_id.trim().to_string(),
				trace_id: req.trace_id,
			})
			.await?;
		let default_stage_items_limit = match req.mode {
			TraceBundleMode::Bounded => DEFAULT_BOUNDED_STAGE_ITEMS_LIMIT,
			TraceBundleMode::Full => DEFAULT_FULL_STAGE_ITEMS_LIMIT,
		};
		let default_candidates_limit = match req.mode {
			TraceBundleMode::Bounded => DEFAULT_BOUNDED_CANDIDATES_LIMIT,
			TraceBundleMode::Full => DEFAULT_FULL_CANDIDATES_LIMIT,
		};
		let stage_items_limit = req
			.stage_items_limit
			.unwrap_or(default_stage_items_limit)
			.min(MAX_TRACE_BUNDLE_ITEMS_LIMIT);
		let candidates_limit = req
			.candidates_limit
			.unwrap_or(default_candidates_limit)
			.min(MAX_TRACE_BUNDLE_CANDIDATES_LIMIT);
		let mut stages = load_trace_trajectory_stages(&self.db.pool, req.trace_id).await?;

		for stage in stages.iter_mut() {
			stage.items.truncate(stage_items_limit as usize);
		}

		let candidates = if candidates_limit == 0 {
			None
		} else {
			let candidate_rows = sqlx::query_as::<_, TraceCandidateSnapshotRow>(
				"\
SELECT candidate_snapshot
FROM search_trace_candidates
WHERE trace_id = $1
ORDER BY retrieval_rank ASC, candidate_id ASC
LIMIT $2
",
			)
			.bind(req.trace_id)
			.bind(candidates_limit as i32)
			.fetch_all(&self.db.pool)
			.await?;
			let mut candidates = Vec::with_capacity(candidate_rows.len());

			for row in candidate_rows {
				candidates
					.push(ranking::decode_json(row.candidate_snapshot, "candidate_snapshot")?);
			}

			if candidates.is_empty() { None } else { Some(candidates) }
		};

		Ok(TraceBundleResponse {
			schema: TRACE_BUNDLE_SCHEMA_V1.to_string(),
			generated_at: OffsetDateTime::now_utc(),
			trace: base.trace,
			items: base.items,
			trajectory_summary: base.trajectory_summary,
			stages,
			candidates,
		})
	}
}
