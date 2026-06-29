use super::*;

impl ElfService {
	/// Lints a derived knowledge page against current source snapshots.
	pub async fn knowledge_page_lint(
		&self,
		req: KnowledgePageLintRequest,
	) -> Result<KnowledgePageLintResponse> {
		let page = knowledge::get_knowledge_page(
			&self.db.pool,
			req.tenant_id.as_str(),
			req.project_id.as_str(),
			req.page_id,
		)
		.await?
		.ok_or_else(|| Error::NotFound { message: "knowledge page not found".to_string() })?;
		let source_refs =
			knowledge::list_knowledge_page_source_refs(&self.db.pool, page.page_id).await?;
		let sections = knowledge::list_knowledge_page_sections(&self.db.pool, page.page_id).await?;
		let mut findings = self.lint_source_refs(&page, &source_refs).await?;

		findings.extend(lint_page_sections(&page, &sections, &source_refs));

		let now = OffsetDateTime::now_utc();
		let mut tx = self.db.pool.begin().await?;

		knowledge::delete_knowledge_page_lint_findings(&mut *tx, page.page_id).await?;

		for finding in &findings {
			insert_lint_finding(&mut tx, page.page_id, finding, now).await?;
		}

		tx.commit().await?;

		let persisted = knowledge::list_knowledge_page_lint_findings(&self.db.pool, page.page_id)
			.await?
			.into_iter()
			.map(KnowledgePageLintFindingResponse::from)
			.collect();

		Ok(KnowledgePageLintResponse { page_id: page.page_id, findings: persisted })
	}

	pub(in crate::knowledge) async fn lint_source_refs(
		&self,
		page: &KnowledgePage,
		source_refs: &[KnowledgePageSourceRef],
	) -> Result<Vec<LintDraft>> {
		let ids = SourceIds::from_source_refs(source_refs)?;
		let current = self.resolve_current_source_map(page, &ids).await?;
		let mut findings = Vec::new();

		for source_ref in source_refs {
			let key = current_key(source_ref.source_kind.as_str(), source_ref.source_id);
			let Some(snapshot) = current.get(&key) else {
				findings.push(missing_source_finding(source_ref));

				continue;
			};

			if source_changed(source_ref, snapshot) {
				findings.push(stale_source_finding(source_ref, snapshot));
			}
		}

		Ok(findings)
	}
}
