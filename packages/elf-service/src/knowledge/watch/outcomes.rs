use super::*;

pub(in crate::knowledge) fn successful_watch_rebuild(
	before_sections: Vec<KnowledgePageSection>,
	before_source_refs: Vec<KnowledgePageSourceRef>,
	before_lint: Vec<LintDraft>,
	rebuilt_page: KnowledgePageResponse,
	changed_sources: &[KnowledgePageChangedSource],
) -> WatchRebuildOutcome {
	let previous_version_diff = rebuilt_page.page.previous_version_diff.clone();
	let outputs = rebuild_outputs(
		&before_sections,
		&before_source_refs,
		&before_lint,
		previous_version_diff.as_ref(),
		changed_sources,
	);
	let sections = successful_section_states(&before_sections, &rebuilt_page.sections, &outputs);
	let rebuild_state = successful_rebuild_state(previous_version_diff.as_ref(), &outputs);
	let candidates = memory_candidates_for_page(&rebuilt_page, &outputs);
	let operator_summary = page_operator_summary(
		rebuilt_page.page.page_key.as_str(),
		rebuild_state.as_str(),
		outputs.len(),
		candidates.len(),
	);
	let item = KnowledgePageWatchRebuildItem {
		page_id: rebuilt_page.page.page_id,
		page_kind: rebuilt_page.page.page_kind.clone(),
		page_key: rebuilt_page.page.page_key.clone(),
		title: rebuilt_page.page.title.clone(),
		rebuild_state,
		sections,
		outputs,
		rebuilt_page: Some(rebuilt_page),
		blocked_reason: None,
		previous_version_diff,
		operator_summary,
	};

	WatchRebuildOutcome { item, candidates }
}

pub(in crate::knowledge) fn blocked_watch_rebuild(
	page: KnowledgePage,
	sections: Vec<KnowledgePageSection>,
	before_lint: Vec<LintDraft>,
	err: Error,
) -> WatchRebuildOutcome {
	let outputs = blocked_outputs(&sections, &before_lint, err.to_string().as_str());
	let section_states = blocked_section_states(&sections, &outputs);
	let operator_summary =
		page_operator_summary(page.page_key.as_str(), "blocked", outputs.len(), 0);
	let item = KnowledgePageWatchRebuildItem {
		page_id: page.page_id,
		page_kind: page.page_kind,
		page_key: page.page_key,
		title: page.title,
		rebuild_state: "blocked".to_string(),
		sections: section_states,
		outputs,
		rebuilt_page: None,
		blocked_reason: Some(err.to_string()),
		previous_version_diff: previous_version_diff_from_metadata(&page.rebuild_metadata),
		operator_summary,
	};

	WatchRebuildOutcome { item, candidates: Vec::new() }
}
