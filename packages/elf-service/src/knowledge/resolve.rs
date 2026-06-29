use super::*;

impl ElfService {
	pub(in crate::knowledge) async fn resolve_sources(
		&self,
		req: &KnowledgePageRebuildRequest,
		ids: &SourceIds,
	) -> Result<Vec<SourceSnapshot>> {
		let allowed_scopes = self.cfg.scopes.allowed.as_slice();
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.agent_id.as_str(),
			org_shared_allowed,
		)
		.await?;
		let (docs, doc_chunks, notes, events, relations, proposals) = self
			.resolve_existing_source_rows(
				req.tenant_id.as_str(),
				req.project_id.as_str(),
				Some(req.agent_id.as_str()),
				allowed_scopes,
				&shared_grants,
				ids,
			)
			.await?;

		ids.require_counts(
			docs.len(),
			doc_chunks.len(),
			notes.len(),
			events.len(),
			relations.len(),
			proposals.len(),
		)?;

		Ok(source_snapshots(docs, doc_chunks, notes, events, relations, proposals))
	}

	#[allow(clippy::type_complexity)]
	pub(in crate::knowledge) async fn resolve_existing_source_rows(
		&self,
		tenant_id: &str,
		project_id: &str,
		agent_id: Option<&str>,
		allowed_scopes: &[String],
		shared_grants: &HashSet<access::SharedSpaceGrantKey>,
		ids: &SourceIds,
	) -> Result<(
		Vec<KnowledgeDocSource>,
		Vec<KnowledgeDocChunkSource>,
		Vec<KnowledgeNoteSource>,
		Vec<KnowledgeEventSource>,
		Vec<KnowledgeRelationSource>,
		Vec<KnowledgeProposalSource>,
	)> {
		let docs = knowledge::fetch_knowledge_doc_sources(
			&self.db.pool,
			tenant_id,
			project_id,
			agent_id,
			allowed_scopes,
			&ids.doc_ids,
		)
		.await?;
		let docs = docs
			.into_iter()
			.filter(|source| {
				source_row_read_allowed(
					source.agent_id.as_str(),
					source.scope.as_str(),
					agent_id,
					allowed_scopes,
					shared_grants,
				)
			})
			.collect();
		let doc_chunks = knowledge::fetch_knowledge_doc_chunk_sources(
			&self.db.pool,
			tenant_id,
			project_id,
			agent_id,
			allowed_scopes,
			&ids.doc_chunk_ids,
		)
		.await?;
		let doc_chunks = doc_chunks
			.into_iter()
			.filter(|source| {
				source_row_read_allowed(
					source.agent_id.as_str(),
					source.scope.as_str(),
					agent_id,
					allowed_scopes,
					shared_grants,
				)
			})
			.collect();
		let notes = knowledge::fetch_knowledge_note_sources(
			&self.db.pool,
			tenant_id,
			project_id,
			agent_id,
			allowed_scopes,
			&ids.note_ids,
		)
		.await?;
		let notes = notes
			.into_iter()
			.filter(|source| {
				source_row_read_allowed(
					source.agent_id.as_str(),
					source.scope.as_str(),
					agent_id,
					allowed_scopes,
					shared_grants,
				)
			})
			.collect();
		let events = knowledge::fetch_knowledge_event_sources(
			&self.db.pool,
			tenant_id,
			project_id,
			agent_id,
			allowed_scopes,
			&ids.event_ids,
		)
		.await?;
		let events = events
			.into_iter()
			.filter(|source| {
				source_row_read_allowed(
					source.agent_id.as_str(),
					source.scope.as_str(),
					agent_id,
					allowed_scopes,
					shared_grants,
				)
			})
			.collect();
		let shared_scope_keys = access::shared_scope_key_strings(shared_grants);
		let private_allowed = allowed_scopes.iter().any(|scope| scope == "agent_private");
		let relations = knowledge::fetch_knowledge_relation_sources(
			&self.db.pool,
			KnowledgeRelationSourcesFetch {
				tenant_id,
				project_id,
				agent_id,
				allowed_scopes,
				shared_scope_keys: shared_scope_keys.as_slice(),
				private_allowed,
				fact_ids: &ids.relation_ids,
			},
		)
		.await?;
		let proposals = knowledge::fetch_knowledge_proposal_sources(
			&self.db.pool,
			tenant_id,
			project_id,
			&ids.proposal_ids,
		)
		.await?;

		Ok((docs, doc_chunks, notes, events, relations, proposals))
	}

	pub(in crate::knowledge) async fn resolve_current_source_map(
		&self,
		page: &KnowledgePage,
		ids: &SourceIds,
	) -> Result<BTreeMap<String, SourceSnapshot>> {
		let _page_kind = KnowledgePageKind::parse(page.page_kind.as_str()).ok_or_else(|| {
			Error::InvalidRequest { message: "stored knowledge page kind is invalid".to_string() }
		})?;
		let (docs, doc_chunks, notes, events, relations, proposals) = self
			.resolve_existing_source_rows(
				page.tenant_id.as_str(),
				page.project_id.as_str(),
				None,
				self.cfg.scopes.allowed.as_slice(),
				&HashSet::new(),
				ids,
			)
			.await?;
		let mut sources = source_snapshots(docs, doc_chunks, notes, events, relations, proposals);

		Ok(sources.drain(..).map(|source| (source_key(&source), source)).collect())
	}

	pub(in crate::knowledge) async fn resolve_current_recallable_source_keys(
		&self,
		tenant_id: &str,
		project_id: &str,
		agent_id: &str,
		allowed_scopes: &[String],
		shared_grants: &HashSet<access::SharedSpaceGrantKey>,
		source_refs: &[KnowledgePageSourceRef],
	) -> Result<BTreeSet<String>> {
		let ids = SourceIds::from_source_refs(source_refs)?;
		let (docs, doc_chunks, notes, events, relations, proposals) = self
			.resolve_existing_source_rows(
				tenant_id,
				project_id,
				Some(agent_id),
				allowed_scopes,
				shared_grants,
				&ids,
			)
			.await?;

		Ok(source_snapshots(docs, doc_chunks, notes, events, relations, proposals)
			.into_iter()
			.map(|source| source_key(&source))
			.collect())
	}
}
