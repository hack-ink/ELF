use super::*;

impl ElfService {
	/// Rebuilds and persists one derived knowledge page from explicit source ids.
	pub async fn knowledge_page_rebuild(
		&self,
		req: KnowledgePageRebuildRequest,
	) -> Result<KnowledgePageRebuildResponse> {
		validate_context(req.tenant_id.as_str(), req.project_id.as_str(), req.agent_id.as_str())?;
		validate_non_empty("page_key", req.page_key.as_str())?;
		validate_object("provider_metadata", &req.provider_metadata)?;

		let ids = SourceIds::from_request(&req)?;
		let title =
			req.title.clone().unwrap_or_else(|| generated_title(req.page_kind, &req.page_key));
		let previous_page = knowledge::get_knowledge_page_by_key(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.page_kind.as_str(),
			req.page_key.as_str(),
		)
		.await?;
		let previous_sections = match &previous_page {
			Some(page) =>
				knowledge::list_knowledge_page_sections(&self.db.pool, page.page_id).await?,
			None => Vec::new(),
		};
		let sources = self.resolve_sources(&req, &ids).await?;
		let now = OffsetDateTime::now_utc();
		let source_snapshot = source_snapshot_value(&sources);
		let source_hash = hash_json(&source_snapshot)?;
		let mut sections = build_sections(&sources)?;
		let lint = lint_unsupported_sections(&sections);

		for section in &mut sections {
			section.citations = citations_value(section, &sources);
			section.content_hash = hash_json(&section_hash_payload(section))?;
		}

		let source_coverage =
			source_coverage_value(req.page_kind, &req.page_key, &sections, &sources);
		let base_rebuild_metadata = rebuild_metadata(&source_hash, &req.provider_metadata, &req);
		let content_hash =
			page_content_hash(&title, &sections, &source_coverage, &base_rebuild_metadata)?;
		let previous_version_diff = previous_version_diff_value(
			previous_page.as_ref(),
			&previous_sections,
			title.as_str(),
			source_hash.as_str(),
			content_hash.as_str(),
			&sections,
		);
		let version_identity = version_identity_value(
			req.page_kind,
			req.page_key.as_str(),
			source_hash.as_str(),
			content_hash.as_str(),
			&sections,
		);
		let rebuild_metadata = rebuild_metadata_with_previous_version_diff(
			base_rebuild_metadata,
			previous_version_diff,
			version_identity,
		);
		let page_id = Uuid::new_v4();
		let mut tx = self.db.pool.begin().await?;
		let page = knowledge::upsert_knowledge_page(
			&mut *tx,
			KnowledgePageUpsert {
				page_id,
				tenant_id: req.tenant_id.as_str(),
				project_id: req.project_id.as_str(),
				page_kind: req.page_kind.as_str(),
				page_key: req.page_key.as_str(),
				title: title.as_str(),
				contract_schema: KNOWLEDGE_PAGE_CONTRACT_SCHEMA_V1,
				status: "active",
				rebuild_source_hash: source_hash.as_str(),
				content_hash: content_hash.as_str(),
				source_coverage: &source_coverage,
				source_snapshot: &source_snapshot,
				rebuild_metadata: &rebuild_metadata,
				now,
			},
		)
		.await?;

		replace_page_children(&mut tx, page.page_id, &sections, &sources, &lint, now).await?;

		tx.commit().await?;

		Ok(KnowledgePageRebuildResponse { page: self.knowledge_page_response(page).await? })
	}
}
