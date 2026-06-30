use crate::recall_debug::{self, BTreeMap, NoteDebugSourceRow, SearchExplainItem, Uuid, Value};

pub(in crate::recall_debug::replay) fn compact_selected_context(
	selected_items: &[&SearchExplainItem],
	source_refs: &BTreeMap<Uuid, NoteDebugSourceRow>,
) -> Vec<Value> {
	selected_items
		.iter()
		.map(|item| {
			let source = source_refs.get(&item.note_id);

			serde_json::json!({
				"result_handle": item.result_handle,
				"note_id": item.note_id,
				"chunk_id": item.chunk_id,
				"source_ref": source.map(|row| row.source_ref.clone()),
				"source_ref_available": source.is_some(),
				"freshness_state": recall_debug::freshness_from_note_source(source),
				"final_rank": item.rank,
				"final_score": item.explain.ranking.final_score,
				"policy_id": item.explain.ranking.policy_id,
				"policy_reason": "final ranked search result",
				"ranking_terms": item
					.explain
					.ranking
					.terms
					.iter()
					.map(|term| serde_json::json!({
						"name": term.name,
						"value": term.value,
					}))
					.collect::<Vec<_>>(),
				"relation_context_count": item
					.explain
					.relation_context
					.as_ref()
					.map(Vec::len)
					.unwrap_or_default(),
			})
		})
		.collect()
}
