use crate::{
	ConsolidationInputRef, ConsolidationSourceKind, ConsolidationSourceSnapshot, CorpusText,
	IngestedCorpus, LoadedJob, OffsetDateTime, Result, Uuid, eyre, serde_json,
};

pub(in crate::consolidation_adapter) fn live_note_ids(ingested: &IngestedCorpus) -> Vec<Uuid> {
	let mut note_ids = Vec::new();

	for ids in ingested.note_ids_by_evidence.values() {
		for note_id in ids {
			if !note_ids.iter().any(|existing| existing == note_id) {
				note_ids.push(*note_id);
			}
		}
	}

	note_ids
}

pub(in crate::consolidation_adapter) fn consolidation_input_refs(
	loaded: &LoadedJob,
	adapter_id: &str,
	evidence_ids: &[String],
	ingested: &IngestedCorpus,
	corpus: &[CorpusText],
) -> Result<Vec<ConsolidationInputRef>> {
	evidence_ids
		.iter()
		.map(|evidence_id| {
			let note_id = ingested
				.note_ids_by_evidence
				.get(evidence_id)
				.and_then(|ids| ids.first().copied())
				.ok_or_else(|| {
					eyre::eyre!(
						"No live note id mapped for consolidation evidence {} in {}.",
						evidence_id,
						loaded.job.job_id
					)
				})?;
			let text = corpus
				.iter()
				.find(|item| item.evidence_id == *evidence_id)
				.map(|item| item.text.as_str())
				.unwrap_or(evidence_id.as_str());
			let content_hash = format!("blake3:{}", blake3::hash(text.as_bytes()).to_hex());

			Ok(ConsolidationInputRef {
				kind: ConsolidationSourceKind::Note,
				id: note_id,
				snapshot: ConsolidationSourceSnapshot {
					status: Some("active".to_string()),
					updated_at: Some(OffsetDateTime::now_utc()),
					content_hash: Some(content_hash),
					embedding_version: None,
					trace_version: None,
					source_ref: serde_json::json!({
						"schema": "real_world_live_adapter/v1",
						"adapter": adapter_id,
						"job_id": loaded.job.job_id,
						"evidence_id": evidence_id
					}),
					metadata: serde_json::json!({
						"evidence_id": evidence_id,
						"source": "memory_notes"
					}),
				},
			})
		})
		.collect()
}

pub(in crate::consolidation_adapter) fn push_unique_input_ref(
	values: &mut Vec<ConsolidationInputRef>,
	value: ConsolidationInputRef,
) {
	if !values.iter().any(|existing| existing.id == value.id) {
		values.push(value);
	}
}
