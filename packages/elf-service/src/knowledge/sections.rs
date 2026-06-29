use super::*;

pub(super) fn build_sections(sources: &[SourceSnapshot]) -> Result<Vec<DraftSection>> {
	let doc_indexes = source_indexes(sources, KnowledgeSourceKind::Doc);
	let doc_chunk_indexes = source_indexes(sources, KnowledgeSourceKind::DocChunk);
	let note_indexes = source_indexes(sources, KnowledgeSourceKind::Note);
	let event_indexes = source_indexes(sources, KnowledgeSourceKind::Event);
	let relation_indexes = source_indexes(sources, KnowledgeSourceKind::Relation);
	let proposal_indexes = source_indexes(sources, KnowledgeSourceKind::Proposal);
	let mut sections = Vec::new();

	push_section(
		&mut sections,
		"source-documents",
		"Source Documents",
		"source_documents",
		sources,
		doc_indexes,
	);
	push_section(
		&mut sections,
		"source-spans",
		"Source Spans",
		"source_spans",
		sources,
		doc_chunk_indexes,
	);
	push_section(
		&mut sections,
		"source-notes",
		"Source Notes",
		"current_truth",
		sources,
		note_indexes,
	);
	push_section(&mut sections, "event-audits", "Event Audits", "history", sources, event_indexes);
	push_section(&mut sections, "relations", "Relations", "relations", sources, relation_indexes);
	push_section(
		&mut sections,
		"reviewed-proposals",
		"Reviewed Proposals",
		"proposals",
		sources,
		proposal_indexes,
	);

	if sections.is_empty() {
		return Err(Error::InvalidRequest {
			message: "knowledge page rebuild did not produce any cited sections".to_string(),
		});
	}

	Ok(sections)
}

pub(super) fn push_section(
	sections: &mut Vec<DraftSection>,
	section_key: &str,
	heading: &str,
	role: &str,
	sources: &[SourceSnapshot],
	source_indexes: Vec<usize>,
) {
	if source_indexes.is_empty() {
		return;
	}

	let ordinal = i32::try_from(sections.len()).unwrap_or(i32::MAX);
	let content = source_indexes
		.iter()
		.filter_map(|index| sources.get(*index))
		.map(|source| format!("- {}", source.line))
		.collect::<Vec<_>>()
		.join("\n");

	sections.push(DraftSection {
		section_id: Uuid::new_v4(),
		section_key: section_key.to_string(),
		heading: heading.to_string(),
		role: role.to_string(),
		content,
		ordinal,
		source_indexes,
		unsupported_reason: None,
		content_hash: String::new(),
		citations: Value::Array(Vec::new()),
	});
}

pub(super) fn lint_unsupported_sections(sections: &[DraftSection]) -> Vec<LintDraft> {
	sections
		.iter()
		.filter_map(|section| {
			section.unsupported_reason.as_ref().map(|reason| LintDraft {
				section_id: Some(section.section_id),
				finding_type: "unsupported_claim".to_string(),
				severity: "warning".to_string(),
				source_kind: None,
				source_id: None,
				message: format!("Knowledge page section has unsupported content: {reason}"),
				details: serde_json::json!({
					"section_key": section.section_key,
					"unsupported_reason": reason,
					"repair_guidance": repair_guidance_for_finding_type("unsupported_claim"),
				}),
			})
		})
		.collect()
}

pub(super) fn lint_page_sections(
	page: &KnowledgePage,
	sections: &[KnowledgePageSection],
	source_refs: &[KnowledgePageSourceRef],
) -> Vec<LintDraft> {
	let source_refs_by_section = source_refs_by_section(source_refs);
	let mut findings = Vec::new();

	for section in sections {
		findings.extend(lint_one_section(section, &source_refs_by_section));
	}

	if !coverage_complete(page.source_coverage.as_object()) {
		findings.push(low_source_coverage_finding(page));
	}

	findings
}

pub(super) fn lint_one_section(
	section: &KnowledgePageSection,
	source_refs_by_section: &HashMap<Uuid, Vec<KnowledgePageSourceRef>>,
) -> Vec<LintDraft> {
	let citation_count = citation_count(&section.citations);
	let source_ref_count =
		source_refs_by_section.get(&section.section_id).map(Vec::len).unwrap_or_default();
	let mut findings = Vec::new();

	if let Some(reason) = &section.unsupported_reason {
		findings.push(section_finding(
			section,
			"unsupported_claim",
			"warning",
			"Knowledge page section contains unsupported content.",
			serde_json::json!({
				"unsupported_reason": reason,
				"citation_count": citation_count,
				"source_ref_count": source_ref_count,
			}),
		));
	}

	if citation_count == 0 && section.unsupported_reason.is_none() {
		findings.push(section_finding(
			section,
			"missing_citation",
			"error",
			"Knowledge page section has no citations.",
			serde_json::json!({ "source_ref_count": source_ref_count }),
		));
	}
	if source_ref_count == 0 && section.unsupported_reason.is_none() {
		findings.push(section_finding(
			section,
			"missing_source_ref",
			"error",
			"Knowledge page section has no normalized source backlinks.",
			serde_json::json!({ "citation_count": citation_count }),
		));
	}

	findings
}

pub(super) fn section_finding(
	section: &KnowledgePageSection,
	finding_type: &str,
	severity: &str,
	message: &str,
	details: Value,
) -> LintDraft {
	LintDraft {
		section_id: Some(section.section_id),
		finding_type: finding_type.to_string(),
		severity: severity.to_string(),
		source_kind: None,
		source_id: None,
		message: message.to_string(),
		details: with_repair_guidance(
			details,
			section.section_key.as_str(),
			repair_guidance_for_finding_type(finding_type),
		),
	}
}
