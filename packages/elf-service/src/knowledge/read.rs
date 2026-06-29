use crate::knowledge::{
	ElfService, Error, KnowledgePage, KnowledgePageGetRequest, KnowledgePageKind,
	KnowledgePageLintFindingResponse, KnowledgePageResponse, KnowledgePageSearchRequest,
	KnowledgePageSearchResponse, KnowledgePageSourceRefResponse, KnowledgePageSummary,
	KnowledgePagesListRequest, KnowledgePagesListResponse, Result, access, english_gate, knowledge,
	search,
};

impl ElfService {
	/// Gets one derived knowledge page with sections, source refs, and lint findings.
	pub async fn knowledge_page_get(
		&self,
		req: KnowledgePageGetRequest,
	) -> Result<KnowledgePageResponse> {
		let page = knowledge::get_knowledge_page(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.page_id,
		)
		.await?
		.ok_or_else(|| Error::NotFound { message: "knowledge page not found".to_string() })?;

		self.knowledge_page_response(page).await
	}

	/// Lists derived knowledge pages.
	pub async fn knowledge_pages_list(
		&self,
		req: KnowledgePagesListRequest,
	) -> Result<KnowledgePagesListResponse> {
		let page_kind = req.page_kind.map(KnowledgePageKind::as_str);
		let pages = knowledge::list_knowledge_pages(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			page_kind,
			crate::knowledge::bounded_limit(req.limit),
		)
		.await?
		.into_iter()
		.map(KnowledgePageSummary::from)
		.collect();

		Ok(KnowledgePagesListResponse { pages })
	}

	/// Searches derived knowledge page sections and returns provenance-rich snippets.
	pub async fn knowledge_pages_search(
		&self,
		req: KnowledgePageSearchRequest,
	) -> Result<KnowledgePageSearchResponse> {
		crate::knowledge::validate_non_empty("tenant_id", req.tenant_id.as_str())?;
		crate::knowledge::validate_non_empty("project_id", req.project_id.as_str())?;
		crate::knowledge::validate_non_empty("agent_id", req.agent_id.as_str())?;
		crate::knowledge::validate_non_empty("read_profile", req.read_profile.as_str())?;
		crate::knowledge::validate_non_empty("query", req.query.as_str())?;

		if !english_gate::is_english_natural_language(req.query.as_str()) {
			return Err(Error::NonEnglishInput { field: "$.query".to_string() });
		}

		let allowed_scopes =
			search::resolve_read_profile_scopes(&self.cfg, req.read_profile.as_str())?;
		let org_shared_allowed = allowed_scopes.iter().any(|scope| scope == "org_shared");
		let shared_grants = access::load_shared_read_grants_with_org_shared(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.agent_id.as_str(),
			org_shared_allowed,
		)
		.await?;
		let query = req.query.trim().to_ascii_lowercase();
		let query_pattern = format!("%{query}%");
		let page_kind = req.page_kind.map(KnowledgePageKind::as_str);
		let rows = knowledge::search_knowledge_page_sections(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			page_kind,
			query_pattern.as_str(),
			crate::knowledge::bounded_limit(req.limit),
		)
		.await?;
		let page_ids = crate::knowledge::sorted_unique(
			&rows.iter().map(|row| row.page_id).collect::<Vec<_>>(),
		);
		let source_refs =
			knowledge::list_knowledge_page_source_refs_for_pages(&self.db.pool, &page_ids).await?;
		let current_source_keys = self
			.resolve_current_recallable_source_keys(
				req.tenant_id.as_str(),
				req.project_id.as_str(),
				req.agent_id.as_str(),
				&allowed_scopes,
				&shared_grants,
				&source_refs,
			)
			.await?;
		let source_refs_by_section = crate::knowledge::source_refs_by_section(&source_refs);
		let items = rows
			.into_iter()
			.filter_map(|row| {
				let refs = crate::knowledge::cloned_source_refs(
					source_refs_by_section.get(&row.section_id),
				);

				crate::knowledge::recallable_source_refs(refs.as_slice(), &current_source_keys)
					.then(|| {
						crate::knowledge::knowledge_page_search_item(row, refs, req.query.as_str())
					})
			})
			.collect();

		Ok(KnowledgePageSearchResponse { items })
	}

	pub(in crate::knowledge) async fn knowledge_page_response(
		&self,
		page: KnowledgePage,
	) -> Result<KnowledgePageResponse> {
		let page_id = page.page_id;
		let section_rows = knowledge::list_knowledge_page_sections(&self.db.pool, page_id).await?;
		let source_ref_rows =
			knowledge::list_knowledge_page_source_refs(&self.db.pool, page_id).await?;
		let source_refs_by_section = crate::knowledge::source_refs_by_section(&source_ref_rows);
		let sections = section_rows
			.into_iter()
			.map(|section| {
				let refs = crate::knowledge::cloned_source_refs(
					source_refs_by_section.get(&section.section_id),
				);

				crate::knowledge::section_response(section, refs)
			})
			.collect();
		let source_refs =
			source_ref_rows.into_iter().map(KnowledgePageSourceRefResponse::from).collect();
		let lint_findings = knowledge::list_knowledge_page_lint_findings(&self.db.pool, page_id)
			.await?
			.into_iter()
			.map(KnowledgePageLintFindingResponse::from)
			.collect();

		Ok(KnowledgePageResponse {
			page: KnowledgePageSummary::from(page),
			sections,
			source_refs,
			lint_findings,
		})
	}
}
