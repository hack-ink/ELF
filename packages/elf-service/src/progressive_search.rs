use std::{
	collections::{BTreeMap, HashMap, hash_map::DefaultHasher, hash_set::HashSet},
	hash::{Hash, Hasher},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgExecutor};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
	ElfService, Error, NoteFetchResponse, PayloadLevel, QueryPlan, Result, SearchRequest,
	access::{self, SharedSpaceGrantKey},
	structured_fields::StructuredFields,
};
use elf_config::Config;
use elf_domain::cjk;
use elf_storage::models::MemoryNote;

const SESSION_SLIDING_TTL_HOURS: i64 = 6;
const SESSION_ABSOLUTE_TTL_HOURS: i64 = 24;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchIndexItem {
	pub note_id: Uuid,
	pub r#type: String,
	pub key: Option<String>,
	pub scope: String,
	pub importance: f32,
	pub confidence: f32,
	#[serde(with = "crate::time_serde")]
	pub updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	pub expires_at: Option<OffsetDateTime>,
	pub final_score: f32,
	pub summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchIndexResponse {
	pub trace_id: Uuid,
	pub search_session_id: Uuid,
	#[serde(with = "crate::time_serde")]
	pub expires_at: OffsetDateTime,
	pub items: Vec<SearchIndexItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchIndexPlannedResponse {
	pub trace_id: Uuid,
	pub search_session_id: Uuid,
	#[serde(with = "crate::time_serde")]
	pub expires_at: OffsetDateTime,
	pub items: Vec<SearchIndexItem>,
	pub query_plan: QueryPlan,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchSessionGetRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub search_session_id: Uuid,
	#[serde(default)]
	pub payload_level: PayloadLevel,
	pub top_k: Option<u32>,
	pub touch: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchTimelineRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub search_session_id: Uuid,
	pub payload_level: PayloadLevel,
	pub group_by: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchTimelineGroup {
	pub date: String,
	pub items: Vec<SearchIndexItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchTimelineResponse {
	pub search_session_id: Uuid,
	#[serde(with = "crate::time_serde")]
	pub expires_at: OffsetDateTime,
	pub groups: Vec<SearchTimelineGroup>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchDetailsRequest {
	pub tenant_id: String,
	pub project_id: String,
	pub agent_id: String,
	pub search_session_id: Uuid,
	#[serde(default)]
	pub payload_level: PayloadLevel,
	pub note_ids: Vec<Uuid>,
	pub record_hits: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchDetailsError {
	pub code: String,
	pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchDetailsResult {
	pub note_id: Uuid,
	pub note: Option<NoteFetchResponse>,
	pub error: Option<SearchDetailsError>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchDetailsResponse {
	pub search_session_id: Uuid,
	#[serde(with = "crate::time_serde")]
	pub expires_at: OffsetDateTime,
	pub results: Vec<SearchDetailsResult>,
}

struct HitItem {
	note_id: Uuid,
	chunk_id: Uuid,
	rank: u32,
	final_score: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchSessionizePath {
	Quick,
	Planned,
}

struct SearchSessionizedOutput {
	index: SearchIndexResponse,
	query_plan: Option<QueryPlan>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SearchSessionItemRecord {
	rank: u32,
	note_id: Uuid,
	chunk_id: Uuid,
	final_score: f32,
	#[serde(with = "crate::time_serde")]
	updated_at: OffsetDateTime,
	#[serde(with = "crate::time_serde::option")]
	expires_at: Option<OffsetDateTime>,
	r#type: String,
	key: Option<String>,
	scope: String,
	importance: f32,
	confidence: f32,
	summary: String,
}
impl SearchSessionItemRecord {
	fn to_index_item(&self) -> SearchIndexItem {
		SearchIndexItem {
			note_id: self.note_id,
			r#type: self.r#type.clone(),
			key: self.key.clone(),
			scope: self.scope.clone(),
			importance: self.importance,
			confidence: self.confidence,
			updated_at: self.updated_at,
			expires_at: self.expires_at,
			final_score: self.final_score,
			summary: self.summary.clone(),
		}
	}
}

struct SearchSession {
	search_session_id: Uuid,
	trace_id: Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	read_profile: String,
	query: String,
	items: Vec<SearchSessionItemRecord>,
	created_at: OffsetDateTime,
	expires_at: OffsetDateTime,
}

#[derive(FromRow)]
struct SearchSessionRow {
	search_session_id: Uuid,
	trace_id: Uuid,
	tenant_id: String,
	project_id: String,
	agent_id: String,
	read_profile: String,
	query: String,
	items: Value,
	created_at: OffsetDateTime,
	expires_at: OffsetDateTime,
}

struct NewSearchSession<'a> {
	search_session_id: Uuid,
	trace_id: Uuid,
	tenant_id: &'a str,
	project_id: &'a str,
	agent_id: &'a str,
	read_profile: &'a str,
	query: &'a str,
	items: &'a [SearchSessionItemRecord],
	created_at: OffsetDateTime,
	expires_at: OffsetDateTime,
}

impl ElfService {
	pub async fn search(&self, req: SearchRequest) -> Result<SearchIndexResponse> {
		let response = self.search_planned(req).await?;

		Ok(SearchIndexResponse {
			trace_id: response.trace_id,
			search_session_id: response.search_session_id,
			expires_at: response.expires_at,
			items: response.items,
		})
	}

	pub async fn search_quick(&self, req: SearchRequest) -> Result<SearchIndexResponse> {
		self.search_sessionized(req, SearchSessionizePath::Quick).await.map(|output| output.index)
	}

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

		let (trace_id, raw_items, query_plan) = match path {
			SearchSessionizePath::Quick => {
				let raw = self.search_raw_quick(raw_req).await?;

				(raw.trace_id, raw.items, None)
			},
			SearchSessionizePath::Planned => {
				let raw = self.search_raw_planned(raw_req).await?;

				(raw.trace_id, raw.items, Some(raw.query_plan))
			},
		};
		let now = OffsetDateTime::now_utc();
		let expires_at = now + Duration::hours(SESSION_SLIDING_TTL_HOURS);
		let search_session_id = Uuid::new_v4();
		let note_ids: Vec<Uuid> = raw_items.iter().map(|item| item.note_id).collect();
		let structured_by_note =
			crate::structured_fields::fetch_structured_fields(&self.db.pool, &note_ids).await?;
		let mut items = Vec::with_capacity(raw_items.len());

		for (idx, item) in raw_items.iter().enumerate() {
			let summary = structured_by_note
				.get(&item.note_id)
				.and_then(|value| value.summary.clone())
				.unwrap_or_else(|| {
					build_summary(&item.snippet, self.cfg.memory.max_note_chars as usize)
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

		store_search_session(
			&self.db.pool,
			NewSearchSession {
				search_session_id,
				trace_id,
				tenant_id: &req.tenant_id,
				project_id: &req.project_id,
				agent_id: &req.agent_id,
				read_profile: &req.read_profile,
				query: &req.query,
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
			},
			query_plan,
		})
	}

	pub async fn search_session_get(
		&self,
		req: SearchSessionGetRequest,
	) -> Result<SearchIndexResponse> {
		let tenant_id = req.tenant_id.trim();
		let project_id = req.project_id.trim();
		let agent_id = req.agent_id.trim();

		if tenant_id.is_empty() || project_id.is_empty() || agent_id.is_empty() {
			return Err(Error::InvalidRequest {
				message: "tenant_id, project_id, and agent_id are required.".to_string(),
			});
		}

		let now = OffsetDateTime::now_utc();
		let session = load_search_session(&self.db.pool, req.search_session_id, now).await?;

		validate_search_session_access(&session, tenant_id, project_id, agent_id)?;

		let touch = req.touch.unwrap_or(true);
		let expires_at = if touch {
			touch_search_session(&self.db.pool, &session, now).await?
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

		Ok(SearchIndexResponse {
			trace_id: session.trace_id,
			search_session_id: session.search_session_id,
			expires_at,
			items,
		})
	}

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
		let session = load_search_session(&self.db.pool, req.search_session_id, now).await?;

		validate_search_session_access(&session, tenant_id, project_id, agent_id)?;

		let expires_at = touch_search_session(&self.db.pool, &session, now).await?;
		let payload_level = req.payload_level;
		let group_by = req.group_by.unwrap_or_else(|| {
			if payload_level == PayloadLevel::L0 { "none".to_string() } else { "day".to_string() }
		});

		match group_by.as_str() {
			"day" => build_timeline_by_day(session.search_session_id, expires_at, &session.items),
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
		let session = load_search_session(&self.db.pool, req.search_session_id, now).await?;

		validate_search_session_access(&session, tenant_id, project_id, agent_id)?;

		let expires_at = touch_search_session(&self.db.pool, &session, now).await?;
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
			.bind(access::ORG_PROJECT_ID)
			.fetch_all(&self.db.pool)
			.await?;

			for note in rows {
				notes_by_id.insert(note.note_id, note);
			}
		}

		let structured_by_note = crate::structured_fields::fetch_structured_fields(
			&self.db.pool,
			requested_in_session.as_slice(),
		)
		.await?;
		let allowed_scopes = resolve_read_scopes(&self.cfg, &session.read_profile)?;
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
		};
		let (results, hits) = build_search_details_results(req.note_ids, details_args);

		if !hits.is_empty() {
			let mut tx = self.db.pool.begin().await?;

			record_detail_hits(&mut *tx, &session.query, &hits, now).await?;

			tx.commit().await?;
		}

		Ok(SearchDetailsResponse {
			search_session_id: session.search_session_id,
			expires_at,
			results,
		})
	}
}

struct SearchDetailsBuildArgs<'a> {
	session_items_by_note_id: &'a HashMap<Uuid, SearchSessionItemRecord>,
	notes_by_id: &'a HashMap<Uuid, MemoryNote>,
	structured_by_note: &'a HashMap<Uuid, StructuredFields>,
	session: &'a SearchSession,
	shared_grants: &'a HashSet<SharedSpaceGrantKey>,
	allowed_scopes: &'a [String],
	now: OffsetDateTime,
	record_hits_enabled: bool,
}

fn build_search_details_results(
	requested_note_ids: Vec<Uuid>,
	args: SearchDetailsBuildArgs<'_>,
) -> (Vec<SearchDetailsResult>, Vec<HitItem>) {
	let mut results = Vec::with_capacity(requested_note_ids.len());
	let mut hits = Vec::new();
	let mut hit_seen = HashSet::new();

	for note_id in requested_note_ids {
		let Some(session_item) = args.session_items_by_note_id.get(&note_id) else {
			results.push(SearchDetailsResult {
				note_id,
				note: None,
				error: Some(SearchDetailsError {
					code: "NOT_IN_SESSION".to_string(),
					message: "Requested note_id is not present in the search session.".to_string(),
				}),
			});

			continue;
		};
		let Some(note) = args.notes_by_id.get(&note_id) else {
			results.push(SearchDetailsResult {
				note_id,
				note: None,
				error: Some(SearchDetailsError {
					code: "NOTE_NOT_FOUND".to_string(),
					message: "Note not found.".to_string(),
				}),
			});

			continue;
		};
		let error = validate_note_access(
			note,
			args.session,
			args.allowed_scopes,
			args.shared_grants,
			args.now,
		);

		if let Some(error) = error {
			results.push(SearchDetailsResult { note_id, note: None, error: Some(error) });

			continue;
		}

		let note_response = NoteFetchResponse {
			note_id: note.note_id,
			tenant_id: note.tenant_id.clone(),
			project_id: note.project_id.clone(),
			agent_id: note.agent_id.clone(),
			scope: note.scope.clone(),
			r#type: note.r#type.clone(),
			key: note.key.clone(),
			text: note.text.clone(),
			importance: note.importance,
			confidence: note.confidence,
			status: note.status.clone(),
			updated_at: note.updated_at,
			expires_at: note.expires_at,
			source_ref: note.source_ref.clone(),
			structured: args.structured_by_note.get(&note.note_id).cloned(),
		};

		results.push(SearchDetailsResult { note_id, note: Some(note_response), error: None });

		if args.record_hits_enabled && hit_seen.insert(note_id) {
			hits.push(HitItem {
				note_id,
				chunk_id: session_item.chunk_id,
				rank: session_item.rank,
				final_score: session_item.final_score,
			});
		}
	}

	(results, hits)
}

fn build_timeline_by_day(
	search_session_id: Uuid,
	expires_at: OffsetDateTime,
	items: &[SearchSessionItemRecord],
) -> Result<SearchTimelineResponse> {
	let mut grouped: BTreeMap<String, Vec<SearchIndexItem>> = BTreeMap::new();

	for item in items {
		let date = item.updated_at.date().to_string();

		grouped.entry(date).or_default().push(item.to_index_item());
	}

	let mut groups = Vec::with_capacity(grouped.len());

	for (date, mut items) in grouped.into_iter().rev() {
		items.sort_by(|a, b| {
			b.updated_at.cmp(&a.updated_at).then_with(|| {
				b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal)
			})
		});
		groups.push(SearchTimelineGroup { date, items });
	}

	Ok(SearchTimelineResponse { search_session_id, expires_at, groups })
}

fn build_summary(raw: &str, max_chars: usize) -> String {
	let normalized = normalize_whitespace(raw);

	truncate_chars(&normalized, max_chars)
}

fn normalize_whitespace(raw: &str) -> String {
	let mut out = String::with_capacity(raw.len());
	let mut prev_space = false;

	for ch in raw.chars() {
		if ch.is_whitespace() {
			if !prev_space {
				out.push(' ');

				prev_space = true;
			}

			continue;
		}

		out.push(ch);

		prev_space = false;
	}

	out.trim().to_string()
}

fn truncate_chars(raw: &str, max_chars: usize) -> String {
	if raw.chars().count() <= max_chars {
		return raw.to_string();
	}

	let mut out = String::with_capacity(max_chars + 3);

	for (idx, ch) in raw.chars().enumerate() {
		if idx >= max_chars {
			break;
		}

		out.push(ch);
	}

	out.push_str("...");

	out
}

fn resolve_read_scopes(cfg: &Config, profile: &str) -> Result<Vec<String>> {
	match profile {
		"private_only" => Ok(cfg.scopes.read_profiles.private_only.clone()),
		"private_plus_project" => Ok(cfg.scopes.read_profiles.private_plus_project.clone()),
		"all_scopes" => Ok(cfg.scopes.read_profiles.all_scopes.clone()),
		_ => Err(Error::InvalidRequest { message: "Unknown read_profile.".to_string() }),
	}
}

fn validate_search_session_access(
	session: &SearchSession,
	tenant_id: &str,
	project_id: &str,
	agent_id: &str,
) -> Result<()> {
	if session.tenant_id != tenant_id
		|| session.project_id != project_id
		|| session.agent_id != agent_id
	{
		return Err(Error::InvalidRequest { message: "Unknown search_session_id.".to_string() });
	}

	Ok(())
}

fn validate_note_access(
	note: &MemoryNote,
	session: &SearchSession,
	allowed_scopes: &[String],
	shared_grants: &HashSet<SharedSpaceGrantKey>,
	now: OffsetDateTime,
) -> Option<SearchDetailsError> {
	if note.status != "active" {
		return Some(SearchDetailsError {
			code: "NOTE_INACTIVE".to_string(),
			message: "Note is not active.".to_string(),
		});
	}
	if note.expires_at.map(|ts| ts <= now).unwrap_or(false) {
		return Some(SearchDetailsError {
			code: "NOTE_EXPIRED".to_string(),
			message: "Note is expired.".to_string(),
		});
	}
	if !allowed_scopes.iter().any(|scope| scope == &note.scope) {
		return Some(SearchDetailsError {
			code: "SCOPE_DENIED".to_string(),
			message: "Note scope is not allowed for this read_profile.".to_string(),
		});
	}
	if !access::note_read_allowed(
		note,
		session.agent_id.as_str(),
		allowed_scopes,
		shared_grants,
		now,
	) {
		return Some(SearchDetailsError {
			code: "SCOPE_DENIED".to_string(),
			message: "Note scope is not allowed for this read_profile.".to_string(),
		});
	}

	None
}

fn hash_query(query: &str) -> String {
	let mut hasher = DefaultHasher::new();

	Hash::hash(query, &mut hasher);

	format!("{:x}", hasher.finish())
}

async fn store_search_session<'e, E>(executor: E, session: NewSearchSession<'_>) -> Result<()>
where
	E: PgExecutor<'e>,
{
	let items_json = serde_json::to_value(session.items).map_err(|err| Error::Storage {
		message: format!("Failed to encode search session items: {err}"),
	})?;

	sqlx::query(
		"\
INSERT INTO search_sessions (
	search_session_id,
	trace_id,
	tenant_id,
	project_id,
	agent_id,
	read_profile,
	query,
	items,
	created_at,
	expires_at
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
	)
	.bind(session.search_session_id)
	.bind(session.trace_id)
	.bind(session.tenant_id.trim())
	.bind(session.project_id.trim())
	.bind(session.agent_id.trim())
	.bind(session.read_profile)
	.bind(session.query)
	.bind(items_json)
	.bind(session.created_at)
	.bind(session.expires_at)
	.execute(executor)
	.await?;

	Ok(())
}

async fn load_search_session<'e, E>(
	executor: E,
	search_session_id: Uuid,
	now: OffsetDateTime,
) -> Result<SearchSession>
where
	E: PgExecutor<'e>,
{
	let row = sqlx::query_as::<_, SearchSessionRow>(
		"\
SELECT
	search_session_id,
	trace_id,
	tenant_id,
	project_id,
	agent_id,
	read_profile,
	query,
	items,
	created_at,
	expires_at
FROM search_sessions
WHERE search_session_id = $1",
	)
	.bind(search_session_id)
	.fetch_optional(executor)
	.await?;
	let Some(row) = row else {
		return Err(Error::InvalidRequest { message: "Unknown search_session_id.".to_string() });
	};
	let expires_at: OffsetDateTime = row.expires_at;

	if expires_at <= now {
		return Err(Error::InvalidRequest { message: "Search session expired.".to_string() });
	}

	let items: Vec<SearchSessionItemRecord> = serde_json::from_value(row.items).map_err(|err| {
		Error::Storage { message: format!("Failed to decode search session items: {err}") }
	})?;

	Ok(SearchSession {
		search_session_id: row.search_session_id,
		trace_id: row.trace_id,
		tenant_id: row.tenant_id,
		project_id: row.project_id,
		agent_id: row.agent_id,
		read_profile: row.read_profile,
		query: row.query,
		items,
		created_at: row.created_at,
		expires_at,
	})
}

async fn touch_search_session<'e, E>(
	executor: E,
	session: &SearchSession,
	now: OffsetDateTime,
) -> Result<OffsetDateTime>
where
	E: PgExecutor<'e>,
{
	let absolute_expires_at = session.created_at + Duration::hours(SESSION_ABSOLUTE_TTL_HOURS);
	let sliding_expires_at = now + Duration::hours(SESSION_SLIDING_TTL_HOURS);
	let touched = if sliding_expires_at < absolute_expires_at {
		sliding_expires_at
	} else {
		absolute_expires_at
	};

	if touched <= session.expires_at {
		return Ok(session.expires_at);
	}

	sqlx::query(
		"UPDATE search_sessions SET expires_at = $1 WHERE search_session_id = $2 AND expires_at < $1",
	)
	.bind(touched)
	.bind(session.search_session_id)
	.execute(executor)
	.await?;

	Ok(touched)
}

async fn record_detail_hits<'e, E>(
	executor: E,
	query: &str,
	items: &[HitItem],
	now: OffsetDateTime,
) -> Result<()>
where
	E: PgExecutor<'e>,
{
	if cjk::contains_cjk(query) {
		return Err(Error::NonEnglishInput { field: "$.query".to_string() });
	}

	let query_hash = hash_query(query);
	let mut hit_ids = Vec::with_capacity(items.len());
	let mut note_ids = Vec::with_capacity(items.len());
	let mut chunk_ids = Vec::with_capacity(items.len());
	let mut ranks = Vec::with_capacity(items.len());
	let mut final_scores = Vec::with_capacity(items.len());

	for item in items {
		let rank = i32::try_from(item.rank).map_err(|_| Error::InvalidRequest {
			message: "Search session rank is out of range.".to_string(),
		})?;

		hit_ids.push(Uuid::new_v4());
		note_ids.push(item.note_id);
		chunk_ids.push(item.chunk_id);
		ranks.push(rank);
		final_scores.push(item.final_score);
	}

	sqlx::query(
		"\
WITH hits AS (
	SELECT *
	FROM unnest(
	$1::uuid[],
	$2::uuid[],
	$3::uuid[],
	$4::int4[],
	$5::real[]
) AS t(hit_id, note_id, chunk_id, rank, final_score)
),
updated AS (
UPDATE memory_notes
SET
	hit_count = hit_count + 1,
	last_hit_at = $6
WHERE note_id = ANY($2)
)
INSERT INTO memory_hits (
	hit_id,
	note_id,
	chunk_id,
	query_hash,
	rank,
	final_score,
	ts
)
SELECT
	hit_id,
	note_id,
	chunk_id,
	$7,
	rank,
	final_score,
	$6
FROM hits",
	)
	.bind(&hit_ids)
	.bind(&note_ids)
	.bind(&chunk_ids)
	.bind(&ranks)
	.bind(&final_scores)
	.bind(now)
	.bind(query_hash.as_str())
	.execute(executor)
	.await?;

	Ok(())
}
