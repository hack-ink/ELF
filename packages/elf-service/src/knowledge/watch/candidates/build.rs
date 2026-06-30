use crate::knowledge::{
	ConsolidationInputRef, KnowledgeDeltaMemoryCandidate, KnowledgePageRebuildOutput,
	KnowledgePageResponse, KnowledgePageSectionResponse, Value,
	watch::{
		self,
		candidates::{proposal, refs},
	},
};

pub(in crate::knowledge) fn memory_candidates_for_page(
	page: &KnowledgePageResponse,
	outputs: &[KnowledgePageRebuildOutput],
) -> Vec<KnowledgeDeltaMemoryCandidate> {
	let reasons = watch::candidate_reasons_by_section(outputs);

	page.sections
		.iter()
		.filter_map(|section| {
			let reason = reasons.get(section.section_key.as_str())?;

			memory_candidate_for_section(page, section, reason.as_str())
		})
		.collect()
}

fn memory_candidate_for_section(
	page: &KnowledgePageResponse,
	section: &KnowledgePageSectionResponse,
	reason: &str,
) -> Option<KnowledgeDeltaMemoryCandidate> {
	let source_refs = page
		.source_refs
		.iter()
		.filter(|source_ref| source_ref.section_id == Some(section.section_id))
		.filter_map(|source_ref| refs::consolidation_input_ref(source_ref, page, section, reason))
		.collect::<Vec<_>>();

	if source_refs.is_empty() {
		return None;
	}

	let source_snapshot = candidate_source_snapshot(page, section, reason, &source_refs);
	let diff = proposal::candidate_diff(page, section, reason);
	let proposed_payload = proposal::candidate_proposed_payload(page, section, reason);

	Some(KnowledgeDeltaMemoryCandidate {
		reason: reason.to_string(),
		page_id: page.page.page_id,
		section_id: section.section_id,
		section_key: section.section_key.clone(),
		source_refs,
		source_snapshot,
		diff,
		proposed_payload,
	})
}

fn candidate_source_snapshot(
	page: &KnowledgePageResponse,
	section: &KnowledgePageSectionResponse,
	reason: &str,
	source_refs: &[ConsolidationInputRef],
) -> Value {
	serde_json::json!({
		"schema": "elf.knowledge_delta.source_snapshot/v1",
		"reason": reason,
		"page": {
			"page_id": page.page.page_id,
			"page_kind": page.page.page_kind,
			"page_key": page.page.page_key,
			"content_hash": page.page.content_hash,
			"rebuild_source_hash": page.page.rebuild_source_hash,
			"previous_version_diff": page.page.previous_version_diff,
		},
		"section": {
			"section_id": section.section_id,
			"section_key": section.section_key,
			"heading": section.heading,
			"content_hash": section.content_hash,
			"citation_count": section.citation_count,
			"source_ref_count": section.source_ref_count,
		},
		"source_ref_count": source_refs.len(),
		"source_mutation_allowed": false,
	})
}
