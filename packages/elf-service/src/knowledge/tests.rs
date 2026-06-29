use std::{
	collections::{BTreeSet, HashSet},
	slice,
};

use crate::{
	access::SharedSpaceGrantKey,
	knowledge::{
		self, DraftSection, KnowledgeDeltaMemoryCandidate, KnowledgePage, KnowledgePageKind,
		KnowledgePageResponse, KnowledgePageSearchRow, KnowledgePageSection,
		KnowledgePageSectionResponse, KnowledgePageSourceRef, KnowledgePageSourceRefResponse,
		KnowledgePageSummary, KnowledgeSourceKind, LintDraft, OffsetDateTime, SourceSnapshot, Uuid,
	},
};
use elf_domain::consolidation::ConsolidationApplyIntent;

fn test_source(kind: KnowledgeSourceKind, raw_id: u128, line: &str) -> SourceSnapshot {
	let id = Uuid::from_u128(raw_id);
	let content_hash = knowledge::hash_text(line);

	SourceSnapshot {
		kind,
		id,
		status: Some("active".to_string()),
		updated_at: Some(OffsetDateTime::UNIX_EPOCH),
		content_hash: Some(content_hash.clone()),
		snapshot: serde_json::json!({
			"kind": kind.as_str(),
			"id": id,
			"status": "active",
			"updated_at": OffsetDateTime::UNIX_EPOCH,
			"content_hash": content_hash,
		}),
		citation_metadata: serde_json::json!({ "fixture": "knowledge_unit" }),
		line: line.to_string(),
	}
}

fn test_rebuild_request(page_kind: KnowledgePageKind) -> knowledge::KnowledgePageRebuildRequest {
	knowledge::KnowledgePageRebuildRequest {
		tenant_id: "tenant".to_string(),
		project_id: "project".to_string(),
		agent_id: "agent".to_string(),
		page_kind,
		page_key: "elf".to_string(),
		title: Some("ELF".to_string()),
		doc_ids: Vec::new(),
		doc_chunk_ids: Vec::new(),
		note_ids: Vec::new(),
		event_ids: Vec::new(),
		relation_ids: Vec::new(),
		proposal_ids: Vec::new(),
		provider_metadata: knowledge::empty_object(),
	}
}

#[test]
fn build_sections_preserves_citations_and_deterministic_hashes() {
	let sources = vec![
		test_source(KnowledgeSourceKind::Doc, 1, "A source document supports the page."),
		test_source(KnowledgeSourceKind::DocChunk, 2, "A source span supports the page."),
		test_source(KnowledgeSourceKind::Note, 3, "A source note supports the page."),
		test_source(KnowledgeSourceKind::Event, 4, "An event audit supports the page."),
		test_source(KnowledgeSourceKind::Relation, 5, "A relation supports the page."),
		test_source(KnowledgeSourceKind::Proposal, 6, "An applied proposal supports the page."),
	];
	let mut first_sections = knowledge::build_sections(&sources).expect("sections should build");

	for section in &mut first_sections {
		section.citations = knowledge::citations_value(section, &sources);
		section.content_hash = knowledge::hash_json(&knowledge::section_hash_payload(section))
			.expect("section hash should serialize");
	}

	assert_eq!(first_sections.len(), 6);
	assert!(first_sections.iter().all(|section| {
		section.citations.as_array().is_some_and(|citations| !citations.is_empty())
	}));

	let coverage = knowledge::source_coverage_value(
		KnowledgePageKind::Project,
		"elf",
		&first_sections,
		&sources,
	);
	let request = test_rebuild_request(KnowledgePageKind::Project);
	let metadata = knowledge::rebuild_metadata("source-hash", &knowledge::empty_object(), &request);
	let first_hash = knowledge::page_content_hash("ELF", &first_sections, &coverage, &metadata)
		.expect("page hash should serialize");
	let second_hash = knowledge::page_content_hash("ELF", &first_sections, &coverage, &metadata)
		.expect("page hash should serialize");

	assert_eq!(coverage["coverage_complete"], true);
	assert_eq!(metadata["deterministic"], true);
	assert_eq!(metadata["memory_candidate_policy"]["direct_memory_ledger_mutation_allowed"], false);
	assert_eq!(first_hash, second_hash);
}

#[test]
fn rebuild_metadata_records_llm_variance() {
	let metadata = knowledge::rebuild_metadata(
		"source-hash",
		&serde_json::json!({
			"llm_derived": true,
			"provider_id": "fixture",
			"model": "fixture-model",
		}),
		&test_rebuild_request(KnowledgePageKind::Timeline),
	);

	assert_eq!(metadata["deterministic"], false);
	assert!(metadata["allowed_variance"].as_array().is_some_and(|items| !items.is_empty()));
	assert_eq!(metadata["provider_metadata"]["provider_id"], "fixture");
	assert_eq!(metadata["generated_by"]["actor_agent_id"], "agent");
}

#[test]
fn generated_titles_cover_author_and_timeline_pages() {
	assert_eq!(
		knowledge::generated_title(KnowledgePageKind::Author, "ada"),
		"Author Knowledge Page: ada"
	);
	assert_eq!(
		knowledge::generated_title(KnowledgePageKind::Timeline, "release-plan"),
		"Timeline Knowledge Page: release-plan"
	);
}

#[test]
fn previous_version_diff_records_delta_without_changing_content_hash() {
	let previous = test_page();
	let previous_section =
		test_section(Uuid::from_u128(10), "source-notes", serde_json::json!([]), None);
	let sections = vec![DraftSection {
		section_id: Uuid::from_u128(12),
		section_key: "source-notes".to_string(),
		heading: "source-notes".to_string(),
		role: "current_truth".to_string(),
		content: "Updated section content.".to_string(),
		ordinal: 0,
		source_indexes: vec![0],
		unsupported_reason: None,
		content_hash: "new-section-hash".to_string(),
		citations: serde_json::json!([{ "source_kind": "note" }]),
	}];
	let request = test_rebuild_request(KnowledgePageKind::Project);
	let base_metadata =
		knowledge::rebuild_metadata("new-source-hash", &knowledge::empty_object(), &request);
	let coverage = serde_json::json!({ "coverage_complete": true });
	let hash_without_diff =
		knowledge::page_content_hash("ELF", &sections, &coverage, &base_metadata)
			.expect("stable hash should serialize");
	let diff = knowledge::previous_version_diff_value(
		Some(&previous),
		&[previous_section],
		"ELF",
		"new-source-hash",
		hash_without_diff.as_str(),
		&sections,
	);
	let version_identity = knowledge::version_identity_value(
		KnowledgePageKind::Project,
		"elf",
		"new-source-hash",
		hash_without_diff.as_str(),
		&sections,
	);
	let metadata_with_diff = knowledge::rebuild_metadata_with_previous_version_diff(
		base_metadata,
		diff.clone(),
		version_identity,
	);
	let hash_with_diff =
		knowledge::page_content_hash("ELF", &sections, &coverage, &metadata_with_diff)
			.expect("hash should ignore previous-version diff metadata");

	assert_eq!(hash_without_diff, hash_with_diff);
	assert_eq!(diff["schema"], "elf.knowledge_page.version_diff/v1");
	assert_eq!(diff["available"], true);
	assert_eq!(diff["source_mutation_allowed"], false);
	assert_eq!(diff["section_changed_count"], 1);
	assert_eq!(
		knowledge::previous_version_diff_from_metadata(&metadata_with_diff)
			.expect("diff should be extractable")["section_changed_count"],
		1
	);
	assert_eq!(
		metadata_with_diff["version_identity"]["schema"],
		"elf.knowledge_page.version_identity/v1"
	);
}

#[test]
fn stale_source_comparison_detects_changed_snapshot() {
	let source_id = Uuid::from_u128(42);
	let stored = KnowledgePageSourceRef {
		ref_id: Uuid::from_u128(1),
		page_id: Uuid::from_u128(2),
		section_id: Some(Uuid::from_u128(3)),
		source_kind: "note".to_string(),
		source_id,
		source_status: Some("active".to_string()),
		source_updated_at: Some(OffsetDateTime::UNIX_EPOCH),
		source_content_hash: Some("old-hash".to_string()),
		source_snapshot: serde_json::json!({}),
		citation_metadata: serde_json::json!({}),
		created_at: OffsetDateTime::UNIX_EPOCH,
	};
	let current = SourceSnapshot {
		kind: KnowledgeSourceKind::Note,
		id: source_id,
		status: Some("active".to_string()),
		updated_at: Some(OffsetDateTime::UNIX_EPOCH),
		content_hash: Some("new-hash".to_string()),
		snapshot: serde_json::json!({}),
		citation_metadata: serde_json::json!({}),
		line: "Updated note source.".to_string(),
	};
	let finding = knowledge::stale_source_finding(&stored, &current);

	assert!(knowledge::source_changed(&stored, &current));
	assert_eq!(finding.finding_type, "stale_source_ref");
	assert_eq!(finding.source_kind, Some(KnowledgeSourceKind::Note));
	assert_eq!(finding.source_id, Some(source_id));
}

#[test]
fn watch_rebuild_outputs_cover_source_update_and_stale_page() {
	let section_id = Uuid::from_u128(50);
	let source_id = Uuid::from_u128(51);
	let section = test_section(
		section_id,
		"source-notes",
		serde_json::json!([{ "source_kind": "note", "source_id": source_id }]),
		None,
	);
	let source_ref = test_source_ref_for(section_id, source_id, "old-hash");
	let lint = vec![LintDraft {
		section_id: Some(section_id),
		finding_type: "stale_source_ref".to_string(),
		severity: "warning".to_string(),
		source_kind: Some(KnowledgeSourceKind::Note),
		source_id: Some(source_id),
		message: "Knowledge page source reference snapshot is stale.".to_string(),
		details: serde_json::json!({ "stored": "old", "current": "new" }),
	}];
	let diff = serde_json::json!({
		"available": true,
		"content_changed": true,
		"changed_section_keys": ["source-notes"]
	});
	let changed_sources = vec![knowledge::KnowledgePageChangedSource {
		source_kind: KnowledgeSourceKind::Note,
		source_id,
	}];
	let outputs =
		knowledge::rebuild_outputs(&[section], &[source_ref], &lint, Some(&diff), &changed_sources);
	let output_types = outputs.iter().map(|output| output.output_type.as_str()).collect::<Vec<_>>();

	assert!(output_types.contains(&"stale_section"));
	assert!(output_types.contains(&"changed_claim"));
	assert!(output_types.contains(&"conflict"));
	assert!(output_types.contains(&"changed_source"));
}

#[test]
fn memory_candidate_uses_reviewable_consolidation_proposal_contract() {
	let section_id = Uuid::from_u128(60);
	let source_id = Uuid::from_u128(61);
	let page = test_page_response(section_id, source_id);
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

	assert_candidate_is_reviewable(&candidates[0]);

	let proposal = knowledge::candidate_proposal_input(&candidates[0]);

	assert_eq!(proposal.apply_intent, ConsolidationApplyIntent::CreateDerivedNote);
	assert_eq!(proposal.source_refs.len(), 1);
	assert_eq!(proposal.proposed_payload["source_ref"]["source_mutation_allowed"], false);
	assert_eq!(proposal.proposed_payload["source_ref"]["reason"], "changed_claim");
	assert!(!proposal.markers.staleness.is_empty());
}

#[test]
fn lint_page_sections_detects_unsupported_missing_and_low_coverage() {
	let page = test_page();
	let unsupported = test_section(
		Uuid::from_u128(10),
		"unsupported",
		serde_json::json!([]),
		Some("No source supports this claim.".to_string()),
	);
	let missing = test_section(Uuid::from_u128(11), "missing", serde_json::json!([]), None);
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

#[test]
fn search_item_marks_derived_page_snippet_with_provenance() {
	let section_id = Uuid::from_u128(20);
	let source_ref = test_source_ref(section_id);
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
	let mut active = test_source_ref(section_id);
	let mut deleted = test_source_ref(section_id);
	let mut ignored = test_source_ref(section_id);
	let current_keys = current_source_keys_for(&[&active, &deleted, &ignored]);

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
	let mut excluded = test_source_ref(section_id);
	let mut source_ref_span = test_source_ref(section_id);
	let mut policy_span = test_source_ref(section_id);
	let mut malformed_span = test_source_ref(section_id);
	let current_keys =
		current_source_keys_for(&[&excluded, &source_ref_span, &policy_span, &malformed_span]);

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
	let mut proposal = test_source_ref_for(section_id, Uuid::from_u128(74), "proposal-hash");

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

	let current_keys = current_source_keys_for(&[&proposal]);

	assert!(!knowledge::recallable_source_refs(&[proposal], &current_keys));
}

#[test]
fn search_item_sanitizes_proposal_citations_and_source_refs() {
	let section_id = Uuid::from_u128(75);
	let mut source_ref = test_source_ref_for(section_id, Uuid::from_u128(76), "proposal-hash");

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
	let source_ref = test_source_ref(section_id);

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

fn test_page() -> KnowledgePage {
	KnowledgePage {
		page_id: Uuid::from_u128(1),
		tenant_id: "tenant".to_string(),
		project_id: "project".to_string(),
		page_kind: "project".to_string(),
		page_key: "elf".to_string(),
		title: "ELF".to_string(),
		contract_schema: "elf.knowledge_page/v1".to_string(),
		status: "active".to_string(),
		rebuild_source_hash: "source-hash".to_string(),
		content_hash: "content-hash".to_string(),
		source_coverage: serde_json::json!({
			"source_count": 2,
			"cited_source_count": 1,
			"coverage_complete": false
		}),
		source_snapshot: serde_json::json!({}),
		rebuild_metadata: serde_json::json!({}),
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
		rebuilt_at: OffsetDateTime::UNIX_EPOCH,
	}
}

fn test_section(
	section_id: Uuid,
	section_key: &str,
	citations: serde_json::Value,
	unsupported_reason: Option<String>,
) -> KnowledgePageSection {
	KnowledgePageSection {
		section_id,
		page_id: Uuid::from_u128(1),
		section_key: section_key.to_string(),
		heading: section_key.to_string(),
		role: "current_truth".to_string(),
		content: "Section content.".to_string(),
		ordinal: 0,
		citations,
		unsupported_reason,
		content_hash: "section-hash".to_string(),
		created_at: OffsetDateTime::UNIX_EPOCH,
		updated_at: OffsetDateTime::UNIX_EPOCH,
	}
}

fn test_source_ref(section_id: Uuid) -> KnowledgePageSourceRef {
	test_source_ref_for(section_id, Uuid::from_u128(31), "source-hash")
}

fn test_source_ref_for(
	section_id: Uuid,
	source_id: Uuid,
	source_hash: &str,
) -> KnowledgePageSourceRef {
	KnowledgePageSourceRef {
		ref_id: Uuid::from_u128(30),
		page_id: Uuid::from_u128(21),
		section_id: Some(section_id),
		source_kind: "note".to_string(),
		source_id,
		source_status: Some("active".to_string()),
		source_updated_at: Some(OffsetDateTime::UNIX_EPOCH),
		source_content_hash: Some(source_hash.to_string()),
		source_snapshot: serde_json::json!({
			"schema": "test_source/v1",
			"source_id": source_id,
			"content_hash": source_hash,
		}),
		citation_metadata: serde_json::json!({}),
		created_at: OffsetDateTime::UNIX_EPOCH,
	}
}

fn current_source_keys_for(source_refs: &[&KnowledgePageSourceRef]) -> BTreeSet<String> {
	source_refs
		.iter()
		.map(|source_ref| {
			knowledge::current_key(source_ref.source_kind.as_str(), source_ref.source_id)
		})
		.collect()
}

fn test_page_response(section_id: Uuid, source_id: Uuid) -> KnowledgePageResponse {
	let page = test_page();
	let section = test_section(
		section_id,
		"source-notes",
		serde_json::json!([{ "source_kind": "note", "source_id": source_id }]),
		None,
	);
	let source_ref = test_source_ref_for(section_id, source_id, "new-hash");

	KnowledgePageResponse {
		page: KnowledgePageSummary::from(page),
		sections: vec![KnowledgePageSectionResponse {
			citation_count: 1,
			source_ref_count: 1,
			coverage_complete: true,
			source_backlinks: Vec::new(),
			..KnowledgePageSectionResponse::from(section)
		}],
		source_refs: vec![KnowledgePageSourceRefResponse::from(source_ref)],
		lint_findings: Vec::new(),
	}
}

fn assert_candidate_is_reviewable(candidate: &KnowledgeDeltaMemoryCandidate) {
	assert_eq!(candidate.reason, "changed_claim");
	assert_eq!(candidate.source_refs.len(), 1);
	assert_eq!(candidate.source_refs[0].kind.as_str(), "note");
	assert_eq!(candidate.source_snapshot["source_mutation_allowed"], false);
	assert_eq!(candidate.diff.after["reason"], "changed_claim");
	assert_eq!(candidate.proposed_payload["type"], "plan");
	assert_eq!(candidate.proposed_payload["source_ref"]["schema"], "elf.knowledge_delta/v1");
}
