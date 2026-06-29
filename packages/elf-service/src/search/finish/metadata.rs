use crate::{
	access,
	search::{
		ElfService, HashMap, MemoryNote, NoteMeta, ORG_PROJECT_ID, OffsetDateTime, Result, Uuid,
	},
};

impl ElfService {
	pub(in crate::search) async fn fetch_note_meta_for_candidates(
		&self,
		tenant_id: &str,
		project_id: &str,
		agent_id: &str,
		allowed_scopes: &[String],
		candidate_note_ids: &[Uuid],
		now: OffsetDateTime,
	) -> Result<HashMap<Uuid, NoteMeta>> {
		if candidate_note_ids.is_empty() {
			return Ok(HashMap::new());
		}

		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&self.db.pool,
			tenant_id,
			project_id,
			agent_id,
			org_shared_allowed,
		)
		.await?;
		let notes: Vec<MemoryNote> = sqlx::query_as(
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
		.bind(candidate_note_ids)
		.bind(tenant_id)
		.bind(project_id)
		.bind(ORG_PROJECT_ID)
		.fetch_all(&self.db.pool)
		.await?;
		let mut note_meta = HashMap::new();

		for note in notes {
			if !access::note_read_allowed(&note, agent_id, allowed_scopes, &shared_grants, now) {
				continue;
			}

			note_meta.insert(
				note.note_id,
				NoteMeta {
					note_id: note.note_id,
					note_type: note.r#type,
					key: note.key,
					scope: note.scope,
					agent_id: note.agent_id,
					importance: note.importance,
					confidence: note.confidence,
					updated_at: note.updated_at,
					expires_at: note.expires_at,
					source_ref: note.source_ref,
					embedding_version: note.embedding_version,
					hit_count: note.hit_count,
					last_hit_at: note.last_hit_at,
				},
			);
		}

		Ok(note_meta)
	}
}
