use elf_service::KnowledgePageRebuildResponse;

pub(crate) fn assert_first_rebuild(first: &KnowledgePageRebuildResponse) {
	assert_eq!(first.page.sections.len(), 6);
	assert_eq!(first.page.source_refs.len(), 6);
	assert!(first.page.sections.iter().all(|section| {
		section.citations.as_array().is_some_and(|citations| !citations.is_empty())
	}));
	assert!(first.page.source_refs.iter().any(|source_ref| source_ref.source_kind == "doc"));
	assert!(first.page.source_refs.iter().any(|source_ref| source_ref.source_kind == "doc_chunk"));
	assert_eq!(first.page.page.source_coverage["coverage_complete"], true);
	assert_eq!(first.page.page.rebuild_metadata["deterministic"], true);
	assert_eq!(
		first.page.page.rebuild_metadata["generated_by"]["runtime"],
		"ElfService::knowledge_page_rebuild"
	);
	assert_eq!(
		first.page.page.rebuild_metadata["memory_candidate_policy"]["direct_memory_ledger_mutation_allowed"],
		false
	);
	assert_eq!(
		first.page.page.rebuild_metadata["version_identity"]["schema"],
		"elf.knowledge_page.version_identity/v1"
	);
	assert_eq!(
		first
			.page
			.page
			.previous_version_diff
			.as_ref()
			.expect("initial rebuild should expose no-previous diff")["available"],
		false
	);
}
