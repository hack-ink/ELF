use crate::{
	access,
	search::{
		ElfService, HashMap, OffsetDateTime, RELATION_CONTEXT_SQL, RelationTemporalStatus, Result,
		ScoredChunk, SearchExplainRelationContext, SearchExplainRelationContextObject,
		SearchExplainRelationEntityRef, SearchRelationContextRow, Uuid,
	},
};

impl ElfService {
	pub(in crate::search) async fn build_relation_context_for_selected_results(
		&self,
		selected_results: &[ScoredChunk],
		tenant_id: &str,
		project_id: &str,
		agent_id: &str,
		allowed_scopes: &[String],
		now: OffsetDateTime,
	) -> Result<HashMap<Uuid, Vec<SearchExplainRelationContext>>> {
		if !self.cfg.search.graph_context.enabled {
			return Ok(HashMap::new());
		}

		let selected_note_ids: Vec<Uuid> =
			selected_results.iter().map(|chunk| chunk.item.note.note_id).collect();

		if selected_note_ids.is_empty() {
			return Ok(HashMap::new());
		}

		self.fetch_relation_contexts_for_notes(
			selected_note_ids.as_slice(),
			tenant_id,
			project_id,
			agent_id,
			allowed_scopes,
			now,
		)
		.await
	}

	pub(in crate::search) async fn fetch_relation_contexts_for_notes(
		&self,
		note_ids: &[Uuid],
		tenant_id: &str,
		project_id: &str,
		agent_id: &str,
		allowed_scopes: &[String],
		now: OffsetDateTime,
	) -> Result<HashMap<Uuid, Vec<SearchExplainRelationContext>>> {
		if note_ids.is_empty() {
			return Ok(HashMap::new());
		}

		let private_allowed = allowed_scopes.iter().any(|scope| scope == "agent_private");
		let non_private_scopes: Vec<String> =
			allowed_scopes.iter().filter(|scope| *scope != "agent_private").cloned().collect();
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&self.db.pool,
			tenant_id,
			project_id,
			agent_id,
			org_shared_allowed,
		)
		.await?;
		let shared_scope_keys = access::shared_scope_key_strings(&shared_grants);
		let (max_evidence_notes_per_fact, max_facts_per_item) = self.relation_context_bounds();
		let rows = self
			.fetch_relation_context_rows(
				note_ids,
				tenant_id,
				project_id,
				agent_id,
				&non_private_scopes,
				shared_scope_keys.as_slice(),
				private_allowed,
				now,
				max_evidence_notes_per_fact,
				max_facts_per_item,
			)
			.await?;

		Ok(Self::group_relation_context_rows(rows))
	}

	pub(in crate::search) fn relation_context_bounds(&self) -> (i32, i32) {
		let max_evidence_notes_per_fact =
			i32::try_from(self.cfg.search.graph_context.max_evidence_notes_per_fact)
				.unwrap_or(i32::MAX);
		let max_facts_per_item =
			i32::try_from(self.cfg.search.graph_context.max_facts_per_item).unwrap_or(i32::MAX);

		(max_evidence_notes_per_fact, max_facts_per_item)
	}

	#[allow(clippy::too_many_arguments)]
	pub(in crate::search) async fn fetch_relation_context_rows(
		&self,
		note_ids: &[Uuid],
		tenant_id: &str,
		project_id: &str,
		agent_id: &str,
		non_private_scopes: &[String],
		shared_scope_keys: &[String],
		private_allowed: bool,
		now: OffsetDateTime,
		max_evidence_notes_per_fact: i32,
		max_facts_per_item: i32,
	) -> Result<Vec<SearchRelationContextRow>> {
		Ok(sqlx::query_as::<_, SearchRelationContextRow>(RELATION_CONTEXT_SQL)
			.bind(tenant_id)
			.bind(project_id)
			.bind(agent_id)
			.bind(now)
			.bind(private_allowed)
			.bind(non_private_scopes)
			.bind(note_ids)
			.bind(max_evidence_notes_per_fact)
			.bind(max_facts_per_item)
			.bind(shared_scope_keys)
			.fetch_all(&self.db.pool)
			.await?)
	}

	pub(in crate::search) fn group_relation_context_rows(
		rows: Vec<SearchRelationContextRow>,
	) -> HashMap<Uuid, Vec<SearchExplainRelationContext>> {
		let mut relation_context_by_note: HashMap<Uuid, Vec<SearchExplainRelationContext>> =
			HashMap::new();

		for row in rows {
			if row.evidence_note_ids.is_empty() {
				continue;
			}

			let object = if row.object_entity_id.is_some() {
				SearchExplainRelationContextObject {
					entity: Some(SearchExplainRelationEntityRef {
						canonical: row.object_canonical,
						kind: row.object_kind,
					}),
					value: None,
				}
			} else {
				SearchExplainRelationContextObject { entity: None, value: row.object_value }
			};

			relation_context_by_note.entry(row.note_id).or_default().push(
				SearchExplainRelationContext {
					fact_id: row.fact_id,
					scope: row.scope,
					subject: SearchExplainRelationEntityRef {
						canonical: row.subject_canonical,
						kind: row.subject_kind,
					},
					predicate: row.predicate,
					object,
					valid_from: row.valid_from,
					valid_to: row.valid_to,
					temporal_status: if row.is_current {
						RelationTemporalStatus::Current
					} else {
						RelationTemporalStatus::Historical
					},
					evidence_note_ids: row.evidence_note_ids,
				},
			);
		}

		relation_context_by_note
	}
}
