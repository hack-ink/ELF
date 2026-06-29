use super::*;

pub(super) async fn replace_page_children(
	tx: &mut Transaction<'_, Postgres>,
	page_id: Uuid,
	sections: &[DraftSection],
	sources: &[SourceSnapshot],
	lint: &[LintDraft],
	now: OffsetDateTime,
) -> Result<()> {
	knowledge::delete_knowledge_page_children(&mut **tx, page_id).await?;

	for section in sections {
		insert_section(tx, page_id, section, now).await?;

		for source_index in &section.source_indexes {
			let source = sources.get(*source_index).ok_or_else(|| Error::InvalidRequest {
				message: "knowledge page section referenced an unknown source".to_string(),
			})?;

			insert_source_ref(tx, page_id, section.section_id, source, now).await?;
		}
	}
	for finding in lint {
		insert_lint_finding(tx, page_id, finding, now).await?;
	}

	Ok(())
}

pub(super) async fn insert_section(
	tx: &mut Transaction<'_, Postgres>,
	page_id: Uuid,
	section: &DraftSection,
	now: OffsetDateTime,
) -> Result<()> {
	knowledge::insert_knowledge_page_section(
		&mut **tx,
		KnowledgePageSectionInsert {
			section_id: section.section_id,
			page_id,
			section_key: section.section_key.as_str(),
			heading: section.heading.as_str(),
			role: section.role.as_str(),
			content: section.content.as_str(),
			ordinal: section.ordinal,
			citations: &section.citations,
			unsupported_reason: section.unsupported_reason.as_deref(),
			content_hash: section.content_hash.as_str(),
			now,
		},
	)
	.await
	.map_err(Error::from)
}

pub(super) async fn insert_source_ref(
	tx: &mut Transaction<'_, Postgres>,
	page_id: Uuid,
	section_id: Uuid,
	source: &SourceSnapshot,
	now: OffsetDateTime,
) -> Result<()> {
	knowledge::insert_knowledge_page_source_ref(
		&mut **tx,
		KnowledgePageSourceRefInsert {
			ref_id: Uuid::new_v4(),
			page_id,
			section_id: Some(section_id),
			source_kind: source.kind.as_str(),
			source_id: source.id,
			source_status: source.status.as_deref(),
			source_updated_at: source.updated_at,
			source_content_hash: source.content_hash.as_deref(),
			source_snapshot: &source.snapshot,
			citation_metadata: &source.citation_metadata,
			now,
		},
	)
	.await
	.map_err(Error::from)
}

pub(super) async fn insert_lint_finding(
	tx: &mut Transaction<'_, Postgres>,
	page_id: Uuid,
	finding: &LintDraft,
	now: OffsetDateTime,
) -> Result<()> {
	knowledge::insert_knowledge_page_lint_finding(
		&mut **tx,
		KnowledgePageLintFindingInsert {
			finding_id: Uuid::new_v4(),
			page_id,
			section_id: finding.section_id,
			finding_type: finding.finding_type.as_str(),
			severity: finding.severity.as_str(),
			source_kind: finding.source_kind.map(KnowledgeSourceKind::as_str),
			source_id: finding.source_id,
			message: finding.message.as_str(),
			details: &finding.details,
			now,
		},
	)
	.await
	.map_err(Error::from)
}
