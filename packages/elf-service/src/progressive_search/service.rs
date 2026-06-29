use std::collections::{HashMap, hash_set::HashSet};

use sqlx;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
	ElfService, Error, PayloadLevel, Result, SearchRequest,
	access::{self, ORG_PROJECT_ID},
	progressive_search::{
		details::{self, SearchDetailsBuildArgs},
		storage::{self},
		types::{
			NewSearchSession, SESSION_SLIDING_TTL_HOURS, SearchDetailsRequest,
			SearchDetailsResponse, SearchIndexItem, SearchIndexPlannedResponse,
			SearchIndexResponse, SearchSessionGetRequest, SearchSessionGetResponse,
			SearchSessionItemRecord, SearchSessionMode, SearchSessionizePath,
			SearchSessionizedOutput, SearchTimelineGroup, SearchTimelineRequest,
			SearchTimelineResponse,
		},
	},
	structured_fields,
};
use elf_storage::models::MemoryNote;

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

	/// Reloads a stored search session and optionally extends its TTL.
	pub async fn search_session_get(
		&self,
		req: SearchSessionGetRequest,
	) -> Result<SearchSessionGetResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let now = OffsetDateTime::now_utc();
		let session =
			storage::load_search_session(&self.db.pool, req.search_session_id, now).await?;

		details::validate_search_session_access(&session, tenant_id, project_id, agent_id)?;

		let touch = req.touch.unwrap_or(true);
		let expires_at = if touch {
			storage::touch_search_session(&self.db.pool, &session, now).await?
		} else {
			session.expires_at
		};
		let top_k = req.top_k.unwrap_or(self.cfg.memory.top_k).max(1);
		let items: Vec<SearchIndexItem> = session
			.items
			.into_iter()
			.take(top_k as usize)
			.map(|item| item.to_index_item())
			.collect();

		Ok(SearchSessionGetResponse {
			trace_id: session.trace_id,
			search_session_id: session.search_session_id,
			expires_at,
			items,
			mode: session.mode,
			query_plan: session.query_plan,
			trajectory_summary: session.trajectory_summary,
		})
	}

	/// Reprojects a stored search session into timeline groups.
	pub async fn search_timeline(
		&self,
		req: SearchTimelineRequest,
	) -> Result<SearchTimelineResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let now = OffsetDateTime::now_utc();
		let session =
			storage::load_search_session(&self.db.pool, req.search_session_id, now).await?;

		details::validate_search_session_access(&session, tenant_id, project_id, agent_id)?;

		let expires_at = storage::touch_search_session(&self.db.pool, &session, now).await?;
		let payload_level = req.payload_level;
		let group_by = req.group_by.unwrap_or_else(|| {
			if payload_level == PayloadLevel::L0 { "none".to_string() } else { "day".to_string() }
		});

		match group_by.as_str() {
			"day" => details::build_timeline_by_day(
				session.search_session_id,
				expires_at,
				&session.items,
			),
			"none" => Ok(SearchTimelineResponse {
				search_session_id: session.search_session_id,
				expires_at,
				groups: vec![SearchTimelineGroup {
					date: "all".to_string(),
					items: session
						.items
						.iter()
						.map(SearchSessionItemRecord::to_index_item)
						.collect(),
				}],
			}),
			_ => Err(Error::InvalidRequest {
				message: "group_by must be one of: day, none.".to_string(),
			}),
		}
	}

	/// Materializes selected note details out of a stored search session.
	pub async fn search_details(&self, req: SearchDetailsRequest) -> Result<SearchDetailsResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let now = OffsetDateTime::now_utc();
		let session =
			storage::load_search_session(&self.db.pool, req.search_session_id, now).await?;

		details::validate_search_session_access(&session, tenant_id, project_id, agent_id)?;

		let expires_at = storage::touch_search_session(&self.db.pool, &session, now).await?;
		let mut by_note_id: HashMap<Uuid, SearchSessionItemRecord> = HashMap::new();

		for item in &session.items {
			by_note_id.insert(item.note_id, item.clone());
		}

		let mut requested_in_session = Vec::new();
		let mut seen = HashSet::new();

		for note_id in &req.note_ids {
			if by_note_id.contains_key(note_id) && seen.insert(*note_id) {
				requested_in_session.push(*note_id);
			}
		}

		let mut notes_by_id = HashMap::new();

		if !requested_in_session.is_empty() {
			let rows: Vec<MemoryNote> = sqlx::query_as::<_, MemoryNote>(
				"\
SELECT *
FROM memory_notes
WHERE note_id = ANY($1::uuid[])
  AND tenant_id = $2
  AND (
    project_id = $3
    OR (project_id = $4 AND scope = 'org_shared')
  )",
			)
			.bind(requested_in_session.as_slice())
			.bind(session.tenant_id.as_str())
			.bind(session.project_id.as_str())
			.bind(ORG_PROJECT_ID)
			.fetch_all(&self.db.pool)
			.await?;

			for note in rows {
				notes_by_id.insert(note.note_id, note);
			}
		}

		let structured_by_note = if req.payload_level == PayloadLevel::L0 {
			HashMap::new()
		} else {
			structured_fields::fetch_structured_fields(
				&self.db.pool,
				requested_in_session.as_slice(),
			)
			.await?
		};
		let allowed_scopes = details::resolve_read_scopes(&self.cfg, &session.read_profile)?;
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&self.db.pool,
			session.tenant_id.as_str(),
			session.project_id.as_str(),
			agent_id,
			allowed_scopes.iter().any(|scope| scope == "org_shared"),
		)
		.await?;
		let record_hits = req.record_hits.unwrap_or(true);
		let details_args = SearchDetailsBuildArgs {
			session_items_by_note_id: &by_note_id,
			notes_by_id: &notes_by_id,
			structured_by_note: &structured_by_note,
			session: &session,
			shared_grants: &shared_grants,
			allowed_scopes: &allowed_scopes,
			now,
			record_hits_enabled: record_hits,
			payload_level: req.payload_level,
			max_note_chars: self.cfg.memory.max_note_chars as usize,
		};
		let (results, hits) = details::build_search_details_results(req.note_ids, details_args);

		if !hits.is_empty() {
			let mut tx = self.db.pool.begin().await?;

			storage::record_detail_hits(&mut *tx, &session.query, &hits, now).await?;

			tx.commit().await?;
		}

		Ok(SearchDetailsResponse {
			search_session_id: session.search_session_id,
			expires_at,
			results,
		})
	}
}
