use crate::knowledge::{
	ConsolidationApplyIntent, ConsolidationLineage, ConsolidationMarker,
	ConsolidationMarkerSeverity, ConsolidationMarkers, ConsolidationProposalDiff,
	ConsolidationProposalInput, KnowledgeDeltaMemoryCandidate, KnowledgePageResponse,
	KnowledgePageSectionResponse, Value, watch,
};

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
		target_ref: watch::empty_object(),
		proposed_payload: candidate.proposed_payload.clone(),
	}
}

pub(in crate::knowledge::watch::candidates) fn candidate_diff(
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

pub(in crate::knowledge::watch::candidates) fn candidate_proposed_payload(
	page: &KnowledgePageResponse,
	section: &KnowledgePageSectionResponse,
	reason: &str,
) -> Value {
	let text = watch::truncate_chars(
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

fn candidate_markers(candidate: &KnowledgeDeltaMemoryCandidate) -> ConsolidationMarkers {
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
