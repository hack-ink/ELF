use super::*;

pub(in crate::knowledge) fn memory_candidates_for_page(
	page: &KnowledgePageResponse,
	outputs: &[KnowledgePageRebuildOutput],
) -> Vec<KnowledgeDeltaMemoryCandidate> {
	let reasons = candidate_reasons_by_section(outputs);

	page.sections
		.iter()
		.filter_map(|section| {
			let reason = reasons.get(section.section_key.as_str())?;

			memory_candidate_for_section(page, section, reason.as_str())
		})
		.collect()
}

pub(in crate::knowledge) fn memory_candidate_for_section(
	page: &KnowledgePageResponse,
	section: &KnowledgePageSectionResponse,
	reason: &str,
) -> Option<KnowledgeDeltaMemoryCandidate> {
	let source_refs = page
		.source_refs
		.iter()
		.filter(|source_ref| source_ref.section_id == Some(section.section_id))
		.filter_map(|source_ref| consolidation_input_ref(source_ref, page, section, reason))
		.collect::<Vec<_>>();

	if source_refs.is_empty() {
		return None;
	}

	let source_snapshot = candidate_source_snapshot(page, section, reason, &source_refs);
	let diff = candidate_diff(page, section, reason);
	let proposed_payload = candidate_proposed_payload(page, section, reason);

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

pub(in crate::knowledge) fn consolidation_input_ref(
	source_ref: &KnowledgePageSourceRefResponse,
	page: &KnowledgePageResponse,
	section: &KnowledgePageSectionResponse,
	reason: &str,
) -> Option<ConsolidationInputRef> {
	let kind = consolidation_source_kind(source_ref.source_kind.as_str())?;

	Some(ConsolidationInputRef {
		kind,
		id: source_ref.source_id,
		snapshot: ConsolidationSourceSnapshot {
			status: source_ref.source_status.clone(),
			updated_at: source_ref.source_updated_at,
			content_hash: source_ref.source_content_hash.clone(),
			embedding_version: None,
			trace_version: None,
			source_ref: source_ref.source_snapshot.clone(),
			metadata: serde_json::json!({
				"schema": "elf.knowledge_delta.source_ref/v1",
				"reason": reason,
				"page_id": page.page.page_id,
				"page_kind": page.page.page_kind,
				"page_key": page.page.page_key,
				"section_id": section.section_id,
				"section_key": section.section_key,
			}),
		},
	})
}

pub(in crate::knowledge) fn consolidation_source_kind(
	source_kind: &str,
) -> Option<ConsolidationSourceKind> {
	match KnowledgeSourceKind::parse(source_kind)? {
		KnowledgeSourceKind::Doc => Some(ConsolidationSourceKind::Doc),
		KnowledgeSourceKind::DocChunk => Some(ConsolidationSourceKind::DocChunk),
		KnowledgeSourceKind::Note => Some(ConsolidationSourceKind::Note),
		KnowledgeSourceKind::Event => Some(ConsolidationSourceKind::Event),
		KnowledgeSourceKind::Relation | KnowledgeSourceKind::Proposal => None,
	}
}

pub(in crate::knowledge) fn candidate_source_snapshot(
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

pub(in crate::knowledge) fn candidate_diff(
	page: &KnowledgePageResponse,
	section: &KnowledgePageSectionResponse,
	reason: &str,
) -> ConsolidationProposalDiff {
	ConsolidationProposalDiff {
		summary: format!(
			"Create a reviewable memory candidate for knowledge page '{}' section '{}' because {reason}.",
			page.page.page_key, section.section_key
		),
		before: serde_json::json!({
			"page_id": page.page.page_id,
			"section_id": section.section_id,
			"previous_version_diff": page.page.previous_version_diff,
		}),
		after: serde_json::json!({
			"target": "derived_note",
			"reason": reason,
			"page_id": page.page.page_id,
			"section_id": section.section_id,
			"section_key": section.section_key,
		}),
	}
}

pub(in crate::knowledge) fn candidate_proposed_payload(
	page: &KnowledgePageResponse,
	section: &KnowledgePageSectionResponse,
	reason: &str,
) -> Value {
	let text = truncate_chars(
		format!(
			"Plan: Review knowledge page {} section {} because source changes produced a {reason} delta.",
			page.page.page_key, section.section_key
		)
		.as_str(),
		220,
	);

	serde_json::json!({
		"type": "plan",
		"key": format!(
			"knowledge_delta_{}_{}",
			page.page.page_key.replace('-', "_"),
			section.section_key.replace('-', "_")
		),
		"text": text,
		"scope": "project_shared",
		"importance": 0.65,
		"confidence": 0.72,
		"source_ref": {
			"schema": "elf.knowledge_delta/v1",
			"reason": reason,
			"page_id": page.page.page_id,
			"section_id": section.section_id,
			"page_key": page.page.page_key,
			"section_key": section.section_key,
			"source_mutation_allowed": false,
		}
	})
}

pub(in crate::knowledge) fn candidate_proposal_input(
	candidate: &KnowledgeDeltaMemoryCandidate,
) -> ConsolidationProposalInput {
	ConsolidationProposalInput {
		proposal_kind: "knowledge_delta_memory_candidate".to_string(),
		apply_intent: ConsolidationApplyIntent::CreateDerivedNote,
		source_refs: candidate.source_refs.clone(),
		source_snapshot: candidate.source_snapshot.clone(),
		lineage: ConsolidationLineage {
			source_refs: candidate.source_refs.clone(),
			parent_run_id: None,
			parent_proposal_ids: Vec::new(),
		},
		confidence: 0.72,
		unsupported_claim_flags: Vec::new(),
		markers: candidate_markers(candidate),
		diff: candidate.diff.clone(),
		target_ref: empty_object(),
		proposed_payload: candidate.proposed_payload.clone(),
	}
}

pub(in crate::knowledge) fn candidate_markers(
	candidate: &KnowledgeDeltaMemoryCandidate,
) -> ConsolidationMarkers {
	let marker = ConsolidationMarker {
		severity: ConsolidationMarkerSeverity::Medium,
		message: format!(
			"Knowledge delta '{}' requires reviewer confirmation before memory promotion.",
			candidate.reason
		),
		source: candidate.source_refs.first().cloned(),
	};

	if candidate.reason == "conflict" {
		ConsolidationMarkers { contradictions: vec![marker], staleness: Vec::new() }
	} else {
		ConsolidationMarkers { contradictions: Vec::new(), staleness: vec![marker] }
	}
}

pub(in crate::knowledge) fn candidate_run_input_refs(
	candidates: &[KnowledgeDeltaMemoryCandidate],
) -> Vec<ConsolidationInputRef> {
	let mut seen = BTreeSet::new();
	let mut out = Vec::new();

	for source_ref in candidates.iter().flat_map(|candidate| &candidate.source_refs) {
		if seen.insert((source_ref.kind.as_str().to_string(), source_ref.id)) {
			out.push(source_ref.clone());
		}
	}

	out
}

pub(in crate::knowledge) fn knowledge_delta_source_snapshot(
	changed_sources: &[KnowledgePageChangedSource],
	candidates: &[KnowledgeDeltaMemoryCandidate],
) -> Value {
	serde_json::json!({
		"schema": "elf.knowledge_delta.run_source_snapshot/v1",
		"changed_sources": changed_sources,
		"candidate_count": candidates.len(),
		"candidate_reasons": candidates
			.iter()
			.map(|candidate| candidate.reason.clone())
			.collect::<Vec<_>>(),
		"source_mutation_allowed": false,
	})
}

pub(in crate::knowledge) fn proposal_run_summary(
	created: ConsolidationRunCreateResponse,
	proposal_count: usize,
) -> KnowledgePageProposalRunSummary {
	KnowledgePageProposalRunSummary {
		run_id: created.run.run_id,
		job_id: created.job_id,
		proposal_count,
		review_surface: "consolidation_proposals".to_string(),
	}
}
