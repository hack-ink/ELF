use std::path::Path;

use crate::{LightragSource, SourceMappingEvidence, serde_json};

pub(super) fn lightrag_source_mappings(
	sources: &[LightragSource],
	response: &serde_json::Value,
) -> Vec<SourceMappingEvidence> {
	let mut mappings = Vec::new();

	if let Some(references) = response.get("references").and_then(serde_json::Value::as_array) {
		for reference in references {
			mappings.push(lightrag_reference_mapping(sources, reference));
		}
	}

	mappings
}

pub(super) fn lightrag_mapped_evidence_ids(mappings: &[SourceMappingEvidence]) -> Vec<String> {
	let mut evidence_ids = Vec::new();

	for mapping in mappings {
		for evidence_id in &mapping.evidence_ids {
			crate::push_unique(&mut evidence_ids, evidence_id.clone());
		}
	}

	evidence_ids
}

fn lightrag_reference_mapping(
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
	let evidence_ids = map_lightrag_evidence_ids(sources, source.as_str());
	let mapping_status =
		if evidence_ids.is_empty() { "unmatched" } else { "matched_native_source" };

	SourceMappingEvidence {
		source,
		evidence_ids,
		mapping_status: mapping_status.to_string(),
		content_count: content.len(),
	}
}

fn map_lightrag_evidence_ids(sources: &[LightragSource], native_source: &str) -> Vec<String> {
	let mut evidence_ids = Vec::new();

	let native_name = Path::new(native_source).file_name();
	for source in sources {
		let expected_name = Path::new(source.file_source.as_str()).file_name();
		let source_match = native_source == source.file_source
			|| native_source.ends_with(source.file_source.as_str())
			|| native_source == source.evidence_id
			|| (native_name.is_some() && native_name == expected_name);

		if source_match {
			crate::push_unique(&mut evidence_ids, source.evidence_id.clone());
		}
	}

	evidence_ids
}

#[cfg(test)]
mod tests {
	use super::*;

	fn source() -> LightragSource {
		LightragSource {
			evidence_id: "ev-current".to_string(),
			file_source: "elf-real-world/run/job/ev-current.md".to_string(),
		}
	}

	#[test]
	fn native_reference_path_maps_to_exact_evidence_identity() {
		let response = serde_json::json!({
			"references": [{"file_path": "/app/elf-real-world/run/job/ev-current.md"}]
		});
		let mappings = lightrag_source_mappings(&[source()], &response);

		assert_eq!(mappings[0].evidence_ids, ["ev-current"]);
		assert_eq!(mappings[0].mapping_status, "matched_native_source");
	}

	#[test]
	fn answer_content_does_not_reconstruct_missing_native_provenance() {
		let response = serde_json::json!({
			"response": "The answer quotes ev-current and its full source text.",
			"references": [{"file_path": "unknown.md", "content": ["ev-current"]}]
		});
		let mappings = lightrag_source_mappings(&[source()], &response);

		assert!(mappings[0].evidence_ids.is_empty());
		assert_eq!(mappings[0].mapping_status, "unmatched");
	}
}
