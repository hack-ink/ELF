use crate::knowledge::support::{
	BTreeMap, BTreeSet, DraftSection, KNOWLEDGE_PAGE_VERSION_DIFF_SCHEMA_V1, KnowledgePage,
	KnowledgePageSection, Value, serde_json,
};

pub(in crate::knowledge) fn previous_version_diff_value(
	previous: Option<&KnowledgePage>,
	previous_sections: &[KnowledgePageSection],
	new_title: &str,
	new_source_hash: &str,
	new_content_hash: &str,
	new_sections: &[DraftSection],
) -> Value {
	let Some(previous) = previous else {
		return serde_json::json!({
			"schema": KNOWLEDGE_PAGE_VERSION_DIFF_SCHEMA_V1,
			"available": false,
			"reason": "no_previous_version",
			"summary": "Initial rebuild; no previous knowledge page version exists.",
			"source_mutation_allowed": false,
		});
	};
	let previous_by_key = previous_sections
		.iter()
		.map(|section| (section.section_key.as_str(), section))
		.collect::<BTreeMap<_, _>>();
	let new_by_key = new_sections
		.iter()
		.map(|section| (section.section_key.as_str(), section))
		.collect::<BTreeMap<_, _>>();
	let previous_keys = previous_by_key.keys().copied().collect::<BTreeSet<_>>();
	let new_keys = new_by_key.keys().copied().collect::<BTreeSet<_>>();
	let added_section_keys = sorted_strings(new_keys.difference(&previous_keys).copied());
	let removed_section_keys = sorted_strings(previous_keys.difference(&new_keys).copied());
	let mut changed_section_keys = Vec::new();
	let mut unchanged_section_keys = Vec::new();

	for key in previous_keys.intersection(&new_keys).copied() {
		let previous_section = previous_by_key[key];
		let new_section = new_by_key[key];

		if previous_section.content_hash == new_section.content_hash
			&& previous_section.heading == new_section.heading
			&& previous_section.role == new_section.role
			&& previous_section.unsupported_reason == new_section.unsupported_reason
		{
			unchanged_section_keys.push(key.to_string());
		} else {
			changed_section_keys.push(key.to_string());
		}
	}

	let title_changed = previous.title != new_title;
	let source_changed = previous.rebuild_source_hash != new_source_hash;
	let content_changed = previous.content_hash != new_content_hash;
	let summary = version_diff_summary(
		title_changed,
		source_changed,
		content_changed,
		added_section_keys.len(),
		removed_section_keys.len(),
		changed_section_keys.len(),
	);

	serde_json::json!({
		"schema": KNOWLEDGE_PAGE_VERSION_DIFF_SCHEMA_V1,
		"available": true,
		"previous_page_id": previous.page_id,
		"previous_content_hash": previous.content_hash,
		"new_content_hash": new_content_hash,
		"previous_source_hash": previous.rebuild_source_hash,
		"new_source_hash": new_source_hash,
		"title_changed": title_changed,
		"source_changed": source_changed,
		"content_changed": content_changed,
		"section_added_count": added_section_keys.len(),
		"section_removed_count": removed_section_keys.len(),
		"section_changed_count": changed_section_keys.len(),
		"section_unchanged_count": unchanged_section_keys.len(),
		"added_section_keys": added_section_keys,
		"removed_section_keys": removed_section_keys,
		"changed_section_keys": changed_section_keys,
		"unchanged_section_keys": unchanged_section_keys,
		"source_mutation_allowed": false,
		"summary": summary,
	})
}

pub(in crate::knowledge) fn sorted_strings<'a>(
	items: impl Iterator<Item = &'a str>,
) -> Vec<String> {
	let mut out = items.map(ToString::to_string).collect::<Vec<_>>();

	out.sort();

	out
}

pub(in crate::knowledge) fn version_diff_summary(
	title_changed: bool,
	source_changed: bool,
	content_changed: bool,
	added: usize,
	removed: usize,
	changed: usize,
) -> String {
	if !title_changed
		&& !source_changed
		&& !content_changed
		&& added == 0
		&& removed == 0
		&& changed == 0
	{
		return "No page-level or section-level changes from the previous rebuild.".to_string();
	}

	format!(
		"Previous rebuild diff: title_changed={title_changed}, source_changed={source_changed}, content_changed={content_changed}, sections added={added}, removed={removed}, changed={changed}."
	)
}
