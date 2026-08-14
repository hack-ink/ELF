use crate::{LoadedJob, MaterializationStatus, MaterializedJob, MaterializedJobInput, serde_json};

fn supports_retrieval_suite(suite: &str) -> bool {
	matches!(suite, "retrieval" | "knowledge_structure")
}

pub(super) fn lightrag_not_encoded_job(
	adapter_id: &str,
	loaded: &LoadedJob,
) -> Option<MaterializedJob> {
	if supports_retrieval_suite(loaded.job.suite.as_str()) {
		None
	} else {
		Some(crate::materialized_declared_status_job(
			adapter_id,
			loaded,
			MaterializationStatus::NotEncoded,
			"LightRAG has no benchmark encoding for this suite's required native operations."
				.to_string(),
		))
	}
}

pub(super) fn lightrag_failure_jobs(
	adapter_id: &str,
	jobs: &[LoadedJob],
	stage: &str,
	reason: String,
) -> Vec<MaterializedJob> {
	jobs.iter()
		.map(|job| {
			if let Some(declared) = crate::declared_encoding_job(adapter_id, job) {
				return declared;
			}
			if let Some(not_encoded) = lightrag_not_encoded_job(adapter_id, job) {
				return not_encoded;
			}

			crate::materialized_job(
				job,
				adapter_id,
				MaterializedJobInput {
					content: String::new(),
					evidence_ids: Vec::new(),
					contexts: None,
					pages: Vec::new(),
					latency_ms: 0.0,
					indexing_latency_ms: None,
					returned_count: 0,
					trace_id: None,
					failure: Some(format!("{stage}: {reason}")),
					source_mappings: Vec::new(),
					operator_debug: None,
					operator_debug_evidence: None,
					capture: None,
					capture_failure: None,
					consolidation_response: None,
					consolidation: None,
					knowledge: None,
					temporal_reconciliation: None,
					dreaming_readback: None,
					memory_summaries: Vec::new(),
					proactive_briefs: Vec::new(),
					scheduled_tasks: Vec::new(),
					trace_stages: None,
				},
			)
		})
		.collect()
}

pub(super) fn lightrag_index_failed(status: &serde_json::Value) -> bool {
	status.get("documents").and_then(serde_json::Value::as_array).into_iter().flatten().any(|doc| {
		doc.get("status")
			.and_then(serde_json::Value::as_str)
			.is_some_and(|status| status.to_ascii_lowercase().contains("fail"))
	})
}

pub(super) fn lightrag_index_processed(status: &serde_json::Value, expected_docs: usize) -> bool {
	let Some(documents) = status.get("documents").and_then(serde_json::Value::as_array) else {
		return false;
	};

	documents.len() >= expected_docs
		&& documents.iter().all(|doc| {
			doc.get("status").and_then(serde_json::Value::as_str).is_some_and(|status| {
				let normalized = status.to_ascii_lowercase();

				normalized.contains("processed") || normalized.contains("success")
			})
		})
}

#[cfg(test)]
mod tests {
	use super::supports_retrieval_suite;

	#[test]
	fn retrieval_and_knowledge_structure_use_the_native_query_adapter() {
		assert!(supports_retrieval_suite("retrieval"));
		assert!(supports_retrieval_suite("knowledge_structure"));
		assert!(!supports_retrieval_suite("memory_lifecycle"));
	}
}
