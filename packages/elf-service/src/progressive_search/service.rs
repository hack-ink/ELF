use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
	ElfService, Error, Result, SearchRequest,
	progressive_search::{
		details,
		storage::{self},
		types::{
			NewSearchSession, SESSION_SLIDING_TTL_HOURS, SearchIndexItem,
			SearchIndexPlannedResponse, SearchIndexResponse, SearchSessionItemRecord,
			SearchSessionMode, SearchSessionizePath, SearchSessionizedOutput,
		},
	},
	structured_fields,
};

impl ElfService {
	/// Runs the default progressive-search path and returns indexed results.
	pub async fn search(&self, req: SearchRequest) -> Result<SearchIndexResponse> {
		let response = self.search_planned(req).await?;

		Ok(SearchIndexResponse {
			trace_id: response.trace_id,
			search_session_id: response.search_session_id,
			expires_at: response.expires_at,
			items: response.items,
			trajectory_summary: response.trajectory_summary,
		})
	}

	/// Runs quick-find search and stores a quick session without a query plan.
	pub async fn search_quick(&self, req: SearchRequest) -> Result<SearchIndexResponse> {
		self.search_sessionized(req, SearchSessionizePath::Quick).await.map(|output| output.index)
	}

	/// Runs planned search and stores a session with a query plan.
	pub async fn search_planned(&self, req: SearchRequest) -> Result<SearchIndexPlannedResponse> {
		let output = self.search_sessionized(req, SearchSessionizePath::Planned).await?;
		let query_plan = output.query_plan.ok_or_else(|| Error::Storage {
			message: "Planned search response is missing query_plan.".to_string(),
		})?;

		Ok(SearchIndexPlannedResponse {
			trace_id: output.index.trace_id,
			search_session_id: output.index.search_session_id,
			expires_at: output.index.expires_at,
			items: output.index.items,
			trajectory_summary: output.index.trajectory_summary,
			query_plan,
		})
	}

	async fn search_sessionized(
		&self,
		req: SearchRequest,
		path: SearchSessionizePath,
	) -> Result<SearchSessionizedOutput> {
		let top_k = req.top_k.unwrap_or(self.cfg.memory.top_k).max(1);
		let candidate_k = req.candidate_k.unwrap_or(self.cfg.memory.candidate_k).max(top_k);
		let mut raw_req = req.clone();

		raw_req.top_k = Some(candidate_k);
		raw_req.record_hits = Some(false);

		let (trace_id, raw_items, trajectory_summary, query_plan) = match path {
			SearchSessionizePath::Quick => {
				let raw = self.search_raw_quick(raw_req).await?;

				(raw.trace_id, raw.items, raw.trajectory_summary, None)
			},
			SearchSessionizePath::Planned => {
				let raw = self.search_raw_planned(raw_req).await?;

				(raw.trace_id, raw.items, raw.trajectory_summary, Some(raw.query_plan))
			},
		};
		let now = OffsetDateTime::now_utc();
		let expires_at = now + Duration::hours(SESSION_SLIDING_TTL_HOURS);
		let search_session_id = Uuid::new_v4();
		let note_ids: Vec<Uuid> = raw_items.iter().map(|item| item.note_id).collect();
		let structured_by_note =
			structured_fields::fetch_structured_fields(&self.db.pool, &note_ids).await?;
		let mut items = Vec::with_capacity(raw_items.len());

		for (idx, item) in raw_items.iter().enumerate() {
			let summary = structured_by_note
				.get(&item.note_id)
				.and_then(|value| value.summary.clone())
				.unwrap_or_else(|| {
					details::build_summary(&item.snippet, self.cfg.memory.max_note_chars as usize)
				});

			items.push(SearchSessionItemRecord {
				rank: idx as u32 + 1,
				note_id: item.note_id,
				chunk_id: item.chunk_id,
				final_score: item.final_score,
				updated_at: item.updated_at,
				expires_at: item.expires_at,
				r#type: item.r#type.clone(),
				key: item.key.clone(),
				scope: item.scope.clone(),
				importance: item.importance,
				confidence: item.confidence,
				summary,
			});
		}

		storage::store_search_session(
			&self.db.pool,
			NewSearchSession {
				search_session_id,
				trace_id,
				tenant_id: &req.tenant_id,
				project_id: &req.project_id,
				agent_id: &req.agent_id,
				read_profile: &req.read_profile,
				query: &req.query,
				mode: SearchSessionMode::from(path),
				query_plan: query_plan.as_ref(),
				trajectory_summary: trajectory_summary.as_ref(),
				items: &items,
				created_at: now,
				expires_at,
			},
		)
		.await?;

		let response_items: Vec<SearchIndexItem> =
			items.into_iter().take(top_k as usize).map(|item| item.to_index_item()).collect();

		Ok(SearchSessionizedOutput {
			index: SearchIndexResponse {
				trace_id,
				search_session_id,
				expires_at,
				items: response_items,
				trajectory_summary,
			},
			query_plan,
		})
	}
}
