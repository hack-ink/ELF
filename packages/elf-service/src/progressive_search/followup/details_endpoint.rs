use std::collections::{HashMap, hash_set::HashSet};

use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
	ElfService, Error, PayloadLevel, Result,
	access::{self, ORG_PROJECT_ID},
	progressive_search::{
		details::{self, SearchDetailsBuildArgs},
		storage,
		types::{SearchDetailsRequest, SearchDetailsResponse, SearchSessionItemRecord},
	},
	structured_fields,
};
use elf_storage::models::MemoryNote;

impl ElfService {
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
