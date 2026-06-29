use super::*;

pub(in crate::knowledge) fn rebuild_metadata(
	source_hash: &str,
	provider_metadata: &Value,
	req: &KnowledgePageRebuildRequest,
) -> Value {
	let llm_derived =
		provider_metadata.get("llm_derived").and_then(Value::as_bool).unwrap_or(false);

	serde_json::json!({
		"schema": KNOWLEDGE_PAGE_REBUILD_SCHEMA_V1,
		"source_snapshot_hash": source_hash,
		"deterministic": !llm_derived,
		"provider_metadata": provider_metadata,
		"generated_by": {
			"schema": "elf.knowledge_page.generated_by/v1",
			"runtime": "ElfService::knowledge_page_rebuild",
			"actor_agent_id": req.agent_id,
			"mode": if llm_derived { "provider_metadata_declared_llm" } else { "deterministic_service" },
			"source_input_counts": {
				"doc": req.doc_ids.len(),
				"doc_chunk": req.doc_chunk_ids.len(),
				"note": req.note_ids.len(),
				"event": req.event_ids.len(),
				"relation": req.relation_ids.len(),
				"proposal": req.proposal_ids.len(),
			},
		},
		"memory_candidate_policy": {
			"schema": "elf.knowledge_page.memory_candidate_policy/v1",
			"review_required": true,
			"review_surface": "consolidation_proposals",
			"proposal_contract_schema": "elf.consolidation/v1",
			"allowed_apply_intents": ["create_derived_note", "update_derived_note"],
			"direct_memory_ledger_mutation_allowed": false,
			"source_mutation_allowed": false,
		},
		"allowed_variance": if llm_derived {
			serde_json::json!(["LLM-derived page text may vary; provider metadata records the nondeterministic input path."])
		} else {
			serde_json::json!([])
		},
	})
}

pub(in crate::knowledge) fn rebuild_metadata_with_previous_version_diff(
	mut metadata: Value,
	diff: Value,
	version_identity: Value,
) -> Value {
	let Some(object) = metadata.as_object_mut() else {
		return serde_json::json!({
			PREVIOUS_VERSION_DIFF_KEY: diff,
			"version_identity": version_identity,
		});
	};

	object.insert(PREVIOUS_VERSION_DIFF_KEY.to_string(), diff);
	object.insert("version_identity".to_string(), version_identity);

	metadata
}

pub(in crate::knowledge) fn previous_version_diff_from_metadata(metadata: &Value) -> Option<Value> {
	metadata
		.get(PREVIOUS_VERSION_DIFF_KEY)
		.filter(|diff| diff.as_object().is_some_and(|object| !object.is_empty()))
		.cloned()
}

pub(in crate::knowledge) fn version_identity_value(
	page_kind: KnowledgePageKind,
	page_key: &str,
	source_hash: &str,
	content_hash: &str,
	sections: &[DraftSection],
) -> Value {
	serde_json::json!({
		"schema": "elf.knowledge_page.version_identity/v1",
		"contract_schema": KNOWLEDGE_PAGE_CONTRACT_SCHEMA_V1,
		"page_kind": page_kind.as_str(),
		"page_key": page_key,
		"source_snapshot_hash": source_hash,
		"content_hash": content_hash,
		"section_hashes": sections
			.iter()
			.map(|section| {
				serde_json::json!({
					"section_key": section.section_key.clone(),
					"content_hash": section.content_hash.clone(),
				})
			})
			.collect::<Vec<_>>(),
		"source_mutation_allowed": false,
	})
}

pub(in crate::knowledge) fn content_hash_rebuild_metadata(rebuild_metadata: &Value) -> Value {
	let Some(object) = rebuild_metadata.as_object() else {
		return rebuild_metadata.clone();
	};
	let mut stable = object.clone();

	stable.remove(PREVIOUS_VERSION_DIFF_KEY);
	stable.remove("generated_by");
	stable.remove("memory_candidate_policy");
	stable.remove("version_identity");

	Value::Object(stable)
}

pub(in crate::knowledge) fn section_hash_payload(section: &DraftSection) -> Value {
	serde_json::json!({
		"section_key": section.section_key.clone(),
		"heading": section.heading.clone(),
		"role": section.role.clone(),
		"content": section.content.clone(),
		"citations": section.citations.clone(),
		"unsupported_reason": section.unsupported_reason.clone(),
	})
}

pub(in crate::knowledge) fn page_content_hash(
	title: &str,
	sections: &[DraftSection],
	source_coverage: &Value,
	rebuild_metadata: &Value,
) -> Result<String> {
	let stable_rebuild_metadata = content_hash_rebuild_metadata(rebuild_metadata);

	hash_json(&serde_json::json!({
		"title": title,
		"sections": sections.iter().map(section_hash_payload).collect::<Vec<_>>(),
		"source_coverage": source_coverage,
		"rebuild_metadata": stable_rebuild_metadata,
	}))
}
