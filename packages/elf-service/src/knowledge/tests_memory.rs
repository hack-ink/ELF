use crate::knowledge::{self, Uuid, tests::tests_helpers};
use elf_domain::consolidation::ConsolidationApplyIntent;

#[test]
fn memory_candidate_uses_reviewable_consolidation_proposal_contract() {
	let section_id = Uuid::from_u128(60);
	let source_id = Uuid::from_u128(61);
	let page = tests_helpers::test_page_response(section_id, source_id);
	let outputs = vec![knowledge::KnowledgePageRebuildOutput {
		output_type: "changed_claim".to_string(),
		severity: "info".to_string(),
		section_key: Some("source-notes".to_string()),
		source_kind: Some("note".to_string()),
		source_id: Some(source_id),
		message: "Changed section.".to_string(),
		details: serde_json::json!({ "reason": "source_update" }),
	}];
	let candidates = knowledge::memory_candidates_for_page(&page, &outputs);

	assert_eq!(candidates.len(), 1);

	tests_helpers::assert_candidate_is_reviewable(&candidates[0]);

	let proposal = knowledge::candidate_proposal_input(&candidates[0]);

	assert_eq!(proposal.apply_intent, ConsolidationApplyIntent::CreateDerivedNote);
	assert_eq!(proposal.source_refs.len(), 1);
	assert_eq!(proposal.proposed_payload["source_ref"]["source_mutation_allowed"], false);
	assert_eq!(proposal.proposed_payload["source_ref"]["reason"], "changed_claim");
	assert!(!proposal.markers.staleness.is_empty());
}

#[test]
fn lint_page_sections_detects_unsupported_missing_and_low_coverage() {
	let page = tests_helpers::test_page();
	let unsupported = tests_helpers::test_section(
		Uuid::from_u128(10),
		"unsupported",
		serde_json::json!([]),
		Some("No source supports this claim.".to_string()),
	);
	let missing =
		tests_helpers::test_section(Uuid::from_u128(11), "missing", serde_json::json!([]), None);
	let findings = knowledge::lint_page_sections(&page, &[unsupported, missing], &[]);
	let finding_types =
		findings.iter().map(|finding| finding.finding_type.as_str()).collect::<Vec<_>>();

	assert!(finding_types.contains(&"unsupported_claim"));
	assert!(finding_types.contains(&"missing_citation"));
	assert!(finding_types.contains(&"missing_source_ref"));
	assert!(finding_types.contains(&"low_source_coverage"));
	assert!(findings.iter().all(|finding| {
		finding
			.details
			.get("repair_guidance")
			.and_then(serde_json::Value::as_str)
			.is_some_and(|guidance| !guidance.is_empty())
	}));
}
