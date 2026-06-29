use crate::knowledge::support::{
	BTreeMap, BTreeSet, DraftSection, KNOWLEDGE_PAGE_CONTRACT_SCHEMA_V1,
	KNOWLEDGE_PAGE_SOURCE_COVERAGE_SCHEMA_V1, KnowledgePageKind, KnowledgeSourceKind, Map,
	SourceSnapshot, Value, serde_json,
};

pub(in crate::knowledge) fn coverage_complete(coverage: Option<&Map<String, Value>>) -> bool {
	let Some(coverage) = coverage else {
		return false;
	};
	let source_count = coverage.get("source_count").and_then(Value::as_u64).unwrap_or(0);
	let cited_count = coverage.get("cited_source_count").and_then(Value::as_u64).unwrap_or(0);
	let complete = coverage.get("coverage_complete").and_then(Value::as_bool).unwrap_or(false);

	complete && source_count == cited_count
}

pub(in crate::knowledge) fn citation_count(citations: &Value) -> usize {
	citations.as_array().map(Vec::len).unwrap_or_default()
}

pub(in crate::knowledge) fn source_indexes(
	sources: &[SourceSnapshot],
	kind: KnowledgeSourceKind,
) -> Vec<usize> {
	sources
		.iter()
		.enumerate()
		.filter_map(|(index, source)| (source.kind == kind).then_some(index))
		.collect()
}

pub(in crate::knowledge) fn citations_value(
	section: &DraftSection,
	sources: &[SourceSnapshot],
) -> Value {
	Value::Array(
		section
			.source_indexes
			.iter()
			.filter_map(|index| sources.get(*index))
			.map(source_citation_value)
			.collect(),
	)
}

pub(in crate::knowledge) fn source_citation_value(source: &SourceSnapshot) -> Value {
	serde_json::json!({
		"source_kind": source.kind.as_str(),
		"source_id": source.id,
		"source_status": source.status.clone(),
		"source_updated_at": source.updated_at,
		"source_content_hash": source.content_hash.clone(),
		"source_snapshot": source.snapshot.clone(),
		"citation_metadata": source.citation_metadata.clone(),
	})
}

pub(in crate::knowledge) fn source_snapshot_value(sources: &[SourceSnapshot]) -> Value {
	serde_json::json!({
		"schema": KNOWLEDGE_PAGE_CONTRACT_SCHEMA_V1,
		"sources": sources.iter().map(source_citation_value).collect::<Vec<_>>(),
	})
}

pub(in crate::knowledge) fn source_coverage_value(
	page_kind: KnowledgePageKind,
	page_key: &str,
	sections: &[DraftSection],
	sources: &[SourceSnapshot],
) -> Value {
	let cited = sections
		.iter()
		.flat_map(|section| section.source_indexes.iter().copied())
		.collect::<BTreeSet<_>>();
	let counts = source_counts(sources);

	serde_json::json!({
		"schema": KNOWLEDGE_PAGE_SOURCE_COVERAGE_SCHEMA_V1,
		"page_kind": page_kind.as_str(),
		"page_key": page_key,
		"source_counts": counts,
		"source_count": sources.len(),
		"cited_source_count": cited.len(),
		"section_count": sections.len(),
		"unsupported_section_count": sections.iter().filter(|section| section.unsupported_reason.is_some()).count(),
		"coverage_complete": cited.len() == sources.len(),
	})
}

pub(in crate::knowledge) fn source_counts(sources: &[SourceSnapshot]) -> Value {
	let mut counts = BTreeMap::<&str, usize>::new();

	for source in sources {
		*counts.entry(source.kind.as_str()).or_insert(0) += 1;
	}

	serde_json::json!(counts)
}
