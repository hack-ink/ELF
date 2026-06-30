use std::{
	collections::{BTreeSet, HashSet},
	slice,
};

use crate::{
	access::SharedSpaceGrantKey,
	knowledge::{
		self, KnowledgePageSearchRow, KnowledgeSourceKind, OffsetDateTime, Uuid,
		tests::tests_helpers,
	},
};

#[test]
fn search_item_marks_derived_page_snippet_with_provenance() {
	let section_id = Uuid::from_u128(20);
	let source_ref = tests_helpers::test_source_ref(section_id);
	let row = KnowledgePageSearchRow {
		page_id: Uuid::from_u128(21),
		page_kind: "project".to_string(),
		page_key: "elf".to_string(),
		title: "ELF Knowledge".to_string(),
		status: "active".to_string(),
		source_coverage: serde_json::json!({
			"source_count": 1,
			"cited_source_count": 1,
			"coverage_complete": true
		}),
		rebuild_metadata: serde_json::json!({ "deterministic": true }),
		page_updated_at: OffsetDateTime::UNIX_EPOCH,
		rebuilt_at: OffsetDateTime::UNIX_EPOCH,
		section_id,
		section_key: "source-notes".to_string(),
		heading: "Source Notes".to_string(),
		role: "current_truth".to_string(),
		content: "Derived knowledge pages cite source notes before they are trusted.".to_string(),
		ordinal: 0,
		citations: serde_json::json!([{ "source_kind": "note", "source_id": source_ref.source_id }]),
		unsupported_reason: None,
		lint_error_count: 0,
		lint_warning_count: 1,
		lint_info_count: 0,
		section_source_ref_count: 1,
	};
	let item = knowledge::knowledge_page_search_item(row, vec![source_ref], "source notes");

	assert_eq!(item.result_kind, "knowledge_page_section");
	assert_eq!(item.trust_state, "derived_warning");
	assert_eq!(item.citation_count, 1);
	assert_eq!(item.source_ref_count, 1);
	assert_eq!(item.source_refs.len(), 1);
	assert!(item.derived_notice.contains("Derived knowledge page snippet"));
	assert!(item.repair_guidance.is_some());
	assert!(item.snippet.contains("source notes"));
}

#[test]
fn search_source_refs_suppress_deleted_and_unreviewed_sources() {
	let section_id = Uuid::from_u128(70);
	let mut active = tests_helpers::test_source_ref(section_id);
	let mut deleted = tests_helpers::test_source_ref(section_id);
	let mut ignored = tests_helpers::test_source_ref(section_id);
	let current_keys = tests_helpers::current_source_keys_for(&[&active, &deleted, &ignored]);

	deleted.source_status = Some("deleted".to_string());
	ignored.source_status = Some("ignore".to_string());

	assert!(knowledge::recallable_source_refs(slice::from_ref(&active), &current_keys));
	assert!(!knowledge::recallable_source_refs(&[deleted], &current_keys));
	assert!(!knowledge::recallable_source_refs(&[ignored], &current_keys));

	active.source_status = None;

	assert!(!knowledge::recallable_source_refs(&[active], &current_keys));
}

#[test]
fn search_source_refs_suppress_non_captured_spans() {
	let section_id = Uuid::from_u128(71);
	let mut excluded = tests_helpers::test_source_ref(section_id);
	let mut source_ref_span = tests_helpers::test_source_ref(section_id);
	let mut policy_span = tests_helpers::test_source_ref(section_id);
	let mut malformed_span = tests_helpers::test_source_ref(section_id);
	let current_keys = tests_helpers::current_source_keys_for(&[
		&excluded,
		&source_ref_span,
		&policy_span,
		&malformed_span,
	]);

	excluded.source_snapshot = serde_json::json!({
		"source_span": {
			"schema": "doc_source_span/v1",
			"status": "excluded",
			"reason_code": "WRITE_POLICY_EXCLUSION"
		}
	});
	source_ref_span.source_snapshot = serde_json::json!({
		"source_ref": {
			"source_spans": [
				{
					"schema": "doc_source_span/v1",
					"status": "redacted",
					"reason_code": "WRITE_POLICY_REDACTION"
				}
			]
		}
	});
	policy_span.source_snapshot = serde_json::json!({
		"source_ref": {
			"policy_spans": [
				{
					"schema": "doc_source_span/v1",
					"status": "excluded",
					"reason_code": "WRITE_POLICY_EXCLUSION"
				}
			]
		}
	});
	malformed_span.source_snapshot = serde_json::json!({
		"source_span": {
			"schema": "doc_source_span/v1",
			"reason_code": "WRITE_POLICY_REDACTION"
		}
	});

	assert!(!knowledge::recallable_source_refs(&[excluded], &current_keys));
	assert!(!knowledge::recallable_source_refs(&[source_ref_span], &current_keys));
	assert!(!knowledge::recallable_source_refs(&[policy_span], &current_keys));
	assert!(!knowledge::recallable_source_refs(&[malformed_span], &current_keys));
}

#[test]
fn search_source_refs_suppress_nested_proposal_non_captured_spans() {
	let section_id = Uuid::from_u128(73);
	let mut proposal =
		tests_helpers::test_source_ref_for(section_id, Uuid::from_u128(74), "proposal-hash");

	proposal.source_kind = KnowledgeSourceKind::Proposal.as_str().to_string();
	proposal.source_status = Some("applied".to_string());
	proposal.source_snapshot = serde_json::json!({
		"kind": "proposal",
		"proposal_id": proposal.source_id,
		"source_refs": [
			{
				"kind": "doc_chunk",
				"source_ref": {
					"policy_spans": [
						{
							"schema": "doc_source_span/v1",
							"status": "excluded",
							"reason_code": "WRITE_POLICY_EXCLUSION"
						}
					]
				}
			}
		],
		"source_snapshot": {
			"sources": [
				{
					"source_snapshot": {
						"source_span": {
							"schema": "doc_source_span/v1",
							"status": "redacted",
							"reason_code": "WRITE_POLICY_REDACTION"
						}
					}
				}
			]
		},
		"diff": {
			"after": {
				"source_ref": {
					"source_spans": [
						{
							"schema": "doc_source_span/v1",
							"status": "excluded",
							"reason_code": "WRITE_POLICY_EXCLUSION"
						}
					]
				}
			}
		}
	});

	let current_keys = tests_helpers::current_source_keys_for(&[&proposal]);

	assert!(!knowledge::recallable_source_refs(&[proposal], &current_keys));
}

#[test]
fn search_item_sanitizes_proposal_citations_and_source_refs() {
	let section_id = Uuid::from_u128(75);
	let mut source_ref =
		tests_helpers::test_source_ref_for(section_id, Uuid::from_u128(76), "proposal-hash");

	source_ref.source_kind = KnowledgeSourceKind::Proposal.as_str().to_string();
	source_ref.source_status = Some("applied".to_string());
	source_ref.source_snapshot = serde_json::json!({
		"kind": "proposal",
		"proposal_id": source_ref.source_id,
		"proposal_kind": "create_derived_note",
		"source_refs": [{ "kind": "doc", "source_id": Uuid::from_u128(77) }],
		"source_snapshot": { "sources": [{ "source_snapshot": { "text": "private raw source" } }] },
		"lineage": { "parents": ["private"] },
		"diff": { "summary": "private raw diff" },
		"unsupported_claim_flags": [{ "quote": "private raw flag" }],
		"target_ref": { "text": "private raw target" }
	});

	let row = KnowledgePageSearchRow {
		page_id: Uuid::from_u128(78),
		page_kind: "project".to_string(),
		page_key: "elf".to_string(),
		title: "ELF Knowledge".to_string(),
		status: "active".to_string(),
		source_coverage: serde_json::json!({
			"source_count": 1,
			"cited_source_count": 1,
			"coverage_complete": true
		}),
		rebuild_metadata: serde_json::json!({ "deterministic": true }),
		page_updated_at: OffsetDateTime::UNIX_EPOCH,
		rebuilt_at: OffsetDateTime::UNIX_EPOCH,
		section_id,
		section_key: "reviewed-proposals".to_string(),
		heading: "Reviewed Proposals".to_string(),
		role: "proposals".to_string(),
		content: "Applied proposal create_derived_note".to_string(),
		ordinal: 0,
		citations: serde_json::json!([{
			"source_kind": "proposal",
			"source_id": source_ref.source_id,
			"source_snapshot": source_ref.source_snapshot.clone()
		}]),
		unsupported_reason: None,
		lint_error_count: 0,
		lint_warning_count: 0,
		lint_info_count: 0,
		section_source_ref_count: 1,
	};
	let item = knowledge::knowledge_page_search_item(row, vec![source_ref], "proposal");
	let citation_snapshot = &item.citations[0]["source_snapshot"];
	let source_ref_snapshot = &item.source_refs[0].source_snapshot;

	assert_eq!(citation_snapshot["sanitized"], true);
	assert_eq!(source_ref_snapshot["sanitized"], true);
	assert!(citation_snapshot.get("source_refs").is_none());
	assert!(citation_snapshot.get("source_snapshot").is_none());
	assert!(citation_snapshot.get("diff").is_none());
	assert!(source_ref_snapshot.get("source_refs").is_none());
	assert!(source_ref_snapshot.get("source_snapshot").is_none());
	assert!(source_ref_snapshot.get("diff").is_none());
}

#[test]
fn search_source_refs_suppress_missing_current_sources() {
	let section_id = Uuid::from_u128(72);
	let source_ref = tests_helpers::test_source_ref(section_id);

	assert!(!knowledge::recallable_source_refs(&[source_ref], &BTreeSet::new()));
}

#[test]
fn source_row_read_allowed_requires_shared_grant_for_other_agent_sources() {
	let allowed_scopes = vec!["agent_private".to_string(), "project_shared".to_string()];
	let shared_grants = HashSet::new();

	assert!(knowledge::source_row_read_allowed(
		"owner-agent",
		"project_shared",
		Some("owner-agent"),
		&allowed_scopes,
		&shared_grants
	));
	assert!(!knowledge::source_row_read_allowed(
		"owner-agent",
		"project_shared",
		Some("reader-agent"),
		&allowed_scopes,
		&shared_grants
	));

	let shared_grants = HashSet::from([SharedSpaceGrantKey {
		scope: "project_shared".to_string(),
		space_owner_agent_id: "owner-agent".to_string(),
	}]);

	assert!(knowledge::source_row_read_allowed(
		"owner-agent",
		"project_shared",
		Some("reader-agent"),
		&allowed_scopes,
		&shared_grants
	));
}
