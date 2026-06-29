use super::super::*;

pub(super) fn lightrag_source_mappings(
	corpus: &[CorpusText],
	sources: &[LightragSource],
	response: &serde_json::Value,
) -> Vec<SourceMappingEvidence> {
	let mut mappings = Vec::new();

	if let Some(references) = response.get("references").and_then(serde_json::Value::as_array) {
		for reference in references {
			mappings.push(lightrag_reference_mapping(corpus, sources, reference));
		}
	}

	if mappings.is_empty()
		&& let Some(context) = response.get("response").and_then(serde_json::Value::as_str)
	{
		let evidence_ids = map_lightrag_evidence_ids(corpus, sources, context);

		if !evidence_ids.is_empty() {
			mappings.push(SourceMappingEvidence {
				source: "response_context".to_string(),
				evidence_ids,
				mapping_status: "matched_context".to_string(),
				content_count: 1,
			});
		}
	}

	mappings
}

fn lightrag_reference_mapping(
	corpus: &[CorpusText],
	sources: &[LightragSource],
	reference: &serde_json::Value,
) -> SourceMappingEvidence {
	let source = reference
		.get("file_path")
		.and_then(serde_json::Value::as_str)
		.or_else(|| reference.get("reference_id").and_then(serde_json::Value::as_str))
		.unwrap_or("unknown_source")
		.to_string();
	let content = reference
		.get("content")
		.and_then(serde_json::Value::as_array)
		.into_iter()
		.flatten()
		.filter_map(serde_json::Value::as_str)
		.collect::<Vec<_>>();
	let joined_content = content.join("\n");
	let combined = format!("{source}\n{joined_content}");
	let evidence_ids = map_lightrag_evidence_ids(corpus, sources, combined.as_str());
	let mapping_status = if evidence_ids.is_empty() {
		"unmatched"
	} else if !joined_content.is_empty() {
		"matched_reference_content"
	} else {
		"matched_reference_source"
	};

	SourceMappingEvidence {
		source,
		evidence_ids,
		mapping_status: mapping_status.to_string(),
		content_count: content.len(),
	}
}

fn map_lightrag_evidence_ids(
	corpus: &[CorpusText],
	sources: &[LightragSource],
	haystack: &str,
) -> Vec<String> {
	let normalized_haystack = normalize_ascii_alnum_lowercase(haystack);
	let mut evidence_ids = Vec::new();

	for item in corpus {
		let evidence_slug = slug(&item.evidence_id);
		let signature = normalized_text_signature(item.text.as_str());
		let source_match = sources.iter().any(|source| {
			source.evidence_id == item.evidence_id
				&& (haystack.contains(source.file_source.as_str())
					|| haystack.contains(source.artifact_path.to_string_lossy().as_ref()))
		});
		let id_match = haystack.contains(item.evidence_id.as_str())
			|| haystack.contains(evidence_slug.as_str())
			|| normalized_haystack.contains(evidence_slug.as_str());
		let content_match =
			!signature.is_empty() && normalized_haystack.contains(signature.as_str());

		if source_match || id_match || content_match {
			push_unique(&mut evidence_ids, item.evidence_id.clone());
		}
	}

	evidence_ids
}

fn normalized_text_signature(text: &str) -> String {
	normalize_ascii_alnum_lowercase(text).split_whitespace().take(8).collect::<Vec<_>>().join(" ")
}

pub(super) fn lightrag_mapped_evidence_ids(mappings: &[SourceMappingEvidence]) -> Vec<String> {
	let mut evidence_ids = Vec::new();

	for mapping in mappings {
		for evidence_id in &mapping.evidence_ids {
			push_unique(&mut evidence_ids, evidence_id.clone());
		}
	}

	evidence_ids
}
