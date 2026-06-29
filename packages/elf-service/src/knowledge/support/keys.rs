use crate::knowledge::support::{
	BTreeSet, DEFAULT_LIST_LIMIT, MAX_LIST_LIMIT, SourceSnapshot, Uuid, serde_json,
};

pub(in crate::knowledge) fn source_sort_key(source: &SourceSnapshot) -> (String, Uuid) {
	(source.kind.as_str().to_string(), source.id)
}

pub(in crate::knowledge) fn source_key(source: &SourceSnapshot) -> String {
	current_key(source.kind.as_str(), source.id)
}

pub(in crate::knowledge) fn current_key(kind: &str, source_id: Uuid) -> String {
	format!("{kind}:{source_id}")
}

pub(in crate::knowledge) fn sorted_unique(ids: &[Uuid]) -> Vec<Uuid> {
	ids.iter().copied().collect::<BTreeSet<_>>().into_iter().collect()
}

pub(in crate::knowledge) fn bounded_limit(limit: Option<u32>) -> i64 {
	limit.map(i64::from).unwrap_or(DEFAULT_LIST_LIMIT).clamp(1, MAX_LIST_LIMIT)
}

pub(in crate::knowledge) fn source_span_id(
	content_hash: &str,
	start: usize,
	end: usize,
	span_kind: &str,
) -> Uuid {
	let name = serde_json::json!(["elf-doc-source-span/v1", content_hash, start, end, span_kind])
		.to_string();

	Uuid::new_v5(&Uuid::NAMESPACE_OID, name.as_bytes())
}
