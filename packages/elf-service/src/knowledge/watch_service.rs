use super::*;

impl ElfService {
	/// Rebuilds pages affected by changed source refs and queues reviewable candidates.
	pub async fn knowledge_pages_watch_rebuild(
		&self,
		req: KnowledgePageWatchRebuildRequest,
	) -> Result<KnowledgePageWatchRebuildResponse> {
		validate_context(req.tenant_id.as_str(), req.project_id.as_str(), req.agent_id.as_str())?;

		let changed_sources = normalized_changed_sources(&req.changed_sources)?;
		let (source_kinds, source_ids) = changed_source_arrays(&changed_sources);
		let page_kind = req.page_kind.map(KnowledgePageKind::as_str);
		let pages = knowledge::list_knowledge_pages_for_sources(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			page_kind,
			&source_kinds,
			&source_ids,
			bounded_limit(req.limit),
		)
		.await?;
		let mut items = Vec::new();
		let mut candidates = Vec::new();

		for page in pages {
			let outcome =
				self.watch_rebuild_page(req.agent_id.as_str(), page, &changed_sources).await?;

			candidates.extend(outcome.candidates);
			items.push(outcome.item);
		}

		let proposal_run = if req.generate_memory_candidates && !candidates.is_empty() {
			Some(self.queue_knowledge_delta_candidates(&req, &changed_sources, &candidates).await?)
		} else {
			None
		};
		let summary = watch_rebuild_summary(changed_sources.len(), &items, candidates.len());
		let operator_summary = watch_operator_summary(&summary, proposal_run.as_ref());

		Ok(KnowledgePageWatchRebuildResponse {
			schema: KNOWLEDGE_PAGE_WATCH_REBUILD_SCHEMA_V1.to_string(),
			summary,
			pages: items,
			memory_candidates: candidates,
			proposal_run,
			operator_summary,
		})
	}

	async fn watch_rebuild_page(
		&self,
		agent_id: &str,
		page: KnowledgePage,
		changed_sources: &[KnowledgePageChangedSource],
	) -> Result<WatchRebuildOutcome> {
		let source_refs =
			knowledge::list_knowledge_page_source_refs(&self.db.pool, page.page_id).await?;
		let sections = knowledge::list_knowledge_page_sections(&self.db.pool, page.page_id).await?;
		let before_lint = self.watch_rebuild_lint(&page, &sections, &source_refs).await?;
		let request = rebuild_request_from_page(agent_id, &page, &source_refs);
		let rebuild = match request {
			Ok(request) => self.knowledge_page_rebuild(request).await,
			Err(err) => Err(err),
		};

		match rebuild {
			Ok(response) => Ok(successful_watch_rebuild(
				sections,
				source_refs,
				before_lint,
				response.page,
				changed_sources,
			)),
			Err(err) => Ok(blocked_watch_rebuild(page, sections, before_lint, err)),
		}
	}

	async fn watch_rebuild_lint(
		&self,
		page: &KnowledgePage,
		sections: &[KnowledgePageSection],
		source_refs: &[KnowledgePageSourceRef],
	) -> Result<Vec<LintDraft>> {
		let mut lint = self.lint_source_refs(page, source_refs).await?;

		lint.extend(lint_page_sections(page, sections, source_refs));

		Ok(lint)
	}

	async fn queue_knowledge_delta_candidates(
		&self,
		req: &KnowledgePageWatchRebuildRequest,
		changed_sources: &[KnowledgePageChangedSource],
		candidates: &[KnowledgeDeltaMemoryCandidate],
	) -> Result<KnowledgePageProposalRunSummary> {
		let source_refs = candidate_run_input_refs(candidates);
		let source_snapshot = knowledge_delta_source_snapshot(changed_sources, candidates);
		let lineage = ConsolidationLineage {
			source_refs: source_refs.clone(),
			parent_run_id: None,
			parent_proposal_ids: Vec::new(),
		};
		let proposals = candidates.iter().map(candidate_proposal_input).collect::<Vec<_>>();
		let created = self
			.consolidation_run_create(ConsolidationRunCreateRequest {
				tenant_id: req.tenant_id.clone(),
				project_id: req.project_id.clone(),
				agent_id: req.agent_id.clone(),
				job_kind: "manual".to_string(),
				input_refs: source_refs,
				source_snapshot,
				lineage,
				proposals,
			})
			.await?;

		Ok(proposal_run_summary(created, candidates.len()))
	}
}
