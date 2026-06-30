use crate::knowledge::{
	self, DraftSection, KnowledgePageKind, KnowledgePageSourceRef, KnowledgeSourceKind, LintDraft,
	OffsetDateTime, SourceSnapshot, Uuid, tests::tests_helpers,
};

#[test]
fn build_sections_preserves_citations_and_deterministic_hashes() {
	let sources = vec![
		tests_helpers::test_source(
			KnowledgeSourceKind::Doc,
			1,
			"A source document supports the page.",
		),
		tests_helpers::test_source(
			KnowledgeSourceKind::DocChunk,
			2,
			"A source span supports the page.",
		),
		tests_helpers::test_source(
			KnowledgeSourceKind::Note,
			3,
			"A source note supports the page.",
		),
		tests_helpers::test_source(
			KnowledgeSourceKind::Event,
			4,
			"An event audit supports the page.",
		),
		tests_helpers::test_source(
			KnowledgeSourceKind::Relation,
			5,
			"A relation supports the page.",
		),
		tests_helpers::test_source(
			KnowledgeSourceKind::Proposal,
			6,
			"An applied proposal supports the page.",
		),
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
	let request = tests_helpers::test_rebuild_request(KnowledgePageKind::Project);
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
		&tests_helpers::test_rebuild_request(KnowledgePageKind::Timeline),
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
	let previous = tests_helpers::test_page();
	let previous_section = tests_helpers::test_section(
		Uuid::from_u128(10),
		"source-notes",
		serde_json::json!([]),
		None,
	);
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
	let request = tests_helpers::test_rebuild_request(KnowledgePageKind::Project);
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
	let section = tests_helpers::test_section(
		section_id,
		"source-notes",
		serde_json::json!([{ "source_kind": "note", "source_id": source_id }]),
		None,
	);
	let source_ref = tests_helpers::test_source_ref_for(section_id, source_id, "old-hash");
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
