use crate::{
	Error,
	search::{
		self, DEFAULT_BOUNDED_CANDIDATES_LIMIT, DEFAULT_BOUNDED_STAGE_ITEMS_LIMIT,
		DEFAULT_FULL_CANDIDATES_LIMIT, DEFAULT_FULL_STAGE_ITEMS_LIMIT, ElfService,
		MAX_TRACE_BUNDLE_CANDIDATES_LIMIT, MAX_TRACE_BUNDLE_ITEMS_LIMIT, OffsetDateTime, Result,
		TRACE_BUNDLE_SCHEMA_V1, TraceBundleGetRequest, TraceBundleMode, TraceBundleResponse,
		TraceCandidateSnapshotRow, TraceGetRequest, ranking,
	},
};

impl ElfService {
	/// Loads a trace bundle with optional trajectory and replay candidates.
	pub async fn trace_bundle_get(
		&self,
		req: TraceBundleGetRequest,
	) -> Result<TraceBundleResponse> {
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
		let mut stages = search::load_trace_trajectory_stages(&self.db.pool, req.trace_id).await?;

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
