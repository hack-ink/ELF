use qdrant_client::qdrant::value;
use uuid::Uuid;

use elf_service::{
	DocsExcerptsGetRequest, DocsGetRequest, DocsSearchL0Request, ElfService, TextQuoteSelector,
	docs::DocRetrievalTrajectory,
};

pub(crate) fn payload_string(payload_value: &qdrant_client::qdrant::Value) -> Option<&str> {
	match payload_value.kind.as_ref() {
		Some(value::Kind::StringValue(value)) => Some(value.as_str()),
		_ => None,
	}
}

pub(crate) fn trajectory_stage_stats<'a>(
	trajectory: &'a DocRetrievalTrajectory,
	stage_name: &str,
) -> Option<&'a serde_json::Value> {
	trajectory.stages.iter().find(|stage| stage.stage_name == stage_name).map(|stage| &stage.stats)
}

pub(crate) async fn assert_doc_get(service: &ElfService, doc_id: Uuid) {
	let get_as_owner = service
		.docs_get(DocsGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "owner".to_string(),
			read_profile: "private_plus_project".to_string(),
			doc_id,
		})
		.await
		.expect("Failed to get doc as owner.");

	assert_eq!(get_as_owner.scope, "project_shared");
	assert_eq!(get_as_owner.doc_type, "knowledge");
	assert_eq!(get_as_owner.agent_id, "owner");
	assert_eq!(get_as_owner.title.as_deref(), Some("Docs v1"));

	let get_as_reader = service
		.docs_get(DocsGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "reader".to_string(),
			read_profile: "private_plus_project".to_string(),
			doc_id,
		})
		.await
		.expect("Failed to get doc as reader (expected project grant).");

	assert_eq!(get_as_reader.doc_id, doc_id);
}

pub(crate) async fn assert_doc_excerpt(service: &ElfService, doc_id: Uuid, content_hash: &str) {
	let excerpts = service
		.docs_excerpts_get(DocsExcerptsGetRequest {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			agent_id: "reader".to_string(),
			read_profile: "private_plus_project".to_string(),
			doc_id,
			level: "L1".to_string(),
			chunk_id: None,
			quote: Some(TextQuoteSelector {
				exact: "Keyword: peregrine.".to_string(),
				prefix: Some("evidence. ".to_string()),
				suffix: Some("\nSecond".to_string()),
			}),
			position: None,
			explain: None,
		})
		.await
		.expect("Failed to get excerpt.");

	assert!(excerpts.verification.verified);
	assert!(excerpts.excerpt.contains("Keyword: peregrine."));
	assert_eq!(excerpts.verification.content_hash, content_hash);
}

pub(crate) async fn assert_docs_search_l0(service: &ElfService, doc_id: Uuid) {
	let results = service
		.docs_search_l0(DocsSearchL0Request {
			tenant_id: "t".to_string(),
			project_id: "p".to_string(),
			caller_agent_id: "reader".to_string(),
			scope: None,
			status: None,
			doc_type: None,
			sparse_mode: None,
			domain: None,
			repo: None,
			agent_id: None,
			thread_id: None,
			updated_after: None,
			updated_before: None,
			ts_gte: None,
			ts_lte: None,
			read_profile: "private_plus_project".to_string(),
			query: "peregrine".to_string(),
			top_k: Some(5),
			candidate_k: Some(20),
			explain: None,
		})
		.await
		.expect("Failed to search docs.");

	assert!(!results.items.is_empty());
	assert_eq!(results.items[0].doc_id, doc_id);
	assert_eq!(results.items[0].doc_type, "knowledge");
	assert!(results.items[0].snippet.contains("peregrine"));
}
